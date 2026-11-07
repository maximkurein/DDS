/**
 * @file uart.c
 * @brief USART0: приём в кольцевой буфер по прерыванию, передача ожиданием.
 */
#include "uart.h"

#include <avr/interrupt.h>
#include <avr/io.h>
#include <avr/pgmspace.h>

#include "dds_config.h"

#define BAUD DDS_UART_BAUD
#include <util/setbaud.h>

#define RX_SIZE 64u  /* степень двойки */

static volatile uint8_t s_rx[RX_SIZE];
static volatile uint8_t s_head;
static volatile uint8_t s_tail;
static volatile bool    s_overrun;

ISR(USART0_RX_vect)
{
    const uint8_t status = UCSR0A;
    const uint8_t c = UDR0;
    if ((status & (_BV(FE0) | _BV(DOR0))) != 0u) {
        s_overrun = true;
    }
    const uint8_t next = (uint8_t)((s_head + 1u) & (RX_SIZE - 1u));
    if (next == s_tail) {
        s_overrun = true;
        return;
    }
    s_rx[s_head] = c;
    s_head = next;
}

void uart_init(void)
{
    UBRR0H = UBRRH_VALUE;
    UBRR0L = UBRRL_VALUE;
#if USE_2X
    UCSR0A = _BV(U2X0);
#else
    UCSR0A = 0;
#endif
    UCSR0C = _BV(UCSZ01) | _BV(UCSZ00);                 /* 8N1; у ATmega128 нет бита URSEL */
    UCSR0B = _BV(TXEN0) | _BV(RXEN0) | _BV(RXCIE0);     /* PE1 — TXD0, PE0 — RXD0 */
    s_head = s_tail = 0;
    s_overrun = false;
}

void uart_putc(char c)
{
    while ((UCSR0A & _BV(UDRE0)) == 0u) {
    }
    UDR0 = (uint8_t)c;
}

void uart_puts(const char *s)
{
    while (*s != '\0') {
        uart_putc(*s++);
    }
}

void uart_puts_P(const char *s)
{
    char c;
    while ((c = (char)pgm_read_byte(s++)) != '\0') {
        uart_putc(c);
    }
}

bool uart_getc(char *c)
{
    if (s_tail == s_head) {
        return false;
    }
    *c = (char)s_rx[s_tail];
    s_tail = (uint8_t)((s_tail + 1u) & (RX_SIZE - 1u));
    return true;
}

bool uart_take_overrun(void)
{
    const uint8_t sreg = SREG;
    cli();
    const bool o = s_overrun;
    s_overrun = false;
    SREG = sreg;
    return o;
}
