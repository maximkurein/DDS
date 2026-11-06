/**
 * @file dip_code.h
 * @brief Декодирование DIP-переключателей S2 (8 разрядов) и S3 (4 разряда).
 *
 * Схема (ревизия F): каждый вывод подтянут к +5 В резистором 10 кОм,
 * переключатель в положении ON замыкает его на землю. ON = бит 1, то есть
 * бит кода равен инверсии уровня на выводе.
 *   S2.1…S2.8 → PA0…PA7 → биты 0…7
 *   S3.1…S3.4 → PC0…PC3 → биты 8…11
 */
#ifndef CORE_DIP_CODE_H
#define CORE_DIP_CODE_H

#include <stdint.h>

/** Маска разрядов кода на порту C (PC0…PC3). */
#define DIP_PORTC_MASK 0x0Fu

/**
 * @param pina  значение PINA (все 8 разрядов — DIP0…DIP7)
 * @param pinc  значение PINC (используются PC0…PC3 — DIP8…DIP11)
 * @return 12-разрядный код N = 0…4095
 */
static inline uint16_t dip_decode(uint8_t pina, uint8_t pinc)
{
    uint16_t lo = (uint8_t)~pina;
    uint16_t hi = (uint8_t)(~pinc) & DIP_PORTC_MASK;
    return (uint16_t)(lo | (hi << 8));
}

#endif /* CORE_DIP_CODE_H */
