/**
 * @file debounce.c
 * @brief Подавление дребезга счётчиком устойчивости.
 */
#include "debounce.h"

void debounce_init(debounce_t *d, bool initial, uint8_t threshold)
{
    d->state = initial;
    d->count = 0;
    d->threshold = (threshold == 0u) ? 1u : threshold;
}

bool debounce_update(debounce_t *d, bool raw)
{
    if (raw == d->state) {
        d->count = 0;
        return false;
    }
    if (++d->count < d->threshold) {
        return false;
    }
    d->state = raw;
    d->count = 0;
    return true;
}
