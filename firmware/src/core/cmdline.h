/**
 * @file cmdline.h
 * @brief Разбор текстовых команд UART-протокола (приложение DDS Studio).
 *
 * Без обращений к аппаратуре: модуль собирается и для AVR, и для unit-тестов.
 * Протокол — docs/PROTOCOL_ru.md в studio/, одна команда на строку, 38400 8N1:
 *
 *   ?               → PL8T <версия> DDS48-52 uart1
 *   REMOTE 1|0      → OK
 *   W aaaa dddd     → OK               запись регистра DDS (hex)
 *   R aaaa          → OK dddd          чтение регистра DDS (hex)
 *   S xxxxxx        → OK yyyyyy        один 24-битный кадр SPI (hex)
 *   RST             → OK               импульс RSTn
 *   PIN RSTn 0|1    → OK               удерживать / отпустить сброс DDS
 *   HOP n ms        → OK               перебор профилей 0…n−1 канала 1 (n < 2 — стоп)
 *   ST              → OK pgood=1 apply=0 dip=2000 remote=1 err=0 hop=0 ms=0
 *
 * Ошибки: ERR ARG, ERR CMD, ERR LOCAL, ERR POWER, ERR STATE, ERR SPI, ERR LONG.
 * Строки журнала отладки ответами не являются: приложение принимает только
 * строки, начинающиеся с OK, ERR или PL8T.
 */
#ifndef CORE_CMDLINE_H
#define CORE_CMDLINE_H

#include <stdbool.h>
#include <stdint.h>

#define CMDLINE_MAX 32u

typedef enum {
    CMD_EMPTY = 0,  /**< пустая строка — без ответа */
    CMD_IDENT,
    CMD_REMOTE,     /**< a = 0|1 */
    CMD_WRITE,      /**< a = адрес, b = данные */
    CMD_READ,       /**< a = адрес */
    CMD_SPI,        /**< x = 24-битный кадр */
    CMD_RESET,
    CMD_PIN,        /**< a = уровень RSTn */
    CMD_STATUS,
    CMD_HOP,        /**< a = число профилей, x = период, мс */
    CMD_ERR_ARG,    /**< известная команда, неверные аргументы */
    CMD_ERR_LONG,   /**< строка длиннее CMDLINE_MAX */
    CMD_UNKNOWN,
} cmd_kind_t;

typedef struct {
    cmd_kind_t kind;
    uint16_t   a;
    uint16_t   b;
    uint32_t   x;
} cmd_t;

typedef struct {
    char    buf[CMDLINE_MAX + 1u];
    uint8_t len;
    bool    overflow;
} cmdline_t;

void cmdline_init(cmdline_t *cl);

/**
 * Принять очередной символ. Строка завершается '\n' или '\r' (CRLF даёт
 * одну команду и одну пустую строку, которая игнорируется).
 * @return true — команда готова и записана в *out
 */
bool cmdline_feed(cmdline_t *cl, char c, cmd_t *out);

/** Разобрать одну строку (без символа конца строки). */
cmd_t cmd_parse(const char *line);

/** Запись числа: hex с ведущими нулями (digits знаков) / десятичное. Возвращает длину. */
uint8_t cmd_fmt_hex(char *dst, uint32_t v, uint8_t digits);
uint8_t cmd_fmt_dec(char *dst, uint32_t v);

#endif /* CORE_CMDLINE_H */
