/**
 * @file uart_log.c
 * @brief USART0 только на передачу, stdout → UART.
 */
#include "uart_log.h"

#if DDS_LOG

#include <avr/io.h>

#define BAUD DDS_LOG_BAUD
#include <util/setbaud.h>

static int uart_putchar(char c, FILE *stream)
{
    (void)stream;
    while ((UCSR0A & _BV(UDRE0)) == 0u) {
    }
    UDR0 = (uint8_t)c;
    return 0;
}

static FILE s_uart_out = FDEV_SETUP_STREAM(uart_putchar, NULL, _FDEV_SETUP_WRITE);

void uart_log_init(void)
{
    UBRR0H = UBRRH_VALUE;
    UBRR0L = UBRRL_VALUE;
#if USE_2X
    UCSR0A = _BV(U2X0);
#else
    UCSR0A = 0;
#endif
    UCSR0C = _BV(UCSZ01) | _BV(UCSZ00);   /* 8N1; у ATmega128 нет бита URSEL */
    UCSR0B = _BV(TXEN0);                  /* PE1 становится выходом TXD0 */
    stdout = &s_uart_out;
}

#endif /* DDS_LOG */
