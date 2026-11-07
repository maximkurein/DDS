/**
 * @file remote.h
 * @brief Управление DDS с ПК по USART0 (приложение DDS Studio).
 *
 * Команды разбирает core/cmdline.c, здесь — их выполнение. Пока режим
 * REMOTE выключен, отвечают только «?», «ST» и «REMOTE»; остальные — ERR LOCAL.
 * В режиме REMOTE автомат app.c не трогает DDS: DIP, кнопка S4 и проверка CTR
 * отключены, а контроль PGOOD и отключение шины при провале питания остаются.
 */
#ifndef APP_REMOTE_H
#define APP_REMOTE_H

#include <stdint.h>

#include "core/cmdline.h"

/** Сброс состояния разбора и перебора профилей. */
void remote_init(void);

/** Принять символы из UART, выполнить готовые команды, вести перебор профилей. */
void remote_poll(void);

/**
 * Выполнить одну команду и записать ответ (без перевода строки) в reply.
 * Отдельно от UART — для unit-тестов. reply — не меньше 64 байт.
 */
void remote_execute(const cmd_t *cmd, char *reply);

/** Перебор профилей: шаг по времени (вызывается из remote_poll). */
void remote_hop_tick(void);

#endif /* APP_REMOTE_H */
