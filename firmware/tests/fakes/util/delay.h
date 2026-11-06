#ifndef TEST_UTIL_DELAY_H
#define TEST_UTIL_DELAY_H
void test_delay_ms(double ms);
#define _delay_ms(ms) test_delay_ms(ms)
#endif
