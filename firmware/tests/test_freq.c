/**
 * Проверка расчёта приращения фазы.
 * Эталоны посчитаны независимо (Python, целочисленно):
 *   ftw = round((48e6 + 1000·N) · 2^48 / 480e6), половина округляется вверх.
 */
#include <stdint.h>

#include "core/freq.h"
#include "unit.h"

#define FCLK 480000000UL

static const freq_scale_t k_scale = { 48000000UL, 1000UL, 4000U };

/* Второй, независимый способ для этого такта: f и Fclk кратны 1000,
 * поэтому f_k · 2^48 при f_k ≤ 52000 помещается в uint64_t. */
static uint64_t ftw_by_khz(uint32_t f_hz)
{
    const uint64_t fk = f_hz / 1000u;
    const uint64_t ck = FCLK / 1000u;
    return (fk * (1ULL << 48) + ck / 2u) / ck;
}

void test_freq(void)
{
    static const struct { uint16_t n; uint64_t ftw; } ref[] = {
        {    0, 0x19999999999AULL },
        {    1, 0x1999BC8D72D3ULL },
        {    2, 0x1999DF814C0DULL },
        {  999, 0x1A21FF2E48E9ULL },
        { 1234, 0x1A421506B0ADULL },
        { 2000, 0x1AAAAAAAAAABULL },
        { 3999, 0x1BBB98C7E282ULL },
        { 4000, 0x1BBBBBBBBBBCULL },
    };

    for (unsigned i = 0; i < sizeof ref / sizeof ref[0]; i++) {
        uint32_t f = 0;
        CHECK(freq_code_to_hz(&k_scale, ref[i].n, &f));
        CHECK_EQ_U64(freq_hz_to_ftw(f, FCLK), ref[i].ftw);
    }

    /* Весь диапазон N: совпадение со вторым способом и монотонность. */
    uint64_t prev = 0;
    for (uint16_t n = 0; n <= 4000u; n++) {
        uint32_t f = 0;
        CHECK(freq_code_to_hz(&k_scale, n, &f));
        const uint64_t ftw = freq_hz_to_ftw(f, FCLK);
        if (ftw != ftw_by_khz(f)) {
            CHECK_EQ_U64(ftw, ftw_by_khz(f));
            break;
        }
        if (n > 0u && ftw <= prev) {
            CHECK(ftw > prev);
            break;
        }
        prev = ftw;
    }

    /* Границы диапазона кода. */
    uint32_t f = 123u;
    CHECK(freq_code_to_hz(&k_scale, 4000u, &f) && f == 52000000UL);
    CHECK(!freq_code_to_hz(&k_scale, 4001u, &f));
    CHECK(f == 52000000UL);  /* при отказе значение не меняется */
    CHECK(!freq_code_to_hz(&k_scale, 4095u, &f));

    /* Простые случаи: Fclk/4 и Fclk/32 (пример из Appnote: 25 МГц при 800 МГц). */
    CHECK_EQ_U64(freq_hz_to_ftw(120000000UL, FCLK), 1ULL << 46);
    CHECK_EQ_U64(freq_hz_to_ftw(25000000UL, 800000000UL), 0x080000000000ULL);
    CHECK_EQ_U64(freq_hz_to_ftw(0, FCLK), 0);

    /* Разбивка на регистры dPh_L/M/H. */
    uint16_t r[3];
    freq_ftw_split(0x1A421506B0ADULL, r);
    CHECK(r[0] == 0xB0ADu && r[1] == 0x1506u && r[2] == 0x1A42u);
}
