/**
 * @file systick.h
 * @brief Системное время 1 мс на Timer0 (режим CTC).
 */
#ifndef DRIVERS_SYSTICK_H
#define DRIVERS_SYSTICK_H

#include <stdbool.h>
#include <stdint.h>

/** Запуск Timer0: F_CPU / 64 / 125 = 1 кГц при 8 МГц. Нужен sei(). */
void systick_init(void);

/** Миллисекунды от старта (переполнение через 49 суток, сравнения через разность). */
uint32_t systick_ms(void);

/** true, если с момента since прошло не меньше period_ms. */
static inline bool systick_elapsed(uint32_t since, uint32_t period_ms)
{
    return (uint32_t)(systick_ms() - since) >= period_ms;
}

/**
 * Флаг «прошла миллисекунда» для периодических задач главного цикла.
 * Возвращает число тиков с прошлого вызова (обычно 0 или 1) и обнуляет его.
 */
uint8_t systick_take_ticks(void);

#endif /* DRIVERS_SYSTICK_H */
