/**
 * @file unit.h
 * @brief Минимальный каркас unit-тестов без внешних зависимостей.
 */
#ifndef TESTS_UNIT_H
#define TESTS_UNIT_H

#include <stdio.h>

extern int g_unit_failures;
extern int g_unit_checks;

#define CHECK(cond) do {                                                     \
        g_unit_checks++;                                                     \
        if (!(cond)) {                                                       \
            g_unit_failures++;                                               \
            printf("  FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);         \
        }                                                                    \
    } while (0)

#define CHECK_EQ_U64(actual, expected) do {                                  \
        unsigned long long a_ = (unsigned long long)(actual);                \
        unsigned long long e_ = (unsigned long long)(expected);              \
        g_unit_checks++;                                                     \
        if (a_ != e_) {                                                      \
            g_unit_failures++;                                               \
            printf("  FAIL %s:%d: %s = 0x%llX, expected 0x%llX\n",           \
                   __FILE__, __LINE__, #actual, a_, e_);                     \
        }                                                                    \
    } while (0)

void test_freq(void);
void test_dip(void);
void test_debounce(void);
void test_cmdline(void);

#endif /* TESTS_UNIT_H */
