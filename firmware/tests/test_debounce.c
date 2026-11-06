/** Подавление дребезга: смена состояния только после threshold одинаковых отсчётов. */
#include <stdbool.h>

#include "core/debounce.h"
#include "unit.h"

void test_debounce(void)
{
    debounce_t d;
    debounce_init(&d, false, 20u);

    /* Дребезг 0-1-0-1…-0 короче порога — состояние не меняется.
     * Последний отсчёт 0 сбрасывает счётчик. */
    for (int i = 0; i <= 100; i++) {
        CHECK(!debounce_update(&d, (i & 1) != 0));
    }
    CHECK(d.state == false);

    /* 19 отсчётов «нажато» — ещё нет события, 20-й — событие. */
    for (int i = 0; i < 19; i++) {
        CHECK(!debounce_update(&d, true));
    }
    CHECK(debounce_update(&d, true));
    CHECK(d.state == true);

    /* Удержание — событий больше нет. */
    for (int i = 0; i < 1000; i++) {
        CHECK(!debounce_update(&d, true));
    }

    /* Короткий провал 10 мс при удержании — не отпускание. */
    for (int i = 0; i < 10; i++) {
        CHECK(!debounce_update(&d, false));
    }
    CHECK(!debounce_update(&d, true));
    CHECK(d.state == true);

    /* Отпускание. */
    for (int i = 0; i < 19; i++) {
        CHECK(!debounce_update(&d, false));
    }
    CHECK(debounce_update(&d, false));
    CHECK(d.state == false);

    /* Порог 0 приводится к 1. */
    debounce_init(&d, false, 0u);
    CHECK(debounce_update(&d, true));
}
