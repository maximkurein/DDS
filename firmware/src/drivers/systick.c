/**
 * @file systick.c
 * @brief Timer0 ATmega128, CTC, прерывание раз в 1 мс.
 */
#include "systick.h"

#include <avr/interrupt.h>
#include <avr/io.h>
#include <util/atomic.h>

/* Timer0 ATmega128: CS02:0 = 100 → clk/64 (у Timer0 своя таблица делителей). */
#define SYSTICK_PRESCALER  64UL
#define SYSTICK_OCR        ((F_CPU / SYSTICK_PRESCALER / 1000UL) - 1UL)

_Static_assert(SYSTICK_OCR > 0 && SYSTICK_OCR <= 255, "Timer0 OCR0 out of range for F_CPU");
_Static_assert((F_CPU % (SYSTICK_PRESCALER * 1000UL)) == 0, "1 ms tick is not exact for this F_CPU");

static volatile uint32_t s_ms;
static volatile uint8_t  s_ticks;

ISR(TIMER0_COMP_vect)
{
    s_ms++;
    if (s_ticks != 0xFFu) {
        s_ticks++;
    }
}

void systick_init(void)
{
    ASSR  &= (uint8_t)~_BV(AS0);               /* такт от системной частоты, не от TOSC */
    TCNT0  = 0;
    OCR0   = (uint8_t)SYSTICK_OCR;
    TCCR0  = _BV(WGM01) | _BV(CS02);           /* CTC, clk/64 */
    TIFR   = _BV(OCF0);
    TIMSK |= _BV(OCIE0);
}

uint32_t systick_ms(void)
{
    uint32_t ms;
    ATOMIC_BLOCK(ATOMIC_RESTORESTATE) {
        ms = s_ms;
    }
    return ms;
}

uint8_t systick_take_ticks(void)
{
    uint8_t t;
    ATOMIC_BLOCK(ATOMIC_RESTORESTATE) {
        t = s_ticks;
        s_ticks = 0;
    }
    return t;
}
