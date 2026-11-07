/**
 * @file cmdline.c
 * @brief Разбор команд UART-протокола (см. cmdline.h).
 */
#include "cmdline.h"

void cmdline_init(cmdline_t *cl)
{
    cl->len = 0;
    cl->overflow = false;
    cl->buf[0] = '\0';
}

bool cmdline_feed(cmdline_t *cl, char c, cmd_t *out)
{
    if (c == '\r' || c == '\n') {
        cl->buf[cl->len] = '\0';
        if (cl->overflow) {
            out->kind = CMD_ERR_LONG;
            out->a = out->b = 0;
            out->x = 0;
        } else {
            *out = cmd_parse(cl->buf);
        }
        const bool ready = cl->overflow || out->kind != CMD_EMPTY;
        cmdline_init(cl);
        return ready;
    }
    if (cl->len >= CMDLINE_MAX) {
        cl->overflow = true;
        return false;
    }
    cl->buf[cl->len++] = c;
    return false;
}

static char upper(char c)
{
    return (c >= 'a' && c <= 'z') ? (char)(c - 'a' + 'A') : c;
}

static const char *skip_ws(const char *p)
{
    while (*p == ' ' || *p == '\t') {
        p++;
    }
    return p;
}

/** Сравнить слово в начале p с ключом (без учёта регистра); вернуть указатель за словом. */
static const char *word(const char *p, const char *key)
{
    p = skip_ws(p);
    while (*key != '\0') {
        if (upper(*p) != *key) {
            return 0;
        }
        p++;
        key++;
    }
    if (*p != '\0' && *p != ' ' && *p != '\t') {
        return 0;
    }
    return p;
}

/** Беззнаковое число (base 10 или 16) не больше max. */
static const char *number(const char *p, uint8_t base, uint32_t max, uint32_t *v)
{
    p = skip_ws(p);
    uint32_t acc = 0;
    uint8_t n = 0;
    for (;; p++, n++) {
        const char c = upper(*p);
        uint8_t d;
        if (c >= '0' && c <= '9') {
            d = (uint8_t)(c - '0');
        } else if (base == 16u && c >= 'A' && c <= 'F') {
            d = (uint8_t)(c - 'A' + 10);
        } else {
            break;
        }
        if (d > max || acc > (max - d) / base) {
            return 0;  /* переполнение */
        }
        acc = acc * base + d;
    }
    if (n == 0u || (*p != '\0' && *p != ' ' && *p != '\t')) {
        return 0;
    }
    *v = acc;
    return p;
}

static bool end(const char *p)
{
    return p != 0 && *skip_ws(p) == '\0';
}

cmd_t cmd_parse(const char *line)
{
    cmd_t c = { CMD_UNKNOWN, 0, 0, 0 };
    const char *p;
    uint32_t v1 = 0, v2 = 0;

    if (*skip_ws(line) == '\0') {
        c.kind = CMD_EMPTY;
        return c;
    }
    if ((p = word(line, "?")) != 0 && end(p)) {
        c.kind = CMD_IDENT;
    } else if ((p = word(line, "REMOTE")) != 0) {
        c.kind = CMD_ERR_ARG;
        if ((p = number(p, 10, 1, &v1)) != 0 && end(p)) {
            c.kind = CMD_REMOTE;
            c.a = (uint16_t)v1;
        }
    } else if ((p = word(line, "W")) != 0) {
        c.kind = CMD_ERR_ARG;
        if ((p = number(p, 16, 0xFFFFu, &v1)) != 0 && (p = number(p, 16, 0xFFFFu, &v2)) != 0 && end(p)) {
            c.kind = CMD_WRITE;
            c.a = (uint16_t)v1;
            c.b = (uint16_t)v2;
        }
    } else if ((p = word(line, "R")) != 0) {
        c.kind = CMD_ERR_ARG;
        if ((p = number(p, 16, 0xFFFFu, &v1)) != 0 && end(p)) {
            c.kind = CMD_READ;
            c.a = (uint16_t)v1;
        }
    } else if ((p = word(line, "S")) != 0) {
        c.kind = CMD_ERR_ARG;
        if ((p = number(p, 16, 0xFFFFFFu, &v1)) != 0 && end(p)) {
            c.kind = CMD_SPI;
            c.x = v1;
        }
    } else if ((p = word(line, "RST")) != 0) {
        c.kind = end(p) ? CMD_RESET : CMD_ERR_ARG;
    } else if ((p = word(line, "PIN")) != 0) {
        c.kind = CMD_ERR_ARG;
        if ((p = word(p, "RSTN")) != 0 && (p = number(p, 10, 1, &v1)) != 0 && end(p)) {
            c.kind = CMD_PIN;
            c.a = (uint16_t)v1;
        }
    } else if ((p = word(line, "ST")) != 0) {
        c.kind = end(p) ? CMD_STATUS : CMD_ERR_ARG;
    } else if ((p = word(line, "HOP")) != 0) {
        c.kind = CMD_ERR_ARG;
        if ((p = number(p, 10, 64, &v1)) != 0) {
            v2 = 0;
            const char *q = number(p, 10, 60000u, &v2);
            if ((q != 0 && end(q) && (v1 < 2u || v2 >= 1u)) || (end(p) && v1 < 2u)) {
                c.kind = CMD_HOP;
                c.a = (uint16_t)v1;
                c.x = v2;
            }
        }
    }
    return c;
}

uint8_t cmd_fmt_hex(char *dst, uint32_t v, uint8_t digits)
{
    static const char hex[] = "0123456789ABCDEF";
    for (uint8_t i = 0; i < digits; i++) {
        dst[digits - 1u - i] = hex[v & 0xFu];
        v >>= 4;
    }
    dst[digits] = '\0';
    return digits;
}

uint8_t cmd_fmt_dec(char *dst, uint32_t v)
{
    char tmp[10];
    uint8_t n = 0;
    do {
        tmp[n++] = (char)('0' + (v % 10u));
        v /= 10u;
    } while (v != 0u);
    for (uint8_t i = 0; i < n; i++) {
        dst[i] = tmp[n - 1u - i];
    }
    dst[n] = '\0';
    return n;
}
