/**
 * @file uart_log.h
 * @brief Отладочный вывод через USART0: TX = PE1 = площадка X1.1, 38400 8N1.
 *
 * Подключение: USB-UART адаптер 5 В (или 3,3 В с входом, допускающим 5 В):
 * RX адаптера → X1.1, GND → X1.6. Во время ISP-программирования адаптер
 * отключить: X1.1 — линия ISP_PDO.
 *
 * Если DDS_LOG = 0, макросы LOG*() пустые и не занимают памяти.
 */
#ifndef DRIVERS_UART_LOG_H
#define DRIVERS_UART_LOG_H

#include "dds_config.h"

#if DDS_LOG
#include <avr/pgmspace.h>
#include <stdio.h>

void uart_log_init(void);

/** printf с форматной строкой во flash. Пример: LOG("N=%u\r\n", n); */
#define LOG(fmt, ...)  printf_P(PSTR(fmt), ##__VA_ARGS__)
#else
static inline void uart_log_init(void) {}
#define LOG(fmt, ...)  do { } while (0)
#endif

#endif /* DRIVERS_UART_LOG_H */
