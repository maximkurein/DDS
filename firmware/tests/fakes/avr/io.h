/* Host-only register stand-ins; selected by the regression test include path. */
#ifndef TEST_AVR_IO_H
#define TEST_AVR_IO_H
#include <stdint.h>
#define _BV(bit) (1u << (bit))
enum { PB0, PB1, PB2, PB3, PB4, PB5, PB6, PB7 };
enum { PC0, PC1, PC2, PC3, PC4, PC5, PC6, PC7 };
extern uint8_t PORTA, DDRA, PINA, PORTB, DDRB, PINB, PORTC, DDRC, PINC;
extern uint8_t PORTD, DDRD, PORTE, DDRE, PORTF, DDRF, PORTG, DDRG, OSCCAL;
#endif
