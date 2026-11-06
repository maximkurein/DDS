/**
 * @file spi.h
 * @brief Аппаратный SPI ATmega128 (1887ВЕ7Т) в режиме ведущего, кадры 24 бита.
 *
 * Режим SPI 0 (CPOL = 0, CPHA = 0), старшим битом вперёд: данные
 * выставляются по спаду SCK и читаются по фронту, как требует 1508ПЛ8Т
 * (ТО, разд. 6.3, рис. 6.6). SCSn (PB0) опускается на время одного слова.
 */
#ifndef DRIVERS_SPI_H
#define DRIVERS_SPI_H

#include <stdbool.h>
#include <stdint.h>

/** Включить SPI ведущим. Вызывать после board_init(). */
void spi_init(void);

/** Выключить SPI (выводы остаются в состоянии board_init()). */
void spi_deinit(void);

/**
 * Передать 24-битное слово и одновременно принять 24 бита с MISO.
 * @param tx    младшие 24 бита передаются, старшим битом вперёд
 * @param rx    принятое слово (может быть NULL)
 * @return false — модуль SPI не завершил обмен (аппаратный сбой)
 */
bool spi_xfer24(uint32_t tx, uint32_t *rx);

#endif /* DRIVERS_SPI_H */
