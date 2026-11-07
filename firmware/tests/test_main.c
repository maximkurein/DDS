#include "unit.h"

int g_unit_failures = 0;
int g_unit_checks = 0;

int main(void)
{
    printf("freq\n");
    test_freq();
    printf("dip\n");
    test_dip();
    printf("debounce\n");
    test_debounce();
    printf("cmdline\n");
    test_cmdline();

    printf("%d checks, %d failures\n", g_unit_checks, g_unit_failures);
    return g_unit_failures == 0 ? 0 : 1;
}
