/**
 * @file sim_dds.c
 * @brief Проверка прошивки в симуляторе simavr с моделью 1508ПЛ8Т на шине SPI.
 *
 * Запускает настоящий ELF прошивки (ATmega128, 8 МГц), подаёт уровни на DIP,
 * кнопку и DDS_PGOOD, принимает 24-битные кадры SPI и исполняет их моделью
 * синтезатора по ТО (табл. 5.1, 6.1). Проверяет сценарий:
 *   1. без +3,3 В шина DDS выключена и кадров нет;
 *   2. после +3,3 В: сброс, настройка, частота с DIP (N = 1234);
 *   3. кнопка: N = 4000 — новая частота через второй профиль;
 *   4. кнопка: N = 4001 — отказ, частота прежняя;
 *   5. сбой DDS (сброс регистров) — повторная настройка и та же частота;
 *   6. пропадание +3,3 В — шина выключена; возврат — восстановление;
 *   7. дребезг кнопки — ровно одна смена частоты;
 *   0. калибровка RC из EEPROM[0..1] применяется при старте.
 *
 * Сборка и запуск (Linux/WSL, пакеты simavr, libsimavr-dev, libelf-dev):
 *   cmake --preset sim && cmake --build --preset sim && ctest --preset sim
 */
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <simavr/avr_eeprom.h>
#include <simavr/avr_ioport.h>
#include <simavr/avr_spi.h>
#include <simavr/avr_uart.h>
#include <simavr/sim_avr.h>
#include <simavr/sim_elf.h>

#include "drivers/pl8t_regs.h"

#define F_CPU_HZ   8000000UL
#define FCLK_HZ    480000000UL

/* Адреса регистров ATmega128 в пространстве данных. */
#define A_PINB  0x36
#define A_DDRB  0x37
#define A_PORTB 0x38
#define A_SPCR  0x2D

/* ------------------------------------------------------------------------- */
/* Модель 1508ПЛ8Т                                                          */
/* ------------------------------------------------------------------------- */
static struct {
    uint16_t regs[0x10000];
    uint16_t addr;
    uint32_t shift;       /* что выдвинется на SDO в следующем кадре */
    uint32_t frame_in;
    uint32_t frame_out;
    int      nbytes;
    bool     cs_low;
    bool     in_reset;
    /* статистика */
    unsigned frames;
    unsigned bad_frames;
    unsigned bad_cmds;
    unsigned sel_switches;
    unsigned resets;
    uint64_t first_frame_cycle;
} dds;

static avr_t   *g_avr;
static avr_irq_t *g_spi_in;
static unsigned g_bad_ctr_reads, g_ctr_reads, g_drop_spi_byte;
static uint16_t g_ctr_xor;
static bool g_alternating_ctr;

static void dds_defaults(void)
{
    memset(dds.regs, 0, sizeof dds.regs);
    dds.regs[PL8T_CTR]  = 0x0002;
    dds.regs[PL8T_SYNC] = 0x4800;
    dds.addr = 0;
    dds.shift = PL8T_DEVID_VALUE;
    dds.resets++;
}

static bool is_write_only(uint16_t a)
{
    if (a == PL8T_SWRST || a == PL8T_CLR) return true;
    if ((a & 0x0FFF) >= 0x300 && (a & 0x0FFF) <= 0x306 && (a >> 12) >= 1 && (a >> 12) <= 2) return true;
    return false;
}

static uint16_t dds_read(uint16_t a)
{
    if (a == PL8T_DEVID) return PL8T_DEVID_VALUE;
    if (is_write_only(a)) return 0;
    if (a == PL8T_CTR) {
        g_ctr_reads++;
        if (g_bad_ctr_reads) {
            g_bad_ctr_reads--;
            return (uint16_t)(dds.regs[a] ^
                (g_alternating_ctr && (g_ctr_reads & 1u) ? 0x2000u : g_ctr_xor));
        }
    }
    return dds.regs[a];
}

static void dds_write(uint16_t a, uint16_t v)
{
    if (a == PL8T_SWRST) { if (v == PL8T_SWRST_KEY) dds_defaults(); return; }
    if (a == PL8T_DEVID) return;
    if (a == PL8T_CTR)  { dds.regs[a] = v & 0x701F; return; }
    if (a == PL8T_SYNC) { dds.regs[a] = v & 0x6FF8; return; }
    if (a == PL8T_SEL_REG) {
        if ((dds.regs[a] & 0x3F) != (v & 0x3F)) dds.sel_switches++;
        dds.regs[a] = v & 0x3F3F;
        return;
    }
    /* Профили канала 1/2: резерв +3, младшие разряды Mul и Offset — резерв. */
    if (((a & 0xF000) == 0x1000 || (a & 0xF000) == 0x2000) && (a & 0x0FFF) >= 0x400 && (a & 0x0FFF) < 0x800) {
        switch (a & 0xF) {
        case 3: return;
        case 5: dds.regs[a] = v & 0xFFF8; return;
        case 6: dds.regs[a] = v & 0xFFF0; return;
        default: break;
        }
    }
    dds.regs[a] = v;
}

static void dds_execute(uint32_t w)
{
    const uint8_t cmd = (uint8_t)(w >> 16);
    const uint16_t p = (uint16_t)w;
    switch (cmd) {
    case PL8T_CMD_NOP:    dds.shift = PL8T_DEVID_VALUE; break;
    case PL8T_CMD_SETA:   dds.addr = p; dds.shift = dds.addr; break;
    case PL8T_CMD_WR:     dds_write(dds.addr, p); dds.shift = dds.addr; break;
    case PL8T_CMD_WRI:    dds_write(dds.addr, p); dds.addr++; dds.shift = dds.addr; break;
    case PL8T_CMD_SETAFT: dds.addr = p; dds.shift = dds_read(dds.addr); break;
    default:              dds.bad_cmds++; break;
    }
}

/* Состояние линий со стороны МК: буфер D3 и RSTn. */
static bool bus_enabled(void)
{
    const uint8_t ddr = g_avr->data[A_DDRB], port = g_avr->data[A_PORTB];
    return (ddr & (1u << 5)) && !(port & (1u << 5));
}

static void update_reset(void)
{
    const bool rst = !(bus_enabled() && (g_avr->data[A_PORTB] & (1u << 4)));
    if (rst && !dds.in_reset) {
        dds.in_reset = true;
    } else if (!rst && dds.in_reset) {
        dds.in_reset = false;
        dds_defaults();   /* выход из сброса: регистры по умолчанию */
    }
}

static void pin_cb(struct avr_irq_t *irq, uint32_t value, void *param)
{
    (void)irq; (void)param; (void)value;
    update_reset();
}

static void cs_cb(struct avr_irq_t *irq, uint32_t value, void *param)
{
    (void)irq; (void)param;
    update_reset();
    if (!bus_enabled()) return;              /* D3 выключен: DDS не видит CS */
    if (!value && !dds.cs_low) {             /* спад SCSn */
        dds.cs_low = true;
        dds.nbytes = 0;
        dds.frame_in = 0;
        dds.frame_out = dds.shift;
    } else if (value && dds.cs_low) {        /* фронт SCSn: исполнение */
        dds.cs_low = false;
        if (dds.in_reset) return;
        dds.frames++;
        if (dds.first_frame_cycle == 0) dds.first_frame_cycle = g_avr->cycle;
        if (dds.nbytes != 3) { dds.bad_frames++; return; }
        dds_execute(dds.frame_in);
    }
}

static void spi_cb(struct avr_irq_t *irq, uint32_t value, void *param)
{
    (void)irq; (void)param;
    uint8_t resp = 0;   /* R82 тянет SDO_BUF к 0, если DDS не отвечает */
    if (bus_enabled() && dds.cs_low && !dds.in_reset) {
        if (dds.nbytes < 3) {
            resp = (uint8_t)(dds.frame_out >> (16 - 8 * dds.nbytes));
            dds.frame_in = (dds.frame_in << 8) | (value & 0xFF);
        }
        dds.nbytes++;
    }
    avr_raise_irq(g_spi_in, resp);
    if (g_drop_spi_byte && --g_drop_spi_byte == 0) {
        avr_raise_irq(avr_io_getirq(g_avr, AVR_IOCTL_IOPORT_GETIRQ('C'), 4), 0);
    }
}

/* ------------------------------------------------------------------------- */
/* UART-лог прошивки                                                         */
/* ------------------------------------------------------------------------- */
static char g_log[16384];
static size_t g_log_len;

static void uart_cb(struct avr_irq_t *irq, uint32_t value, void *param)
{
    (void)irq; (void)param;
    if (g_log_len + 1 < sizeof g_log) g_log[g_log_len++] = (char)value;
    if (value != '\r') fputc((int)value, stdout);
}

/* ------------------------------------------------------------------------- */
/* Входы платы                                                               */
/* ------------------------------------------------------------------------- */
static void set_pin(char port, int bit, int level)
{
    avr_raise_irq(avr_io_getirq(g_avr, AVR_IOCTL_IOPORT_GETIRQ(port), bit), level ? 1 : 0);
}

static void set_dip(uint16_t n)
{
    for (int i = 0; i < 8; i++)  set_pin('A', i, !((n >> i) & 1));        /* ON = 0 */
    for (int i = 0; i < 4; i++)  set_pin('C', i, !((n >> (8 + i)) & 1));
}

static void set_pgood(int on) { set_pin('C', 4, on); }
static void set_button(int pressed) { set_pin('C', 5, !pressed); }

static uint64_t ms_to_cycles(double ms) { return (uint64_t)(ms * (F_CPU_HZ / 1000.0)); }

static void run_until_ms(double ms)
{
    const uint64_t target = ms_to_cycles(ms);
    while (g_avr->cycle < target) {
        const int st = avr_run(g_avr);
        if (st == cpu_Done || st == cpu_Crashed) {
            fprintf(stderr, "\nCPU stopped (state %d) at %.1f ms\n", st, g_avr->cycle / (F_CPU_HZ / 1000.0));
            exit(2);
        }
    }
}

static void run_for_ms(double ms)
{
    run_until_ms(g_avr->cycle / (F_CPU_HZ / 1000.0) + ms);
}

/* ------------------------------------------------------------------------- */
/* Проверки                                                                  */
/* ------------------------------------------------------------------------- */
static int g_fail;

#define EXPECT(cond, ...) do {                                    \
        if (cond) { printf("  ok   "); printf(__VA_ARGS__); printf("\n"); } \
        else { g_fail++; printf("  FAIL "); printf(__VA_ARGS__); printf("\n"); } \
    } while (0)

static uint64_t ref_ftw(uint16_t n)
{
    const unsigned __int128 num = (unsigned __int128)(48000000ULL + 1000ULL * n) << 48;
    uint64_t q = (uint64_t)(num / FCLK_HZ);
    const uint64_t r = (uint64_t)(num % FCLK_HZ);
    if (2 * r >= FCLK_HZ) q++;
    return q;
}

static uint8_t active_profile(void) { return (uint8_t)(dds.regs[PL8T_SEL_REG] & 0x3F); }

static uint64_t profile_ftw(uint8_t y)
{
    const uint16_t b = PL8T_PROFILE(1, y);
    return (uint64_t)dds.regs[b] | ((uint64_t)dds.regs[b + 1] << 16) | ((uint64_t)dds.regs[b + 2] << 32);
}

static void expect_output(uint16_t n, const char *what)
{
    const uint8_t y = active_profile();
    const uint16_t mul = dds.regs[PL8T_PROFILE(1, y) + PL8T_PRF_MUL];
    EXPECT(dds.regs[PL8T_CTR] == 0x1000, "%s: CTR = 0x%04X (ЦАП1 вкл, cmx 0)", what, dds.regs[PL8T_CTR]);
    EXPECT(dds.regs[PL8T_SYNC] == 0x0000, "%s: SYNC = 0x%04X (CSYNC/SEL выкл)", what, dds.regs[PL8T_SYNC]);
    EXPECT(y <= 1, "%s: активный профиль %u", what, y);
    EXPECT(profile_ftw(y) == ref_ftw(n), "%s: dPh = 0x%012" PRIX64 ", ожидается 0x%012" PRIX64 " (N=%u)",
           what, profile_ftw(y), ref_ftw(n), n);
    EXPECT(mul == (4095u << 3), "%s: Mul = 0x%04X", what, mul);
}

static void press(double t_ms, double hold_ms)
{
    run_until_ms(t_ms);
    set_button(1);
    run_until_ms(t_ms + hold_ms);
    set_button(0);
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: %s dds_fw.elf\n", argv[0]);
        return 2;
    }

    elf_firmware_t fw;
    memset(&fw, 0, sizeof fw);
    if (elf_read_firmware(argv[1], &fw) != 0) {
        fprintf(stderr, "cannot read %s\n", argv[1]);
        return 2;
    }
    g_avr = avr_make_mcu_by_name("atmega128");
    if (!g_avr) { fprintf(stderr, "simavr: no atmega128 core\n"); return 2; }
    avr_init(g_avr);
    avr_load_firmware(g_avr, &fw);
    g_avr->frequency = F_CPU_HZ;
    g_avr->log = 1;

    g_spi_in = avr_io_getirq(g_avr, AVR_IOCTL_SPI_GETIRQ(0), SPI_IRQ_INPUT);
    avr_irq_register_notify(avr_io_getirq(g_avr, AVR_IOCTL_SPI_GETIRQ(0), SPI_IRQ_OUTPUT), spi_cb, NULL);
    avr_irq_register_notify(avr_io_getirq(g_avr, AVR_IOCTL_IOPORT_GETIRQ('B'), 0), cs_cb, NULL);
    avr_irq_register_notify(avr_io_getirq(g_avr, AVR_IOCTL_IOPORT_GETIRQ('B'), 4), pin_cb, NULL);
    avr_irq_register_notify(avr_io_getirq(g_avr, AVR_IOCTL_IOPORT_GETIRQ('B'), IOPORT_IRQ_DIRECTION_ALL), pin_cb, NULL);
    avr_irq_register_notify(avr_io_getirq(g_avr, AVR_IOCTL_UART_GETIRQ('0'), UART_IRQ_OUTPUT), uart_cb, NULL);

    /* Отключить ожидание чтения из UART в simavr. */
    uint32_t f = 0;
    avr_ioctl(g_avr, AVR_IOCTL_UART_GET_FLAGS('0'), &f);
    f &= ~AVR_UART_FLAG_STDIO;
    avr_ioctl(g_avr, AVR_IOCTL_UART_SET_FLAGS('0'), &f);

    /* Калибровка, как её записывает цель osccal: байт и дополнение. */
    uint8_t ee[2] = { 0x9E, 0x61 };
    const bool missing_cal = argc > 2;
    if (missing_cal) {
        if (strcmp(argv[2], "empty") == 0) ee[0] = ee[1] = 0xFF;
        else if (strcmp(argv[2], "invalid") == 0) ee[1] = 0x60;
        else { fprintf(stderr, "unknown calibration case\n"); return 2; }
    }
    avr_eeprom_desc_t eed = { .ee = ee, .offset = 0, .size = sizeof ee };
    avr_ioctl(g_avr, AVR_IOCTL_EEPROM_SET, &eed);

    dds.in_reset = true;
    dds_defaults();
    dds.resets = 0;

    /* 1. Нет +3,3 В. */
    set_dip(1234);
    set_button(0);
    set_pgood(0);
    run_until_ms(50);
    printf("\n[0] калибровка RC\n");
    if (missing_cal) {
        EXPECT(strstr(g_log, "8 MHz calibration missing") != NULL, "нет калибровки 8 МГц: корректное сообщение");
        EXPECT(strstr(g_log, "factory 1 MHz") == NULL, "нет ложного утверждения о частоте ядра");
    } else {
        EXPECT(strstr(g_log, "OSCCAL=0x9E (EEPROM)") != NULL, "OSCCAL = 0x9E из EEPROM");
    }

    printf("\n[1] без +3,3 В\n");
    EXPECT(dds.frames == 0, "кадров SPI: %u", dds.frames);
    EXPECT(!bus_enabled(), "буфер D3 выключен (PB5 — вход)");
    EXPECT((g_avr->data[A_PORTB] & 0x10) == 0, "DDS в сбросе (PB4 = 0)");
    const uint8_t spcr = g_avr->data[A_SPCR];
    EXPECT((spcr & 0x7C) == 0x50, "SPCR = 0x%02X: SPE, MSTR, MSB first, режим 0", spcr);

    /* 2. Появилось +3,3 В. */
    set_pgood(1);
    run_until_ms(400);
    printf("\n[2] старт, N = 1234\n");
    const double first_ms = dds.first_frame_cycle / (F_CPU_HZ / 1000.0);
    EXPECT(first_ms >= 50 + 10 + 20, "первый кадр через %.1f мс после старта (PGOOD в 50 мс)", first_ms);
    EXPECT(dds.bad_frames == 0 && dds.bad_cmds == 0, "все кадры по 24 бита, команды известные");
    expect_output(1234, "N=1234");
    const unsigned sw0 = dds.sel_switches;

    /* 3. N = 4000 и кнопка. */
    set_dip(4000);
    press(450, 60);
    run_until_ms(700);
    printf("\n[3] кнопка, N = 4000\n");
    expect_output(4000, "N=4000");
    EXPECT(dds.sel_switches == sw0 + 1, "переключение профиля: %u", dds.sel_switches - sw0);

    /* 4. N = 4001 — отказ. */
    set_dip(4001);
    press(750, 60);
    run_until_ms(1000);
    printf("\n[4] кнопка, N = 4001 (вне диапазона)\n");
    expect_output(4000, "N=4001 отвергнут");
    EXPECT(strstr(g_log, "4001 > 4000 rejected") != NULL, "в логе: отказ");

    /* 5. Сбой DDS: регистры сброшены. DIP меняем без кнопки — должна вернуться 4000. */
    set_dip(2000);
    const unsigned resets_before = dds.resets;
    dds_defaults();
    run_until_ms(3000);
    printf("\n[5] сбой DDS\n");
    EXPECT(strstr(g_log, "health: error VERIFY") != NULL, "сбой обнаружен проверкой CTR");
    EXPECT(dds.resets >= resets_before + 2, "DDS повторно сброшен по RSTn");
    expect_output(4000, "восстановлено");

    /* 6. Пропадание +3,3 В. */
    set_pgood(0);
    run_until_ms(3100);
    printf("\n[6] нет +3,3 В, затем снова есть\n");
    EXPECT(!bus_enabled(), "буфер D3 выключен");
    EXPECT((g_avr->data[A_PORTB] & 0x10) == 0, "RSTn = 0");
    set_pgood(1);
    run_until_ms(3500);
    expect_output(4000, "после возврата питания");

    /* 7. Дребезг кнопки: 30 мс по 2 мс, затем удержание. */
    const unsigned sw1 = dds.sel_switches;
    double t = 3600;
    for (int i = 0; i < 15; i++) {
        run_until_ms(t);      set_button(1);
        run_until_ms(t + 1);  set_button(0);
        t += 2;
    }
    press(t, 80);
    run_until_ms(t + 300);
    printf("\n[7] дребезг кнопки, N = 2000\n");
    EXPECT(dds.sel_switches == sw1 + 1, "смен частоты: %u", dds.sel_switches - sw1);
    expect_output(2000, "N=2000");

    printf("\n[8] одиночная ошибка CTR\n");
    unsigned saved_resets = dds.resets;
    g_ctr_xor = 1; g_bad_ctr_reads = 1; g_ctr_reads = 0;
    run_for_ms(1100);
    EXPECT(g_bad_ctr_reads == 0 && g_ctr_reads >= 2, "ошибочное чтение повторено");
    EXPECT(dds.resets == saved_resets, "нет лишнего сброса DDS");
    expect_output(2000, "после одиночной ошибки CTR");

    printf("\n[9] резервные биты CTR\n");
    g_ctr_xor = 0x8FE0; g_bad_ctr_reads = 10;
    run_for_ms(1100);
    EXPECT(g_bad_ctr_reads < 10, "проверка CTR выполнена");
    EXPECT(dds.resets == saved_resets, "резервные биты не вызывают сброс");

    printf("\n[10] три различных неверных чтения CTR\n");
    g_ctr_xor = 1; g_alternating_ctr = true; g_bad_ctr_reads = 3;
    run_for_ms(1800);
    EXPECT(g_bad_ctr_reads == 0 && dds.resets > saved_resets, "устойчивые ошибки вызывают восстановление");
    g_alternating_ctr = false;
    expect_output(2000, "после повторных ошибок CTR");

    printf("\n[11] потеря PGOOD внутри SPI-кадра\n");
    g_drop_spi_byte = 1;
    /* Дождаться ближайшего обмена; отключить PGOOD на первом байте. */
    for (unsigned i = 0; i < 1100 && g_drop_spi_byte; i++) run_for_ms(1);
    EXPECT(g_drop_spi_byte == 0, "провал питания внесён во время SPI");
    run_for_ms(2);
    EXPECT(!bus_enabled(), "D3 отключён после текущего кадра");
    const unsigned frames_after_loss = dds.frames;
    run_for_ms(50);
    EXPECT(dds.frames == frames_after_loss, "новых кадров без питания нет");
    set_pgood(1); run_for_ms(400);
    expect_output(2000, "после питания, потерянного во время SPI");

    printf("\nкадров SPI: %u, сбросов DDS: %u\n", dds.frames, dds.resets);
    printf("%s: %d ошибок\n", g_fail ? "FAILED" : "PASSED", g_fail);
    return g_fail ? 1 : 0;
}
