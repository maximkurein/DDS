/**
 * @file remote.c
 * @brief Выполнение команд UART-протокола (см. core/cmdline.h).
 */
#include "remote.h"

#include <stdbool.h>
#include <string.h>
#include <util/delay.h>

#include "app.h"
#include "core/dip_code.h"
#include "dds_config.h"
#include "drivers/board.h"
#include "drivers/pl8t.h"
#include "drivers/pl8t_regs.h"
#include "drivers/systick.h"
#include "fw_version.h"

#if DDS_REMOTE && !defined(REMOTE_NO_UART)
#include "drivers/uart.h"
#endif

static cmdline_t s_line;
static struct {
    uint8_t  n;        /**< число профилей, 0 — перебор выключен */
    uint8_t  k;        /**< текущий профиль */
    uint16_t sel_hi;   /**< SEL_REG[15:8]: профиль канала 2 сохраняется */
    uint32_t ms;
    uint32_t t;
} s_hop;
static uint8_t s_err;  /**< последний код pl8t_status_t */

void remote_init(void)
{
    cmdline_init(&s_line);
    memset(&s_hop, 0, sizeof s_hop);
    s_err = 0;
}

static char *put(char *p, const char *s)
{
    while (*s != '\0') {
        *p++ = *s++;
    }
    *p = '\0';
    return p;
}

static char *put_kv(char *p, const char *key, uint32_t v)
{
    p = put(p, key);
    p += cmd_fmt_dec(p, v);
    return p;
}

/** Ответ на ошибку драйвера DDS. */
static void reply_status(pl8t_status_t st, char *reply)
{
    s_err = (uint8_t)st;
    switch (st) {
    case PL8T_OK:        put(reply, "OK"); break;
    case PL8T_ERR_POWER: put(reply, "ERR POWER"); break;
    case PL8T_ERR_SPI:   put(reply, "ERR SPI"); break;
    default:             put(reply, "ERR STATE"); break;
    }
}

/** Команды, которые обращаются к DDS: только в REMOTE и при работающей шине. */
static bool bus_ready(char *reply)
{
    if (!app_remote()) {
        put(reply, "ERR LOCAL");
        return false;
    }
    if (!board_pgood_raw()) {
        put(reply, "ERR POWER");
        return false;
    }
    if (app_state() != APP_ST_RUN) {
        put(reply, "ERR STATE");
        return false;
    }
    return true;
}

void remote_execute(const cmd_t *cmd, char *reply)
{
    reply[0] = '\0';
    switch (cmd->kind) {
    case CMD_EMPTY:
        break;
    case CMD_IDENT:
        put(reply, "PL8T " DDS_FW_VERSION " DDS48-52 uart1");
        break;
    case CMD_REMOTE:
        s_hop.n = 0;
        app_set_remote(cmd->a != 0u);
        put(reply, "OK");
        break;
    case CMD_WRITE:
        if (bus_ready(reply)) {
            reply_status(pl8t_write(cmd->a, cmd->b), reply);
        }
        break;
    case CMD_READ:
        if (bus_ready(reply)) {
            uint16_t v = 0;
            const pl8t_status_t st = pl8t_read(cmd->a, &v);
            reply_status(st, reply);
            if (st == PL8T_OK) {
                char *p = put(reply, "OK ");
                cmd_fmt_hex(p, v, 4);
            }
        }
        break;
    case CMD_SPI:
        if (bus_ready(reply)) {
            uint16_t v = 0;
            const pl8t_status_t st = pl8t_frame((uint8_t)(cmd->x >> 16), (uint16_t)cmd->x, &v);
            reply_status(st, reply);
            if (st == PL8T_OK) {
                char *p = put(reply, "OK 00");
                cmd_fmt_hex(p, v, 4);
            }
        }
        break;
    case CMD_RESET:
        if (bus_ready(reply)) {
            s_hop.n = 0;
            board_dds_reset(true);
            _delay_ms(DDS_RESET_PULSE_MS);
            board_dds_reset(false);
            _delay_ms(DDS_RESET_RECOVERY_MS);
            put(reply, "OK");
        }
        break;
    case CMD_PIN:
        if (bus_ready(reply)) {
            s_hop.n = 0;
            board_dds_reset(cmd->a == 0u);
            put(reply, "OK");
        }
        break;
    case CMD_STATUS: {
        char *p = put(reply, "OK");
        p = put_kv(p, " pgood=", board_pgood_raw() ? 1u : 0u);
        p = put_kv(p, " apply=", board_apply_pressed_raw() ? 1u : 0u);
        p = put_kv(p, " dip=", dip_decode(board_dip_lo_raw(), board_dip_hi_raw()));
        p = put_kv(p, " remote=", app_remote() ? 1u : 0u);
        p = put_kv(p, " err=", s_err);
        p = put_kv(p, " hop=", s_hop.n);
        put_kv(p, " ms=", s_hop.ms);
        break;
    }
    case CMD_HOP:
        if (cmd->a < 2u) {
            s_hop.n = 0;
            put(reply, app_remote() ? "OK" : "ERR LOCAL");
        } else if (bus_ready(reply)) {
            uint16_t sel = 0;
            const pl8t_status_t st = pl8t_read(PL8T_SEL_REG, &sel);
            reply_status(st, reply);
            if (st == PL8T_OK) {
                s_hop.n = (uint8_t)cmd->a;
                s_hop.k = 0;
                s_hop.sel_hi = (uint16_t)(sel & 0x3F00u);
                s_hop.ms = cmd->x;
                s_hop.t = systick_ms() - cmd->x;  /* первый шаг сразу */
            }
        }
        break;
    case CMD_ERR_ARG:
        put(reply, "ERR ARG");
        break;
    case CMD_ERR_LONG:
        put(reply, "ERR LONG");
        break;
    default:
        put(reply, "ERR CMD");
        break;
    }
}

void remote_hop_tick(void)
{
    if (s_hop.n < 2u || !app_remote() || app_state() != APP_ST_RUN) {
        s_hop.n = app_remote() ? s_hop.n : 0u;
        return;
    }
    if (!systick_elapsed(s_hop.t, s_hop.ms)) {
        return;
    }
    s_hop.t += s_hop.ms;
    if (systick_elapsed(s_hop.t, s_hop.ms)) {
        s_hop.t = systick_ms();  /* отстали (блокирующие операции): без догоняющих шагов */
    }
    const pl8t_status_t st = pl8t_write(PL8T_SEL_REG, (uint16_t)(s_hop.sel_hi | PL8T_SEL_PR1(s_hop.k)));
    if (st != PL8T_OK) {
        s_err = (uint8_t)st;
        s_hop.n = 0;
        return;
    }
    s_hop.k = (uint8_t)((s_hop.k + 1u) % s_hop.n);
}

#if DDS_REMOTE && !defined(REMOTE_NO_UART)
void remote_poll(void)
{
    char c;
    cmd_t cmd;
    char reply[64];
    if (uart_take_overrun()) {
        cmdline_init(&s_line);
        uart_puts_P(PSTR("ERR LONG\r\n"));
    }
    while (uart_getc(&c)) {
        if (cmdline_feed(&s_line, c, &cmd)) {
            remote_execute(&cmd, reply);
            if (reply[0] != '\0') {
                uart_puts(reply);
                uart_puts_P(PSTR("\r\n"));
            }
        }
    }
    remote_hop_tick();
}
#endif
