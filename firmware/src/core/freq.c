/**
 * @file freq.c
 * @brief Расчёт частоты и приращения фазы DDS.
 */
#include "freq.h"

#define FTW_BITS 48u

bool freq_code_to_hz(const freq_scale_t *scale, uint16_t n, uint32_t *f_hz)
{
    if (n > scale->n_max) {
        return false;
    }
    *f_hz = scale->base_hz + scale->step_hz * (uint32_t)n;
    return true;
}

uint64_t freq_hz_to_ftw(uint32_t f_hz, uint32_t fclk_hz)
{
    /*
     * Ищем q = floor(f · 2^48 / fclk) по одному биту, начиная со старшего.
     * Остаток r всегда < fclk < 2^31, поэтому 2r помещается в uint32_t.
     */
    uint64_t q = 0;
    uint32_t r = f_hz;

    for (uint8_t i = 0; i < FTW_BITS; i++) {
        r <<= 1;
        q <<= 1;
        if (r >= fclk_hz) {
            r -= fclk_hz;
            q |= 1u;
        }
    }
    /* Округление к ближайшему: дробная часть = r / fclk. */
    if (r >= fclk_hz - r) {
        q++;
    }
    return q;
}

void freq_ftw_split(uint64_t ftw, uint16_t out[3])
{
    out[0] = (uint16_t)(ftw & 0xFFFFu);          /* [15:0]  → dPh_L */
    out[1] = (uint16_t)((ftw >> 16) & 0xFFFFu);  /* [31:16] → dPh_M */
    out[2] = (uint16_t)((ftw >> 32) & 0xFFFFu);  /* [47:32] → dPh_H */
}
