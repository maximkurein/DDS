/**
 * @file main.c
 * @brief Прошивка контроллера D2 (1887ВЕ7Т) синтезатора DDS 48–52 МГц.
 *
 * Сборка — под ATmega128 (-mmcu=atmega128), программирование — профилем
 * 1887ВЕ7Т (tools/avrdude/1887ve7t.conf). См. README.md.
 */
#include <avr/interrupt.h>
#include <avr/io.h>
#include <avr/wdt.h>
#include <stdint.h>

#include "app/app.h"
#include "dds_config.h"
#include "drivers/board.h"
#include "drivers/osccal.h"
#include "drivers/spi.h"
#include "drivers/systick.h"
#include "drivers/uart_log.h"
#include "fw_version.h"

/** Причина последнего сброса (копия MCUCSR до очистки). */
static uint8_t s_reset_cause __attribute__((section(".noinit")));

/*
 * Выполняется до инициализации .data/.bss (секция .init3): сохраняем флаги
 * сброса и выключаем сторожевой таймер, если он остался включён.
 * Бит JTD (MCUCSR.7) не трогаем: JTAG должен оставаться включённым.
 */
void early_init(void) __attribute__((naked, used, section(".init3")));
void early_init(void)
{
    s_reset_cause = MCUCSR;
    MCUCSR = (uint8_t)(MCUCSR & ~(_BV(JTRF) | _BV(WDRF) | _BV(BORF) | _BV(EXTRF) | _BV(PORF)));
    wdt_disable();
}

int main(void)
{
    uint8_t cal;

    board_init();
    const osccal_result_t cal_res = osccal_apply_from_eeprom(&cal);
    uart_log_init();      /* после OSCCAL: от него зависит точность скорости UART */
    systick_init();
    spi_init();
#if DDS_WDT
    wdt_enable(DDS_WDT_TIMEOUT);
#endif
    sei();

    LOG("\r\nDDS 48-52 MHz, fw " DDS_FW_VERSION "\r\n");
    LOG("[boot] MCUCSR=0x%02X OSCCAL=0x%02X (%s)\r\n", s_reset_cause, cal,
        cal_res == OSCCAL_APPLIED ? "EEPROM" : "factory 1 MHz, run 'osccal' target");
    (void)cal_res;
    (void)cal;

    app_init();
    for (;;) {
#if DDS_WDT
        wdt_reset();
#endif
        app_poll();
    }
}
