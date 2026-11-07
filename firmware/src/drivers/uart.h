/**
 * @file uart.h
 * @brief USART0 на приём и передачу: TXD0 = PE1 = X1.1, RXD0 = PE0 = X1.4, 8N1.
 *
 * Скорость — DDS_UART_BAUD (dds_config.h). Приём по прерыванию в кольцевой
 * буфер, передача — ожиданием (короткие строки ответа).
 *
 * Подключение USB-UART адаптера с логикой 5 В: RX адаптера → X1.1,
 * TX адаптера → X1.4, GND → X1.6. Питание адаптера к плате не подключать.
 * На время ISP-программирования через X1 адаптер отсоединить: PE0/PE1 —
 * линии ISP_PDI/ISP_PDO. Программирование через JTAG (X3) адаптеру не мешает.
 */
#ifndef DRIVERS_UART_H
#define DRIVERS_UART_H

#include <stdbool.h>
#include <stdint.h>

void uart_init(void);
void uart_putc(char c);
void uart_puts(const char *s);
/** Строка из flash (PSTR). */
void uart_puts_P(const char *s);
/** Взять принятый символ. false — буфер пуст. */
bool uart_getc(char *c);
/** Принятые символы были потеряны (переполнение буфера или ошибка кадра). */
bool uart_take_overrun(void);

#endif /* DRIVERS_UART_H */
