/**
 * @file osccal.c
 * @brief Загрузка калибровки RC 8 МГц из EEPROM.
 */
#include "osccal.h"

#include <avr/eeprom.h>
#include <avr/io.h>

#include "dds_config.h"

osccal_result_t osccal_apply_from_eeprom(uint8_t *value)
{
    const uint8_t cal = eeprom_read_byte((const uint8_t *)EE_ADDR_OSCCAL);
    const uint8_t inv = eeprom_read_byte((const uint8_t *)EE_ADDR_OSCCAL_INV);

    /*
     * Пара (байт, дополнение) отличает записанную калибровку от чистой
     * EEPROM. У 1887ВЕ7Т стёртая память может читаться и 0x00, и 0xFF,
     * а у этих значений дополнение не совпадёт.
     */
    if ((uint8_t)(cal ^ inv) != 0xFFu) {
        *value = OSCCAL;
        return OSCCAL_MISSING;
    }
    OSCCAL = cal;
    *value = cal;
    return OSCCAL_APPLIED;
}
