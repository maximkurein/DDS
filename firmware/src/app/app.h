/**
 * @file app.h
 * @brief Логика изделия: запуск DDS, задание частоты с DIP, кнопка «Применить».
 *
 * Конечный автомат; SPI, чтение DIP и UART-лог блокируют основной цикл.
 * PGOOD проверяется в каждом опросе, между кадрами SPI и ожиданиями DIP.
 *
 *   WAIT_POWER ──(+3,3 В есть, прошло ≥ DDS_POWER_SETTLE_MS)──► RESET
 *   RESET ──(импульс RSTn)──► RECOVER ──(пауза)──► INIT
 *   INIT ──ok──► RUN (частота с DIP при первом старте или последняя заданная)
 *   INIT ──ошибка──► FAULT ──(DDS_RETRY_PERIOD_MS)──► WAIT_POWER
 *   RUN  ──кнопка──► чтение DIP → новая частота или отказ (N > 4000)
 *   RESET/RECOVER/INIT/RUN ──нет +3,3 В──► WAIT_POWER
 *   RUN ──DDS сбросился──► FAULT → повторная настройка
 */
#ifndef APP_APP_H
#define APP_APP_H

#include <stdint.h>

typedef enum {
    APP_ST_WAIT_POWER = 0,
    APP_ST_RESET,
    APP_ST_RECOVER,
    APP_ST_INIT,
    APP_ST_RUN,
    APP_ST_FAULT,
} app_state_t;

/** Начальное состояние. Вызывать после инициализации драйверов и sei(). */
void app_init(void);

/** Один проход главного цикла. */
void app_poll(void);

/** Текущее состояние (для отладчика и тестов). */
app_state_t app_state(void);

#endif /* APP_APP_H */
