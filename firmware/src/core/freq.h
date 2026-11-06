/**
 * @file freq.h
 * @brief Расчёт частоты и приращения фазы DDS. Без обращений к аппаратуре:
 *        модуль собирается и для AVR, и для unit-тестов на ПК.
 */
#ifndef CORE_FREQ_H
#define CORE_FREQ_H

#include <stdbool.h>
#include <stdint.h>

/** Параметры шкалы частот: f = base_hz + step_hz · N, N = 0…n_max. */
typedef struct {
    uint32_t base_hz;
    uint32_t step_hz;
    uint16_t n_max;
} freq_scale_t;

/**
 * Код N → частота, Гц.
 * @return false, если N вне диапазона 0…n_max (значение *f_hz не меняется).
 */
bool freq_code_to_hz(const freq_scale_t *scale, uint16_t n, uint32_t *f_hz);

/**
 * Частота → 48-разрядное приращение фазы: round(f · 2^48 / fclk).
 *
 * Деление «столбиком», точно, без 128-битной арифметики и без float.
 * Условия: f_hz < fclk_hz, fclk_hz < 2^31. Погрешность частоты на выходе —
 * не больше fclk / 2^49 (около 0,85 мкГц при 480 МГц).
 */
uint64_t freq_hz_to_ftw(uint32_t f_hz, uint32_t fclk_hz);

/** Разбивка 48-битного слова на регистры dPh_L, dPh_M, dPh_H. */
void freq_ftw_split(uint64_t ftw, uint16_t out[3]);

#endif /* CORE_FREQ_H */
