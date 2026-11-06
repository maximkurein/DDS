/**
 * @file pl8t_regs.h
 * @brief Карта регистров и команды SPI синтезатора 1508ПЛ8Т.
 *
 * Источник: «Микросхема интегральная 1508ПЛ8Т. Техническое описание»,
 * АО НПЦ «ЭЛВИС», ред. 14.10.2020: табл. 5.1–5.21 (регистры), табл. 6.1
 * (команды последовательного порта), разд. 6.3 (протокол SPI).
 *
 * Протокол SPI (разд. 6.3):
 *  - кадр 24 бита, старшим битом вперёд: [23:16] код команды, [15:0] параметр;
 *  - SDI читается по фронту SCK, SDO меняется по спаду SCK (режимы SPI 0 и 3);
 *  - команда выполняется по положительному фронту SCSn, то есть SCSn
 *    поднимается после каждого 24-битного слова;
 *  - на входах CSn, RDn, WRn должен быть высокий уровень (R7, R8, R9 на плате).
 *  - После команды сдвиговый регистр получает новое значение (табл. 6.1), и оно
 *    выдвигается на SDO в СЛЕДУЮЩЕМ кадре. Поэтому чтение регистра —
 *    это SETAFT(addr), а затем любая команда, например NOP.
 */
#ifndef PL8T_REGS_H
#define PL8T_REGS_H

#include <stdint.h>

/* ---- Команды последовательного порта (табл. 6.1), старший байт кадра ---- */
#define PL8T_CMD_NOP     0x00u  /**< нет операции; сдвиговый регистр ← DEVID */
#define PL8T_CMD_SETA    0x10u  /**< адресный регистр ← addr */
#define PL8T_CMD_WR      0x20u  /**< [адрес] ← data */
#define PL8T_CMD_WRI     0x30u  /**< [адрес] ← data, затем адрес + 1 */
#define PL8T_CMD_SETAFT  0xB0u  /**< адресный регистр ← addr, сдвиговый регистр ← [addr] */

/* ---- Общие регистры (табл. 5.1) ---- */
#define PL8T_SWRST       0x0000u  /**< W: запись 0x0078 — программный сброс */
#define PL8T_DEVID       0x0001u  /**< R: идентификатор, 0x0201 */
#define PL8T_SEL_REG     0x0002u  /**< RW: активный профиль, сброс 0x0000 */
#define PL8T_CTR         0x0003u  /**< RW: управление, сброс 0x0002 */
#define PL8T_SYNC        0x0004u  /**< RW: синхронизация, сброс 0x4800 */
#define PL8T_CLR         0x0005u  /**< W: очистка аккумуляторов, ЛЧМ */
#define PL8T_LINK        0x0006u  /**< RW: LINK-порт, сброс 0x0000 */
#define PL8T_ROUTE       0x0007u  /**< RW: маршрутизация к ЦАП, сброс 0x0000 */
#define PL8T_TC_L        0x0008u  /**< RW: делитель чиповой скорости [15:0] */
#define PL8T_TC_H        0x0009u  /**< RW: делитель чиповой скорости [31:16] */

#define PL8T_SWRST_KEY   0x0078u
#define PL8T_DEVID_VALUE 0x0201u

/* ---- Регистры каналов: канал 1 — 0x1000…, канал 2 — 0x2000… ---- */
#define PL8T_CH_BASE(ch)        ((uint16_t)((ch) == 1u ? 0x1000u : 0x2000u))
#define PL8T_CH_LS_CTR(ch)      ((uint16_t)(PL8T_CH_BASE(ch) + 0x000u)) /**< ЛЧМ; 0 — выкл */
#define PL8T_CH_TSW(ch)         ((uint16_t)(PL8T_CH_BASE(ch) + 0x002u)) /**< гауссов фильтр */

/*
 * Профили модуляции y = 0…63 (табл. 5.1): профиль 0 — 0x1400…0x1406,
 * профили 1…63 — 0x1410…0x17F6, то есть шаг 0x10. Внутри профиля:
 *   +0 dPh_L [15:0], +1 dPh_M [31:16], +2 dPh_H [47:32], +3 — резерв,
 *   +4 P (фаза), +5 Mul (амплитуда), +6 Offset (смещение).
 */
#define PL8T_PROFILE_COUNT      64u
#define PL8T_PROFILE(ch, y)     ((uint16_t)(PL8T_CH_BASE(ch) + 0x400u + 0x10u * (uint16_t)(y)))
#define PL8T_PRF_DPH_L          0u
#define PL8T_PRF_DPH_M          1u
#define PL8T_PRF_DPH_H          2u
#define PL8T_PRF_RESERVED       3u
#define PL8T_PRF_P              4u
#define PL8T_PRF_MUL            5u
#define PL8T_PRF_OFFSET         6u
#define PL8T_PRF_WORDS          7u   /**< длина блока для записи командами WRI */

/* ---- Поля CTR (табл. 5.3) ---- */
#define PL8T_CTR_CMP_ON         (1u << 14)
#define PL8T_CTR_DAC2_ON        (1u << 13)
#define PL8T_CTR_DAC1_ON        (1u << 12)
#define PL8T_CTR_CMX(div)       ((uint16_t)((div) & 0x1Fu))

/* ---- Поля SYNC (табл. 5.4) ---- */
#define PL8T_SYNC_CSYNC_OE      (1u << 14)
#define PL8T_SYNC_CSYNC_IE      (1u << 13)
#define PL8T_SYNC_CSYNC_DIV     (1u << 11)
#define PL8T_SYNC_SPI_MASTER    (1u << 10)
#define PL8T_SYNC_SEL_IE        (1u << 8)
#define PL8T_SYNC_SEL_OE        (1u << 7)

/* ---- Поля SEL_REG (табл. 5.2) ---- */
#define PL8T_SEL_PR1(y)         ((uint16_t)((y) & 0x3Fu))         /**< профиль канала 1 */
#define PL8T_SEL_PR2(y)         ((uint16_t)(((y) & 0x3Fu) << 8))  /**< профиль канала 2 */

/* ---- Поля ROUTE (табл. 5.7) ---- */
#define PL8T_ROUTE_PDITH2       (1u << 7)
#define PL8T_ROUTE_PDITH1       (1u << 6)
#define PL8T_ROUTE_SUM_OWN      0u   /**< каждый канал на свой ЦАП */

/* ---- Поля профиля ---- */
/** CHx_Muly[15:3] = mul (13 бит, доп. код), A = mul / 2^12 (табл. 5.13). */
#define PL8T_MUL(mul)           ((uint16_t)(((uint16_t)(mul) & 0x1FFFu) << 3))
/** CHx_Offsety[15:4] = offset (12 бит, доп. код) (табл. 5.14). */
#define PL8T_OFFSET(off)        ((uint16_t)(((uint16_t)(off) & 0x0FFFu) << 4))

/** Разрядность приращения фазы: Fout = dPh · Fclk / 2^48 (разд. 3.1). */
#define PL8T_FTW_BITS           48u
#define PL8T_FTW_MASK           ((1ULL << PL8T_FTW_BITS) - 1ULL)

#endif /* PL8T_REGS_H */
