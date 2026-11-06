/**
 * @file osccal.h
 * @brief Калибровка внутреннего RC-генератора 8 МГц.
 *
 * ТО 1887ВЕ7Т (с. 300): при сбросе в OSCCAL автоматически загружается
 * калибровка для 1 МГц. Для 8 МГц нужен свой байт — 4-й байт калибровочной
 * области (адрес 3). Программа его прочитать не может, поэтому программатор
 * переносит его в EEPROM (цель `osccal` в CMake), а прошивка применяет
 * его при старте.
 */
#ifndef DRIVERS_OSCCAL_H
#define DRIVERS_OSCCAL_H

#include <stdbool.h>
#include <stdint.h>

typedef enum {
    OSCCAL_APPLIED,    /**< байт из EEPROM применён */
    OSCCAL_MISSING,    /**< в EEPROM нет корректной пары (байт, ~байт) */
} osccal_result_t;

/** Прочитать калибровку из EEPROM и записать в OSCCAL. */
osccal_result_t osccal_apply_from_eeprom(uint8_t *value);

#endif /* DRIVERS_OSCCAL_H */
