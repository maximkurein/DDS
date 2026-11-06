/**
 * @file app.c
 * @brief Конечный автомат изделия DDS 48–52 МГц.
 */
#include "app.h"

#include <avr/pgmspace.h>
#include <stdbool.h>
#include <util/delay.h>

#include "core/debounce.h"
#include "core/dip_code.h"
#include "core/freq.h"
#include "dds_config.h"
#include "drivers/board.h"
#include "drivers/pl8t.h"
#include "drivers/systick.h"
#include "drivers/uart_log.h"

static const freq_scale_t k_scale = {
    .base_hz = DDS_F_BASE_HZ,
    .step_hz = DDS_F_STEP_HZ,
    .n_max   = DDS_N_MAX,
};

static const pl8t_config_t k_dds_cfg = {
    .amplitude_mul = DDS_AMPLITUDE_MUL,
    .clk_cmx       = DDS_CLK_CMX,
    .tsw           = DDS_TSW,
    .phase_dither  = DDS_PHASE_DITHER != 0,
};

static struct {
    app_state_t state;
    uint32_t    t_state;      /**< момент входа в текущее состояние */
    uint32_t    t_health;     /**< последняя проверка DDS */
    uint32_t    t_pgood;      /**< момент, когда +3,3 В стало устойчиво */
    debounce_t  apply;        /**< кнопка S4, true = нажата */
    debounce_t  pgood;        /**< DDS_PGOOD, true = +3,3 В есть */
    bool        apply_event;  /**< нажатие ещё не обработано */
    bool        have_freq;    /**< частота уже задавалась */
    uint16_t    n;            /**< последний применённый код */
    uint64_t    ftw;          /**< последнее применённое приращение фазы */
} s;

static void enter(app_state_t st)
{
    s.state = st;
    s.t_state = systick_ms();
}

/** Имя кода ошибки (строка во flash, для LOG с форматом %S). */
static inline PGM_P status_str(pl8t_status_t st)
{
    switch (st) {
    case PL8T_OK:         return PSTR("ok");
    case PL8T_ERR_SPI:    return PSTR("SPI");
    case PL8T_ERR_DEVID:  return PSTR("DEVID");
    case PL8T_ERR_VERIFY: return PSTR("VERIFY");
    case PL8T_ERR_POWER:  return PSTR("POWER");
    default:              return PSTR("?");
    }
}

/** DDS в сброс, буфер D3 выключен. */
static void dds_shutdown(void)
{
    board_dds_reset(true);
    board_dds_bus_enable(false);
}

/** Повторный запуск только после нового подтверждения устойчивого PGOOD. */
static void wait_power(void)
{
    dds_shutdown();
    debounce_init(&s.pgood, false, DDS_PGOOD_STABLE_MS);
    s.t_pgood = systick_ms();
    s.apply_event = false;
    enter(APP_ST_WAIT_POWER);
}

/** Ошибка связи с DDS: в сброс и повтор через DDS_RETRY_PERIOD_MS. where — PSTR(). */
static void fault(PGM_P where, pl8t_status_t st)
{
    (void)where;
    (void)st;
    dds_shutdown();
    if (st == PL8T_ERR_POWER) {
        wait_power();
        LOG("[dds] +3.3V lost\r\n");
        return;
    }
    enter(APP_ST_FAULT);
    LOG("[dds] %S: error %S, retry in %u ms\r\n", where, status_str(st), DDS_RETRY_PERIOD_MS);
}

/**
 * Чтение DIP: два чтения через DIP_SETTLE_MS должны совпасть
 * (design_notes, разд. 5: «DIP читать дважды»).
 */
static bool dip_read_stable(uint16_t *n)
{
    uint16_t prev = dip_decode(board_dip_lo_raw(), board_dip_hi_raw());
    for (uint8_t i = 0; i < DIP_READ_ATTEMPTS; i++) {
        /* Не оставлять DDS без контроля питания на всё ожидание DIP. */
        for (uint8_t ms = 0; ms < DIP_SETTLE_MS; ms++) {
            if (!board_pgood_raw()) {
                fault(PSTR("dip"), PL8T_ERR_POWER);
                return false;
            }
            _delay_ms(1);
        }
        if (!board_pgood_raw()) {
            fault(PSTR("dip"), PL8T_ERR_POWER);
            return false;
        }
        const uint16_t cur = dip_decode(board_dip_lo_raw(), board_dip_hi_raw());
        if (cur == prev) {
            *n = cur;
            return true;
        }
        prev = cur;
    }
    return false;
}

static inline void log_freq(uint16_t n, uint32_t f_hz, uint64_t ftw)
{
    (void)n;
    (void)f_hz;
    (void)ftw;
    LOG("[app] N=%u f=%lu Hz FTW=0x%04X%08lX profile=%u\r\n", n, (unsigned long)f_hz,
        (unsigned)(ftw >> 32), (unsigned long)(ftw & 0xFFFFFFFFu), pl8t_active_profile());
}

/** Прочитать DIP и задать частоту. false — ошибка связи с DDS (нужна повторная настройка). */
static bool apply_from_dip(void)
{
    uint16_t n;
    uint32_t f_hz;

    if (!dip_read_stable(&n)) {
        if (s.state == APP_ST_WAIT_POWER) {
            /* Уже отключились: не терять обнаруженный провал, даже если
             * PGOOD успел вернуться во время сообщения UART. */
            return false;
        }
        LOG("[app] DIP unstable, ignored\r\n");
        return true;
    }
    if (!freq_code_to_hz(&k_scale, n, &f_hz)) {
        LOG("[app] N=%u > %u rejected\r\n", n, DDS_N_MAX);
        if (!s.have_freq) {
            /* Задавать нечего: выход остаётся выключенным до корректного N. */
            const pl8t_status_t st = pl8t_output_off();
            if (st != PL8T_OK) {
                fault(PSTR("output_off"), st);
                return false;
            }
        }
        return true;
    }

    const uint64_t ftw = freq_hz_to_ftw(f_hz, DDS_FCLK_HZ);
    const pl8t_status_t st = pl8t_set_ftw(ftw);
    if (st != PL8T_OK) {
        fault(PSTR("set_ftw"), st);
        return false;
    }
    s.have_freq = true;
    s.n = n;
    s.ftw = ftw;
    log_freq(n, f_hz, ftw);
    return true;
}

/** Один реальный замер; история уровней за пропущенные тики неизвестна. */
static void sample_inputs(uint8_t ticks)
{
    const bool pressed = board_apply_pressed_raw();
    const bool pgood = board_pgood_raw();
    if (ticks > 1u) {
        s.apply.count = 0;
        s.pgood.count = 0;
        if (s.state == APP_ST_WAIT_POWER) {
            /* Во время паузы PGOOD тоже мог исчезать: подтвердить заново. */
            s.pgood.state = false;
        }
    }
    if (debounce_update(&s.apply, pressed) && s.apply.state) {
        s.apply_event = true;
    }
    if (debounce_update(&s.pgood, pgood) && s.pgood.state) {
        s.t_pgood = systick_ms();
    }
}

void app_init(void)
{
    debounce_init(&s.apply, false, APPLY_DEBOUNCE_MS);
    debounce_init(&s.pgood, false, DDS_PGOOD_STABLE_MS);
    s.t_pgood = systick_ms();
    s.apply_event = false;
    s.have_freq = false;
    dds_shutdown();
    enter(APP_ST_WAIT_POWER);
}

app_state_t app_state(void)
{
    return s.state;
}

void app_poll(void)
{
    /* Спад питания не фильтруем: проверяем и между миллисекундными тиками,
     * во всех стадиях запуска. Фильтрация нужна только перед включением. */
    if (!board_pgood_raw()) {
        if (s.state != APP_ST_WAIT_POWER) {
            wait_power();
            LOG("[dds] +3.3V lost\r\n");
        } else {
            debounce_init(&s.pgood, false, DDS_PGOOD_STABLE_MS);
            s.t_pgood = systick_ms();
        }
    }
    const uint8_t ticks = systick_take_ticks();
    if (ticks == 0u) {
        return;
    }
    sample_inputs(ticks);

    switch (s.state) {
    case APP_ST_WAIT_POWER:
        s.apply_event = false;  /* нажатия до готовности DDS не копим */
        /* Пауза отсчитывается и от старта, и от появления +3,3 В:
         * +1,8 В не контролируется и поднимается вместе с ним. */
        if (s.pgood.state && systick_elapsed(s.t_pgood, DDS_POWER_SETTLE_MS) &&
            systick_elapsed(s.t_state, DDS_POWER_SETTLE_MS)) {
            board_dds_reset(true);         /* RSTn = 0 */
            board_dds_bus_enable(true);    /* D3 включён: CS = 1, SCK = 0, MOSI = 0 */
            enter(APP_ST_RESET);
        }
        break;

    case APP_ST_RESET:
        if (systick_elapsed(s.t_state, DDS_RESET_PULSE_MS)) {
            board_dds_reset(false);
            enter(APP_ST_RECOVER);
        }
        break;

    case APP_ST_RECOVER:
        if (systick_elapsed(s.t_state, DDS_RESET_RECOVERY_MS)) {
            enter(APP_ST_INIT);
        }
        break;

    case APP_ST_INIT: {
        const pl8t_status_t st = pl8t_init(&k_dds_cfg);
        if (st != PL8T_OK) {
            fault(PSTR("init"), st);
            break;
        }
        LOG("[dds] 1508PL8T ready (DEVID ok)\r\n");
        if (s.have_freq) {
            /* Повторная настройка после сбоя: восстанавливаем последнюю частоту. */
            const pl8t_status_t st2 = pl8t_set_ftw(s.ftw);
            if (st2 != PL8T_OK) {
                fault(PSTR("restore"), st2);
                break;
            }
            LOG("[app] restored N=%u\r\n", s.n);
        } else if (!apply_from_dip()) {
            break;  /* уже в FAULT */
        }
        s.t_health = systick_ms();
        s.apply_event = false;
        enter(APP_ST_RUN);
        break;
    }

    case APP_ST_RUN:
        if (s.apply_event) {
            s.apply_event = false;
            if (!apply_from_dip()) {
                break;
            }
        }
        if (DDS_HEALTH_PERIOD_MS != 0u && systick_elapsed(s.t_health, DDS_HEALTH_PERIOD_MS)) {
            s.t_health = systick_ms();
            const pl8t_status_t st = pl8t_check_alive();
            if (st != PL8T_OK) {
                fault(PSTR("health"), st);
            }
        }
        break;

    case APP_ST_FAULT:
        if (systick_elapsed(s.t_state, DDS_RETRY_PERIOD_MS)) {
            enter(APP_ST_WAIT_POWER);
        }
        break;

    default:
        app_init();
        break;
    }
}
