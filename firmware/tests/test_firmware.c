/* Regression tests run production app, DDS, board and OSCCAL code.
 * Only hardware registers, SPI transport, EEPROM and time are substituted. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include "unit.h"
#include "app/app.h"
#include "app/remote.h"
#include "core/cmdline.h"
#include "drivers/board.h"
#include "drivers/pl8t.h"
#include "drivers/pl8t_regs.h"
#include "drivers/spi.h"
#include "drivers/systick.h"
#include "drivers/osccal.h"
#include "dds_config.h"
#include <util/delay.h>
#include <avr/eeprom.h>

int g_unit_failures, g_unit_checks;
uint8_t PORTA, DDRA, PINA, PORTB, DDRB, PINB, PORTC, DDRC, PINC;
uint8_t PORTD, DDRD, PORTE, DDRE, PORTF, DDRF, PORTG, DDRG, OSCCAL;
static uint16_t regs[0x10000], address, shifted;
static uint32_t now_ms;
static uint8_t ticks, ee[2];
static unsigned frames, switches, drop_frame, drop_delay, delays, unsafe_frames;
static unsigned ctr_reads, corrupt_reads;
static bool alternating, spi_failure;
static uint16_t ctr_xor;

uint32_t systick_ms(void) { return now_ms; }
uint8_t systick_take_ticks(void) { uint8_t n = ticks; ticks = 0; return n; }
uint8_t eeprom_read_byte(const uint8_t *p) { return ee[(uintptr_t)p]; }

static void power(bool on)
{
    if (on) PINC |= _BV(PC4);
    else PINC &= (uint8_t)~_BV(PC4);
}

static bool bus_on(void) { return (DDRB & _BV(PB5)) != 0 && (PORTB & _BV(PB5)) == 0; }

void test_delay_ms(double ms)
{
    now_ms += (uint32_t)ms;
    if ((unsigned)ticks + (unsigned)ms > 255u) ticks = 255;
    else ticks = (uint8_t)(ticks + (uint8_t)ms);
    if (++delays == drop_delay) power(false);
}

bool spi_xfer24(uint32_t tx, uint32_t *rx)
{
    if (!board_pgood_raw() || !bus_on()) unsafe_frames++;
    frames++;
    if (rx) *rx = shifted;
    const uint8_t cmd = (uint8_t)(tx >> 16);
    const uint16_t value = (uint16_t)tx;
    switch (cmd) {
    case PL8T_CMD_SETA: address = value; shifted = address; break;
    case PL8T_CMD_SETAFT:
        address = value;
        shifted = address == PL8T_DEVID ? PL8T_DEVID_VALUE : regs[address];
        if (address == PL8T_CTR) {
            ctr_reads++;
            if (corrupt_reads) {
                corrupt_reads--;
                shifted = (uint16_t)(shifted ^ (alternating && (ctr_reads & 1u) ? 0x2000u : ctr_xor));
            }
        }
        break;
    case PL8T_CMD_WR:
    case PL8T_CMD_WRI:
        if (address == PL8T_SEL_REG && regs[address] != value) switches++;
        regs[address] = address == PL8T_CTR ? (uint16_t)(value & 0x701Fu) : value;
        if (cmd == PL8T_CMD_WRI) address++;
        shifted = address;
        break;
    default: shifted = PL8T_DEVID_VALUE; break;
    }
    if (frames == drop_frame) power(false);
    return !spi_failure;
}

static void poll_ms(unsigned ms)
{
    for (unsigned i = 0; i < ms; i++) {
        now_ms++;
        if (ticks < 255) ticks++;
        app_poll();
    }
}

static void dip(uint16_t n)
{
    PINA = (uint8_t)~n;
    PINC = (uint8_t)((PINC & 0xF0u) | ((uint16_t)~(n >> 8) & 0x0Fu));
}

static void reset_fixture(void)
{
    memset(regs, 0, sizeof regs);
    address = 0; shifted = 0; now_ms = 0; ticks = 0;
    frames = switches = drop_frame = drop_delay = delays = unsafe_frames = 0;
    ctr_reads = corrupt_reads = 0; ctr_xor = 1; alternating = spi_failure = false;
    board_init();
    PINC = _BV(PC5); /* button released */
    dip(2000);
    power(true);
    app_init();
}

static void reach(app_state_t state)
{
    for (unsigned i = 0; i < 100 && app_state() != state; i++) poll_ms(1);
    CHECK(app_state() == state);
}

static void expect_off(void)
{
    CHECK(!bus_on());
    CHECK((PORTB & _BV(PB4)) == 0);
    CHECK(app_state() == APP_ST_WAIT_POWER);
    CHECK(unsafe_frames == 0);
}

static void test_ctr(void)
{
    reset_fixture(); reach(APP_ST_RUN);
    const unsigned sw = switches;
    ctr_reads = 0; corrupt_reads = 1;
    CHECK(pl8t_check_alive() == PL8T_OK);
    CHECK(ctr_reads == 2); CHECK(switches == sw); CHECK(bus_on());
    /* Reserved bits are ignored; every defined bit is checked, including DAC2/CMP. */
    ctr_xor = 0x8FE0u; corrupt_reads = 3; ctr_reads = 0;
    CHECK(pl8t_check_alive() == PL8T_OK); CHECK(ctr_reads == 1);
    for (unsigned bit = 0; bit < 16; bit++) {
        if (!(0x701Fu & (1u << bit))) continue;
        ctr_xor = (uint16_t)(1u << bit); corrupt_reads = 3; ctr_reads = 0;
        CHECK(pl8t_check_alive() == PL8T_ERR_VERIFY); CHECK(ctr_reads == 3);
    }
    ctr_xor = 1; alternating = true; corrupt_reads = 3; ctr_reads = 0;
    CHECK(pl8t_check_alive() == PL8T_ERR_VERIFY); CHECK(ctr_reads == 3);
    alternating = false; corrupt_reads = 0;
    regs[PL8T_CTR] = 2; /* DDS reset defaults */
    CHECK(pl8t_check_alive() == PL8T_ERR_VERIFY);
    spi_failure = true;
    CHECK(pl8t_check_alive() == PL8T_ERR_SPI);
    spi_failure = false;
    power(false);
    const unsigned before = frames;
    CHECK(pl8t_check_alive() == PL8T_ERR_POWER); CHECK(frames == before);
    CHECK(!bus_on());
}

static void test_input_gaps(void)
{
    reset_fixture(); reach(APP_ST_RUN); poll_ms(30);
    unsigned sw = switches;
    dip(3000);
    /* A fresh press after 255 missed ticks is not a 255 ms hold. */
    PINC &= (uint8_t)~_BV(PC5);
    now_ms += 300; ticks = 255; app_poll();
    CHECK(switches == sw);
    PINC |= _BV(PC5); poll_ms(30); CHECK(switches == sw);
    /* A partially accumulated hold is invalidated by an observation gap. */
    PINC &= (uint8_t)~_BV(PC5); poll_ms(10);
    now_ms += 30; ticks = 30; app_poll();
    poll_ms(APPLY_DEBOUNCE_MS - 2); CHECK(switches == sw);
    poll_ms(1); CHECK(switches == sw + 1);
    PINC |= _BV(PC5); poll_ms(30);
    sw = switches;
    for (unsigned i = 0; i < 10; i++) {
        PINC &= (uint8_t)~_BV(PC5); poll_ms(1);
        PINC |= _BV(PC5); poll_ms(1);
    }
    CHECK(switches == sw);
    reset_fixture(); power(false); poll_ms(10);
    power(true); now_ms += 40; ticks = 40; app_poll();
    poll_ms(DDS_PGOOD_STABLE_MS - 2 + DDS_POWER_SETTLE_MS);
    CHECK(!bus_on()); poll_ms(2); CHECK(bus_on());
    /* Even previously qualified high PGOOD must be requalified after a gap in WAIT. */
    reset_fixture(); poll_ms(15); CHECK(!bus_on());
    now_ms += 50; ticks = 50; app_poll(); CHECK(!bus_on());
    poll_ms(20); CHECK(!bus_on()); poll_ms(12); CHECK(bus_on());
}

static void test_power(void)
{
    const app_state_t states[] = { APP_ST_WAIT_POWER, APP_ST_RESET,
        APP_ST_RECOVER, APP_ST_INIT, APP_ST_RUN, APP_ST_FAULT };
    for (unsigned i = 0; i < sizeof states / sizeof states[0]; i++) {
        reset_fixture();
        if (states[i] == APP_ST_FAULT) {
            reach(APP_ST_RUN); regs[PL8T_CTR] = 2;
            now_ms += DDS_HEALTH_PERIOD_MS; ticks = 1; app_poll();
            CHECK(app_state() == APP_ST_FAULT);
        } else reach(states[i]);
        power(false); ticks = 0; app_poll(); /* must work without a timer tick */
        expect_off();
        const unsigned before = frames;
        poll_ms(50); CHECK(frames == before);
        power(true); reach(APP_ST_RUN); CHECK(unsafe_frames == 0);
    }
    reset_fixture(); reach(APP_ST_INIT);
    poll_ms(1);
    const unsigned startup_frames = frames;
    /* Lose power after EVERY frame of init and first frequency application. */
    for (unsigned frame = 1; frame <= startup_frames; frame++) {
        reset_fixture(); reach(APP_ST_INIT); drop_frame = frame;
        poll_ms(1); expect_off(); CHECK(frames == frame);
        const unsigned before = frames; poll_ms(10); CHECK(frames == before);
        drop_frame = 0; power(true); reach(APP_ST_RUN);
    }
    /* Power loss during the blocking DIP wait. */
    reset_fixture(); reach(APP_ST_RUN); poll_ms(30);
    drop_delay = delays + 1;
    PINC &= (uint8_t)~_BV(PC5); poll_ms(APPLY_DEBOUNCE_MS);
    expect_off();
}

static void test_calibration(void)
{
    uint8_t value;
    for (unsigned cal = 0; cal < 256; cal++) {
        ee[0] = (uint8_t)cal; ee[1] = (uint8_t)~cal; OSCCAL = 0x55;
        CHECK(osccal_apply_from_eeprom(&value) == OSCCAL_APPLIED);
        CHECK(value == cal && OSCCAL == cal);
    }
    const uint8_t invalid[][2] = { {255, 255}, {0, 0}, {0x9E, 0x60} };
    for (unsigned i = 0; i < sizeof invalid / sizeof invalid[0]; i++) {
        memcpy(ee, invalid[i], sizeof ee); OSCCAL = 0x55;
        CHECK(osccal_apply_from_eeprom(&value) == OSCCAL_MISSING);
        CHECK(value == 0x55 && OSCCAL == 0x55);
    }
}

static char reply[64];

static const char *cmd(const char *line)
{
    const cmd_t c = cmd_parse(line);
    remote_execute(&c, reply);
    return reply;
}

static void poll_remote(unsigned ms)
{
    for (unsigned i = 0; i < ms; i++) {
        poll_ms(1);
        remote_hop_tick();
    }
}

static void test_remote(void)
{
    reset_fixture(); remote_init(); reach(APP_ST_RUN);
    CHECK(strncmp(cmd("?"), "PL8T test DDS48-52", 18) == 0);
    /* Локальный режим: доступ к DDS закрыт, состояние читается. */
    CHECK(strcmp(cmd("W 0402 1234"), "ERR LOCAL") == 0);
    CHECK(strcmp(cmd("HOP 4 10"), "ERR LOCAL") == 0);
    CHECK(strcmp(cmd("ST"), "OK pgood=1 apply=0 dip=2000 remote=0 err=0 hop=0 ms=0") == 0);
    CHECK(strcmp(cmd("BAD"), "ERR CMD") == 0);
    CHECK(strcmp(cmd("W 1"), "ERR ARG") == 0);

    CHECK(strcmp(cmd("REMOTE 1"), "OK") == 0);
    CHECK(app_remote() && app_state() == APP_ST_RUN);
    CHECK(strcmp(cmd("W 0402 1234"), "OK") == 0);
    CHECK(regs[0x402] == 0x1234u);
    CHECK(strcmp(cmd("R 0402"), "OK 1234") == 0);
    CHECK(strcmp(cmd("R 0001"), "OK 0201") == 0);
    CHECK(strncmp(cmd("S B00402"), "OK 00", 5) == 0);
    CHECK(strcmp(cmd("S 000000"), "OK 001234") == 0);

    /* Кнопка и проверка CTR не трогают DDS в режиме REMOTE. */
    const unsigned f0 = frames;
    regs[PL8T_CTR] = 0;
    PINC &= (uint8_t)~_BV(PC5); poll_ms(APPLY_DEBOUNCE_MS + 2u);
    PINC |= _BV(PC5); poll_ms(DDS_HEALTH_PERIOD_MS + 10u);
    CHECK(frames == f0); CHECK(app_state() == APP_ST_RUN);

    /* Перебор: профиль канала 2 сохраняется, канал 1 идёт 0,1,2,0… */
    regs[PL8T_SEL_REG] = 0x0500u;
    CHECK(strcmp(cmd("HOP 3 10"), "OK") == 0);
    poll_remote(1); CHECK(regs[PL8T_SEL_REG] == 0x0500u);
    poll_remote(10); CHECK(regs[PL8T_SEL_REG] == 0x0501u);
    poll_remote(10); CHECK(regs[PL8T_SEL_REG] == 0x0502u);
    poll_remote(10); CHECK(regs[PL8T_SEL_REG] == 0x0500u);
    CHECK(strstr(cmd("ST"), "hop=3 ms=10") != NULL);
    CHECK(strcmp(cmd("HOP 0"), "OK") == 0);
    poll_remote(30); CHECK(regs[PL8T_SEL_REG] == 0x0500u);

    /* Провал питания: ошибка, после возврата — RUN без INIT (CTR не пишется). */
    power(false);
    CHECK(strcmp(cmd("W 0402 1"), "ERR POWER") == 0);
    poll_ms(1); CHECK(app_state() == APP_ST_WAIT_POWER); CHECK(!bus_on());
    power(true); reach(APP_ST_RUN);
    CHECK(app_remote()); CHECK(regs[PL8T_CTR] == 0u);
    CHECK(strcmp(cmd("RST"), "OK") == 0);
    CHECK((PORTB & _BV(PB4)) != 0);

    /* Возврат к DIP: перезапуск и частота с переключателей. */
    CHECK(strcmp(cmd("REMOTE 0"), "OK") == 0);
    CHECK(!app_remote()); CHECK(app_state() == APP_ST_WAIT_POWER);
    for (unsigned i = 0; i < 2000 && app_state() != APP_ST_RUN; i++) poll_ms(1);
    CHECK(app_state() == APP_ST_RUN);
    CHECK(regs[PL8T_CTR] != 0u);
    CHECK(strcmp(cmd("R 0002"), "ERR LOCAL") == 0);
    CHECK(unsafe_frames == 0);
}

int main(void)
{
    puts("CTR: bounded retries and register mask"); test_ctr();
    puts("Input sampling after blocked polls"); test_input_gaps();
    puts("Power loss in every state and SPI frame"); test_power();
    puts("EEPROM calibration validation"); test_calibration();
    puts("UART remote commands"); test_remote();
    printf("%d checks, %d failures\n", g_unit_checks, g_unit_failures);
    return g_unit_failures ? 1 : 0;
}
