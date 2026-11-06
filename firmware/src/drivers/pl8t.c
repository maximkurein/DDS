/**
 * @file pl8t.c
 * @brief Драйвер 1508ПЛ8Т: команды SETA/WR/WRI/SETAFT/NOP (ТО, табл. 6.1).
 */
#include "pl8t.h"

#include "core/freq.h"
#include "board.h"
#include "pl8t_regs.h"
#include "spi.h"

#define PL8T_CHANNEL      1u
#define PL8T_WRITE_TRIES  3u
#define PL8T_HEALTH_READS 3u
/* При чтении SEL_REG поля LSx_stage показывают стадию ЛЧМ — сравниваем только Pr_1, Pr_2. */
#define SEL_PR_MASK       (PL8T_SEL_PR1(0x3Fu) | PL8T_SEL_PR2(0x3Fu))

/* Состояние драйвера: копия CTR и номер активного профиля. */
static uint16_t s_ctr;
static uint8_t  s_active;

static inline uint32_t frame(uint8_t cmd, uint16_t param)
{
    return ((uint32_t)cmd << 16) | param;
}

static pl8t_status_t xfer(uint8_t cmd, uint16_t param, uint16_t *rx)
{
    uint32_t word;
    /* Проверка до и после каждого кадра, включая блокирующую инициализацию.
     * Это программный контроль PC4, не защита от аналоговых провалов шин. */
    if (!board_pgood_raw()) {
        board_dds_reset(true);
        board_dds_bus_enable(false);
        return PL8T_ERR_POWER;
    }
    const bool ok = spi_xfer24(frame(cmd, param), &word);
    if (!board_pgood_raw()) {
        board_dds_reset(true);
        board_dds_bus_enable(false);
        return PL8T_ERR_POWER;
    }
    if (!ok) {
        return PL8T_ERR_SPI;
    }
    if (rx != 0) {
        *rx = (uint16_t)(word & 0xFFFFu);  /* данные — младшие 16 бит */
    }
    return PL8T_OK;
}

#define TRY(expr) do { pl8t_status_t st_ = (expr); if (st_ != PL8T_OK) { return st_; } } while (0)

pl8t_status_t pl8t_write(uint16_t addr, uint16_t value)
{
    TRY(xfer(PL8T_CMD_SETA, addr, 0));
    return xfer(PL8T_CMD_WR, value, 0);
}

pl8t_status_t pl8t_read(uint16_t addr, uint16_t *value)
{
    /* SETAFT кладёт [addr] в сдвиговый регистр, NOP выдвигает его на SDO. */
    TRY(xfer(PL8T_CMD_SETAFT, addr, 0));
    return xfer(PL8T_CMD_NOP, 0, value);
}

pl8t_status_t pl8t_write_block(uint16_t addr, const uint16_t *values, uint8_t n)
{
    TRY(xfer(PL8T_CMD_SETA, addr, 0));
    for (uint8_t i = 0; i < n; i++) {
        TRY(xfer(PL8T_CMD_WRI, values[i], 0));
    }
    return PL8T_OK;
}

pl8t_status_t pl8t_read_block(uint16_t addr, uint16_t *values, uint8_t n)
{
    if (n == 0u) {
        return PL8T_OK;
    }
    /* Конвейер: в каждом кадре выдвигается результат предыдущего SETAFT. */
    TRY(xfer(PL8T_CMD_SETAFT, addr, 0));
    for (uint8_t i = 1; i < n; i++) {
        TRY(xfer(PL8T_CMD_SETAFT, (uint16_t)(addr + i), &values[i - 1u]));
    }
    return xfer(PL8T_CMD_NOP, 0, &values[n - 1u]);
}

/** Запись блока и проверка чтением. */
static pl8t_status_t write_block_verified(uint16_t addr, const uint16_t *values, uint8_t n)
{
    uint16_t rb[PL8T_PRF_WORDS];
    if (n > PL8T_PRF_WORDS) {
        return PL8T_ERR_VERIFY;
    }
    for (uint8_t attempt = 0; attempt < PL8T_WRITE_TRIES; attempt++) {
        TRY(pl8t_write_block(addr, values, n));
        TRY(pl8t_read_block(addr, rb, n));
        bool same = true;
        for (uint8_t i = 0; i < n; i++) {
            same = same && (rb[i] == values[i]);
        }
        if (same) {
            return PL8T_OK;
        }
    }
    return PL8T_ERR_VERIFY;
}

/** Запись одного регистра и проверка чтением по маске значимых полей. */
static pl8t_status_t write_verified_mask(uint16_t addr, uint16_t value, uint16_t mask)
{
    for (uint8_t attempt = 0; attempt < PL8T_WRITE_TRIES; attempt++) {
        uint16_t rb = 0;
        TRY(pl8t_write(addr, value));
        TRY(pl8t_read(addr, &rb));
        if ((rb & mask) == (value & mask)) {
            return PL8T_OK;
        }
    }
    return PL8T_ERR_VERIFY;
}

static pl8t_status_t write_verified(uint16_t addr, uint16_t value)
{
    return write_verified_mask(addr, value, 0xFFFFu);
}

pl8t_status_t pl8t_init(const pl8t_config_t *cfg)
{
    uint16_t devid = 0;

    /* 1. Связь: DEVID = 0x0201 (табл. 5.1); ошибка не определяет её причину. */
    TRY(pl8t_read(PL8T_DEVID, &devid));
    if (devid != PL8T_DEVID_VALUE) {
        return PL8T_ERR_DEVID;
    }

    /* 2. CTR: делитель такта, ЦАП 1 пока выключен, ЦАП 2 и компаратор выключены. */
    s_ctr = PL8T_CTR_CMX(cfg->clk_cmx);
    TRY(write_verified_mask(PL8T_CTR, s_ctr, PL8T_CTR_CONTROL_MASK));

    /* 3. Синхронизация: CSYNC и SEL выключены, профиль выбирается через SEL_REG. */
    TRY(write_verified(PL8T_SYNC, 0x0000u));
    TRY(write_verified(PL8T_LINK, 0x0000u));
    TRY(write_verified(PL8T_ROUTE, (uint16_t)(PL8T_ROUTE_SUM_OWN |
                                              (cfg->phase_dither ? PL8T_ROUTE_PDITH1 : 0u))));
    TRY(write_verified(PL8T_CH_LS_CTR(PL8T_CHANNEL), 0x0000u));  /* ЛЧМ выключен */
    TRY(write_verified(PL8T_CH_TSW(PL8T_CHANNEL), (uint16_t)(cfg->tsw & 0x7u)));

    /* 4. Профили 0 и 1: частота 0, фаза 0, амплитуда, смещение 0. */
    const uint16_t profile[PL8T_PRF_WORDS] = {
        [PL8T_PRF_DPH_L]    = 0,
        [PL8T_PRF_DPH_M]    = 0,
        [PL8T_PRF_DPH_H]    = 0,
        [PL8T_PRF_RESERVED] = 0,  /* резерв: запись игнорируется, читается 0 */
        [PL8T_PRF_P]        = 0,
        [PL8T_PRF_MUL]      = PL8T_MUL(cfg->amplitude_mul),
        [PL8T_PRF_OFFSET]   = PL8T_OFFSET(0),
    };
    TRY(write_block_verified(PL8T_PROFILE(PL8T_CHANNEL, 0u), profile, PL8T_PRF_WORDS));
    TRY(write_block_verified(PL8T_PROFILE(PL8T_CHANNEL, 1u), profile, PL8T_PRF_WORDS));

    /* 5. Активный профиль 0 в обоих каналах. */
    TRY(write_verified_mask(PL8T_SEL_REG, PL8T_SEL_PR1(0u) | PL8T_SEL_PR2(0u), SEL_PR_MASK));
    s_active = 0;
    return PL8T_OK;
}

pl8t_status_t pl8t_set_ftw(uint64_t ftw)
{
    uint16_t dph[3];
    const uint8_t next = (uint8_t)(s_active ^ 1u);

    freq_ftw_split(ftw & PL8T_FTW_MASK, dph);

    /* Неактивный профиль: dPh_L, dPh_M, dPh_H подряд, затем проверка. */
    TRY(write_block_verified(PL8T_PROFILE(PL8T_CHANNEL, next), dph, 3u));

    /* Переключение профиля — одна запись, частота меняется без промежуточных значений. */
    TRY(write_verified_mask(PL8T_SEL_REG, PL8T_SEL_PR1(next) | PL8T_SEL_PR2(0u), SEL_PR_MASK));
    s_active = next;

    if ((s_ctr & PL8T_CTR_DAC1_ON) == 0u) {
        s_ctr |= PL8T_CTR_DAC1_ON;
        TRY(write_verified_mask(PL8T_CTR, s_ctr, PL8T_CTR_CONTROL_MASK));
    }
    return PL8T_OK;
}

pl8t_status_t pl8t_output_off(void)
{
    s_ctr &= (uint16_t)~PL8T_CTR_DAC1_ON;
    return write_verified_mask(PL8T_CTR, s_ctr, PL8T_CTR_CONTROL_MASK);
}

pl8t_status_t pl8t_check_alive(void)
{
    for (uint8_t attempt = 0; attempt < PL8T_HEALTH_READS; attempt++) {
        uint16_t ctr = 0;
        TRY(pl8t_read(PL8T_CTR, &ctr));
        if ((ctr & PL8T_CTR_CONTROL_MASK) == (s_ctr & PL8T_CTR_CONTROL_MASK)) {
            return PL8T_OK;
        }
    }
    return PL8T_ERR_VERIFY;
}

uint8_t pl8t_active_profile(void)
{
    return s_active;
}
