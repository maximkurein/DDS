/**
 * @file pl8t.h
 * @brief Драйвер синтезатора 1508ПЛ8Т (канал 1, ЦАП 1) через SPI.
 *
 * Смена частоты без промежуточных значений: два профиля (0 и 1) работают
 * попеременно. Новое приращение фазы пишется в неактивный профиль,
 * проверяется чтением и только потом включается записью в SEL_REG.
 * ТО, разд. 3.2: неактивный профиль можно программировать «на лету».
 */
#ifndef DRIVERS_PL8T_H
#define DRIVERS_PL8T_H

#include <stdbool.h>
#include <stdint.h>

typedef enum {
    PL8T_OK = 0,
    PL8T_ERR_SPI,      /**< модуль SPI не завершил обмен */
    PL8T_ERR_DEVID,    /**< DEVID != 0x0201: нет питания, такта или связи */
    PL8T_ERR_VERIFY,   /**< записанное значение не совпало при чтении */
    PL8T_ERR_POWER,    /**< низкий PGOOD: обмен прекращён, D3 выключен */
} pl8t_status_t;

/** Настройки, которые пишутся при инициализации. */
typedef struct {
    uint16_t amplitude_mul;  /**< поле mul регистра CH1_Muly, 0…4095 */
    uint8_t  clk_cmx;        /**< CTR.cmx — делитель такта (при CSEL = 0) */
    uint8_t  tsw;            /**< CH1_TSW.tsw — сглаживание переключения */
    bool     phase_dither;   /**< ROUTE.pdith1 */
} pl8t_config_t;

/** Низкий уровень: запись и чтение одного 16-битного регистра. */
pl8t_status_t pl8t_write(uint16_t addr, uint16_t value);
pl8t_status_t pl8t_read(uint16_t addr, uint16_t *value);

/** Запись подряд идущих регистров: SETA + n команд WRI. */
pl8t_status_t pl8t_write_block(uint16_t addr, const uint16_t *values, uint8_t n);

/** Чтение подряд идущих регистров конвейером команд SETAFT. */
pl8t_status_t pl8t_read_block(uint16_t addr, uint16_t *values, uint8_t n);

/**
 * Проверка связи и настройка после аппаратного сброса: DEVID, CTR (только
 * ЦАП1), SYNC = 0 (выходы синхронизации выкл.), LINK = 0, ROUTE, CH1_TSW,
 * профили 0 и 1 (фаза 0, амплитуда, смещение 0, частота 0), SEL_REG = 0.
 * ЦАП 1 остаётся выключенным до первого pl8t_set_ftw().
 */
pl8t_status_t pl8t_init(const pl8t_config_t *cfg);

/**
 * Установить 48-битное приращение фазы канала 1 и включить ЦАП 1.
 * Запись в неактивный профиль → проверка → переключение SEL_REG.
 */
pl8t_status_t pl8t_set_ftw(uint64_t ftw);

/** Выключить ЦАП 1 (на выходе нет сигнала). */
pl8t_status_t pl8t_output_off(void);

/**
 * Проверка, что DDS не потерял настройки (например, после провала питания
 * он сбрасывается и CTR возвращается к 0x0002). До трёх чтений CTR по
 * маске 0x701F; любое корректное чтение подтверждает связь. Три неверных
 * чтения (в том числе различных) дают VERIFY, ошибка SPI/питания — сразу.
 */
pl8t_status_t pl8t_check_alive(void);

/** Номер активного профиля канала 1 (0 или 1). */
uint8_t pl8t_active_profile(void);

#endif /* DRIVERS_PL8T_H */
