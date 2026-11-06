/**
 * @file spi.c
 * @brief Аппаратный SPI ведущий, 24-битные кадры для 1508ПЛ8Т.
 */
#include "spi.h"

#include <avr/io.h>
#include <util/delay.h>

#include "board.h"
#include "dds_config.h"

/* ---- Делитель частоты SPI (ATmega128: SPR1:0 и SPI2X) ---- */
#if   DDS_SPI_PRESCALER == 2
#  define SPI_SPCR_DIV 0u
#  define SPI_SPSR_2X  1u
#elif DDS_SPI_PRESCALER == 4
#  define SPI_SPCR_DIV 0u
#  define SPI_SPSR_2X  0u
#elif DDS_SPI_PRESCALER == 8
#  define SPI_SPCR_DIV _BV(SPR0)
#  define SPI_SPSR_2X  1u
#elif DDS_SPI_PRESCALER == 16
#  define SPI_SPCR_DIV _BV(SPR0)
#  define SPI_SPSR_2X  0u
#elif DDS_SPI_PRESCALER == 32
#  define SPI_SPCR_DIV _BV(SPR1)
#  define SPI_SPSR_2X  1u
#elif DDS_SPI_PRESCALER == 64
#  define SPI_SPCR_DIV _BV(SPR1)
#  define SPI_SPSR_2X  0u
#elif DDS_SPI_PRESCALER == 128
#  define SPI_SPCR_DIV (_BV(SPR1) | _BV(SPR0))
#  define SPI_SPSR_2X  0u
#else
#  error "DDS_SPI_PRESCALER must be 2, 4, 8, 16, 32, 64 or 128"
#endif

/*
 * Защита от зависания: байт передаётся за 8 · DDS_SPI_PRESCALER тактов,
 * один проход цикла ожидания — не меньше 4 тактов. Берём запас в 4 раза.
 */
#define SPI_WAIT_LOOPS ((uint16_t)(8u * DDS_SPI_PRESCALER))

static inline void cs_low(void)  { BOARD_SPI_PORT &= (uint8_t)~_BV(BOARD_CS_BIT); }
static inline void cs_high(void) { BOARD_SPI_PORT |= _BV(BOARD_CS_BIT); }

void spi_init(void)
{
    /*
     * PB0 (SS) уже выход с уровнем 1 (board_init) — это обязательное условие:
     * если SS станет входом и придёт 0, модуль SPI сам уйдёт в режим ведомого.
     */
    cs_high();
    SPCR = _BV(SPE) | _BV(MSTR) | SPI_SPCR_DIV;  /* режим 0, MSB first */
    SPSR = SPI_SPSR_2X ? _BV(SPI2X) : 0u;
    (void)SPSR;                                   /* сброс SPIF чтением SPSR и SPDR */
    (void)SPDR;
}

void spi_deinit(void)
{
    SPCR = 0;
    cs_high();
}

static bool xfer8(uint8_t tx, uint8_t *rx)
{
    SPDR = tx;
    for (uint16_t i = 0; i < SPI_WAIT_LOOPS; i++) {
        if (SPSR & _BV(SPIF)) {
            *rx = SPDR;
            return true;
        }
    }
    *rx = 0;
    return false;
}

bool spi_xfer24(uint32_t tx, uint32_t *rx)
{
    uint8_t b2, b1, b0;
    bool ok;

    cs_low();
    ok  = xfer8((uint8_t)(tx >> 16), &b2);
    ok &= xfer8((uint8_t)(tx >> 8), &b1);
    ok &= xfer8((uint8_t)tx, &b0);
    cs_high();

    /*
     * Команда выполняется по фронту SCSn. Держим SCSn = 1 не меньше 2 мкс:
     * с запасом на задержки буфера D3 и RC-цепочек 1 кОм.
     */
    _delay_us(2);

    if (rx != 0) {
        *rx = ((uint32_t)b2 << 16) | ((uint32_t)b1 << 8) | b0;
    }
    return ok;
}
