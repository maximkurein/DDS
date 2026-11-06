/**
 * @file board.h
 * @brief Распиновка платы DDS 48–52 МГц, ревизия F (D2 1887ВЕ7Т ↔ периферия).
 *
 * Источник — нетлист output/netlist/netlist.xml ревизии F.
 *
 * | Вывод D2 | Порт | Цепь              | Куда                                 |
 * |----------|------|-------------------|--------------------------------------|
 * | 10       | PB0  | MCU_CS            | R70 → D3 → DDS_SCSn (D1.6); R63 ↑5В  |
 * | 11       | PB1  | MCU_SCK           | R66 → D3 → DDS_SCK (D1.2); ISP SCK X1.3 |
 * | 12       | PB2  | MCU_MOSI          | R68 → D3 → DDS_SDI (D1.3)            |
 * | 13       | PB3  | MCU_MISO          | ← D4 (2 инвертора) ← DDS_SDO (D1.5)  |
 * | 14       | PB4  | MCU_DDS_RESET     | R71 → D3 → DDS_RSTn (D1.10), R80 ↓   |
 * | 15       | PB5  | BUF_OEn           | R65 → D3.1 (1OE#), R64 ↑3,3В         |
 * | 51…44    | PA0…PA7 | DIP0…DIP7      | S2.1…S2.8, R48…R55 ↑5В, ON = 0       |
 * | 35…38    | PC0…PC3 | DIP8…DIP11     | S3.1…S3.4, R56…R59 ↑5В, ON = 0       |
 * | 39       | PC4  | DDS_PGOOD         | R88 47к ↑+3,3В — признак +3,3 В      |
 * | 40       | PC5  | APPLYn            | S4 «Применить», R60 ↑5В, C38 100 нФ  |
 * | 2        | PE0  | ISP_PDI / RXD0    | X1.4                                 |
 * | 3        | PE1  | ISP_PDO / TXD0    | X1.1 — отладочный UART               |
 * | 54…57    | PF7…PF4 | JTAG TDI/TDO/TMS/TCK | X3                             |
 */
#ifndef DRIVERS_BOARD_H
#define DRIVERS_BOARD_H

#include <stdbool.h>
#include <stdint.h>
#include <avr/io.h>

/* ---- SPI и управление DDS: порт B ---- */
#define BOARD_SPI_PORT      PORTB
#define BOARD_SPI_DDR       DDRB
#define BOARD_SPI_PIN       PINB
#define BOARD_CS_BIT        PB0   /* активный 0 */
#define BOARD_SCK_BIT       PB1
#define BOARD_MOSI_BIT      PB2
#define BOARD_MISO_BIT      PB3
#define BOARD_DDS_RST_BIT   PB4   /* активный 0: 0 — DDS в сбросе */
#define BOARD_BUF_OEN_BIT   PB5   /* активный 0: 0 — буфер D3 включён */

/* ---- Входы: порты A и C ---- */
#define BOARD_DIP_LO_PIN    PINA          /* DIP0…DIP7 */
#define BOARD_DIP_HI_PIN    PINC          /* DIP8…DIP11 на PC0…PC3 */
#define BOARD_PGOOD_BIT     PC4
#define BOARD_APPLY_BIT     PC5

/** Начальная настройка всех выводов: буфер D3 выключен, DDS в сбросе. */
void board_init(void);

/**
 * Подключение шины SPI к DDS через буфер D3.
 * on = false: OE# в третье состояние (R64 выключает D3), DDS_RSTn тянется
 * к 0 резистором R80 — DDS в сбросе.
 */
void board_dds_bus_enable(bool on);

/** Сброс DDS: true — RSTn = 0, false — RSTn = 1 (действует при включённом D3). */
void board_dds_reset(bool assert_reset);

/** Сырые уровни входов. */
static inline bool board_pgood_raw(void)   { return (PINC & _BV(BOARD_PGOOD_BIT)) != 0u; }
static inline bool board_apply_pressed_raw(void) { return (PINC & _BV(BOARD_APPLY_BIT)) == 0u; }
static inline uint8_t board_dip_lo_raw(void) { return BOARD_DIP_LO_PIN; }
static inline uint8_t board_dip_hi_raw(void) { return BOARD_DIP_HI_PIN; }

#endif /* DRIVERS_BOARD_H */
