/** Декодирование DIP: ON = вывод на земле = бит 1. */
#include <stdint.h>

#include "core/dip_code.h"
#include "unit.h"

void test_dip(void)
{
    /* Все выключены: на всех выводах 1 (подтяжки), код 0. */
    CHECK(dip_decode(0xFFu, 0xFFu) == 0u);
    /* Все включены. */
    CHECK(dip_decode(0x00u, 0x00u) == 0x0FFFu);
    /* Только S2.1 (PA0) → бит 0. */
    CHECK(dip_decode(0xFEu, 0xFFu) == 0x0001u);
    /* Только S3.4 (PC3) → бит 11. */
    CHECK(dip_decode(0xFFu, 0xF7u) == 0x0800u);
    /* PC4…PC7 (PGOOD, кнопка, свободные) не влияют на код. */
    CHECK(dip_decode(0xFFu, 0x0Fu) == 0u);
    CHECK(dip_decode(0xFFu, 0xEFu) == 0u);
    /* N = 4000 = 0xFA0: S3 = 1111, S2 = 1010 0000. */
    CHECK(dip_decode((uint8_t)~0xA0u, (uint8_t)~0x0Fu) == 4000u);
    /* N = 1234 = 0x4D2. */
    CHECK(dip_decode((uint8_t)~0xD2u, (uint8_t)~0x04u) == 1234u);
}
