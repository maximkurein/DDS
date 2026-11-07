/**
 * @file uart_log.c
 * @brief stdout → USART0 (драйвер uart.c). Порт настраивает uart_init().
 */
#include "uart_log.h"

#if DDS_LOG

#include "uart.h"

static int uart_putchar(char c, FILE *stream)
{
    (void)stream;
    uart_putc(c);
    return 0;
}

static FILE s_uart_out = FDEV_SETUP_STREAM(uart_putchar, NULL, _FDEV_SETUP_WRITE);

void uart_log_init(void)
{
    stdout = &s_uart_out;
}
#endif /* DDS_LOG */
