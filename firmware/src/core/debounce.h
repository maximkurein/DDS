/**
 * @file debounce.h
 * @brief Подавление дребезга дискретного входа счётчиком устойчивости.
 *
 * debounce_update() вызывается с постоянным периодом (1 мс). Новое состояние
 * принимается, когда сырой уровень отличается от принятого threshold вызовов
 * подряд. Чистая логика — тестируется на ПК.
 */
#ifndef CORE_DEBOUNCE_H
#define CORE_DEBOUNCE_H

#include <stdbool.h>
#include <stdint.h>

typedef struct {
    bool    state;      /**< принятое (устойчивое) состояние */
    uint8_t count;      /**< сколько отсчётов подряд сырой уровень != state */
    uint8_t threshold;  /**< сколько отсчётов нужно для смены состояния */
} debounce_t;

/** Начальное состояние: state = initial, threshold ≥ 1. */
void debounce_init(debounce_t *d, bool initial, uint8_t threshold);

/**
 * Очередной отсчёт.
 * @return true, если принятое состояние только что изменилось.
 */
bool debounce_update(debounce_t *d, bool raw);

#endif /* CORE_DEBOUNCE_H */
