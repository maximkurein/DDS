/**
 * @file board.c
 * @brief Начальная настройка выводов D2 по нетлисту ревизии F.
 */
#include "board.h"

void board_init(void)
{
    /*
     * Порт A: DIP0…DIP7. Внешние подтяжки 10 кОм к +5 В (R48…R55),
     * внутренние не нужны.
     */
    DDRA  = 0x00;
    PORTA = 0x00;

    /*
     * Порт B. По design_notes, разд. 5: до включения D3 выставить
     * CS = 1, SCK = 0, MOSI = 0, RESET = 0.
     *   PB0 CS     — выход, 1
     *   PB1 SCK    — выход, 0 (режим SPI 0: в покое 0, совпадает с R77)
     *   PB2 MOSI   — выход, 0
     *   PB3 MISO   — вход без подтяжки (D4 всегда управляет линией)
     *   PB4 RESET  — выход, 0 (DDS в сбросе)
     *   PB5 OE#    — вход без подтяжки: R64 к +3,3 В держит D3 выключенным
     *   PB6, PB7   — не подключены: вход с подтяжкой
     */
    PORTB = _BV(BOARD_CS_BIT) | _BV(PB6) | _BV(PB7);
    DDRB  = _BV(BOARD_CS_BIT) | _BV(BOARD_SCK_BIT) | _BV(BOARD_MOSI_BIT) | _BV(BOARD_DDS_RST_BIT);

    /*
     * Порт C: PC0…PC3 — DIP8…DIP11 (подтяжки R56…R59), PC5 — кнопка (R60).
     * PC4 DDS_PGOOD — БЕЗ внутренней подтяжки: иначе вход покажет 1 и без
     * +3,3 В. PC6, PC7 не подключены — подтяжка.
     */
    DDRC  = 0x00;
    PORTC = _BV(PC6) | _BV(PC7);

    /* Порт D не используется: все выводы — вход с подтяжкой. */
    DDRD  = 0x00;
    PORTD = 0xFF;

    /*
     * Порт E: PE0/PE1 — площадки ISP X1.4/X1.1 (в рабочем режиме — USART0),
     * PE2…PE7 не подключены. Всё — вход с подтяжкой. Если включён лог,
     * uart_log_init() переключит PE1 на передатчик USART0.
     */
    DDRE  = 0x00;
    PORTE = 0xFF;

    /*
     * Порт F: PF4…PF7 заняты JTAG (при JTAGEN = 0 и JTD = 0 управляет
     * TAP-контроллер). PF0…PF3 не подключены — подтяжка.
     */
    DDRF  = 0x00;
    PORTF = 0x0F;

    /* Порт G (5 разрядов) не используется. */
    DDRG  = 0x00;
    PORTG = 0x1F;
}

void board_dds_bus_enable(bool on)
{
    if (on) {
        /* OE# = 0: сначала уровень 0, потом выход, чтобы не было импульса 1. */
        BOARD_SPI_PORT &= (uint8_t)~_BV(BOARD_BUF_OEN_BIT);
        BOARD_SPI_DDR  |= _BV(BOARD_BUF_OEN_BIT);
    } else {
        /* Третье состояние: R64 подтягивает OE# к +3,3 В без тока через защитные диоды D3. */
        BOARD_SPI_DDR  &= (uint8_t)~_BV(BOARD_BUF_OEN_BIT);
        BOARD_SPI_PORT &= (uint8_t)~_BV(BOARD_BUF_OEN_BIT);
    }
}

void board_dds_reset(bool assert_reset)
{
    if (assert_reset) {
        BOARD_SPI_PORT &= (uint8_t)~_BV(BOARD_DDS_RST_BIT);
    } else {
        BOARD_SPI_PORT |= _BV(BOARD_DDS_RST_BIT);
    }
}
