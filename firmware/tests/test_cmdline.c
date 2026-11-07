/* Разбор команд UART-протокола (src/core/cmdline.c). */
#include <string.h>

#include "core/cmdline.h"
#include "unit.h"

static cmd_t feed(const char *s)
{
    cmdline_t cl;
    cmd_t c = { CMD_EMPTY, 0, 0, 0 };
    bool got = false;
    cmdline_init(&cl);
    for (; *s; s++) {
        cmd_t t;
        if (cmdline_feed(&cl, *s, &t)) {
            c = t;
            got = true;
        }
    }
    if (!got) {
        c.kind = CMD_EMPTY;
    }
    return c;
}

void test_cmdline(void)
{
    cmd_t c;
    CHECK(feed("?\r\n").kind == CMD_IDENT);
    CHECK(feed("\r\n\r\n").kind == CMD_EMPTY);

    c = feed("remote 1\n");
    CHECK(c.kind == CMD_REMOTE && c.a == 1u);
    CHECK(feed("REMOTE 2\n").kind == CMD_ERR_ARG);
    CHECK(feed("REMOTE\n").kind == CMD_ERR_ARG);

    c = feed("W 0402 1a2B\r");
    CHECK(c.kind == CMD_WRITE && c.a == 0x0402u && c.b == 0x1A2Bu);
    CHECK(feed("W 10000 1\n").kind == CMD_ERR_ARG);
    CHECK(feed("W 12\n").kind == CMD_ERR_ARG);
    CHECK(feed("W 12 3 4\n").kind == CMD_ERR_ARG);
    CHECK(feed("W 1G 3\n").kind == CMD_ERR_ARG);

    c = feed("  r   0002  \n");
    CHECK(c.kind == CMD_READ && c.a == 2u);

    c = feed("S B00000\n");
    CHECK(c.kind == CMD_SPI && c.x == 0xB00000u);
    CHECK(feed("S 1000000\n").kind == CMD_ERR_ARG);

    CHECK(feed("RST\n").kind == CMD_RESET);
    CHECK(feed("RST 1\n").kind == CMD_ERR_ARG);
    c = feed("PIN RSTn 0\n");
    CHECK(c.kind == CMD_PIN && c.a == 0u);
    CHECK(feed("PIN CS 0\n").kind == CMD_ERR_ARG);
    CHECK(feed("ST\n").kind == CMD_STATUS);

    c = feed("HOP 8 25\n");
    CHECK(c.kind == CMD_HOP && c.a == 8u && c.x == 25u);
    c = feed("HOP 0\n");
    CHECK(c.kind == CMD_HOP && c.a == 0u);
    CHECK(feed("HOP 8\n").kind == CMD_ERR_ARG);
    CHECK(feed("HOP 8 0\n").kind == CMD_ERR_ARG);
    CHECK(feed("HOP 65 10\n").kind == CMD_ERR_ARG);
    CHECK(feed("HOP 4 60001\n").kind == CMD_ERR_ARG);

    CHECK(feed("WR 1 2\n").kind == CMD_UNKNOWN);
    CHECK(feed("FOO\n").kind == CMD_UNKNOWN);

    /* Слишком длинная строка: одна ошибка, следующая команда разбирается. */
    char longline[80];
    memset(longline, 'A', sizeof longline - 2u);
    longline[sizeof longline - 2u] = '\n';
    longline[sizeof longline - 1u] = '\0';
    CHECK(feed(longline).kind == CMD_ERR_LONG);
    {
        cmdline_t cl;
        cmd_t out;
        cmdline_init(&cl);
        for (const char *p = longline; *p; p++) (void)cmdline_feed(&cl, *p, &out);
        CHECK(out.kind == CMD_ERR_LONG);
        const char *next = "ST\n";
        bool got = false;
        for (; *next; next++) got = cmdline_feed(&cl, *next, &out) || got;
        CHECK(got && out.kind == CMD_STATUS);
    }

    char buf[12];
    CHECK(cmd_fmt_hex(buf, 0x2Au, 4) == 4u && strcmp(buf, "002A") == 0);
    CHECK(cmd_fmt_dec(buf, 0u) == 1u && strcmp(buf, "0") == 0);
    CHECK(cmd_fmt_dec(buf, 4294967295u) == 10u && strcmp(buf, "4294967295") == 0);
}
