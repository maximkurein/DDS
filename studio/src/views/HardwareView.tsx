import { useEffect, useState, type ReactNode } from "react";
import { Cable, Cpu, HardDrive, PlugZap, RefreshCw, RotateCcw, ShieldAlert, Stethoscope, Undo2, Usb } from "lucide-react";
import { useBoard, useStore } from "../lib/store";
import { SAFETY, WIRING } from "../lib/board";
import type { PortInfo, UsbDev } from "../lib/types";
import { Badge, Button, Card, Hint, Led, NumField, PageHeader, Row, Select, Toggle, cx } from "../components/ui";

const hex4 = (v: number | null) => (v == null ? "—" : v.toString(16).toUpperCase().padStart(4, "0"));

function BoardStateCard() {
  const snap = useStore((s) => s.snap)!;
  const call = useStore((s) => s.call);
  const setSettings = useStore((s) => s.setSettings);
  const b = useBoard();
  const sim = snap.link.kind === "sim";
  if (!b) return null;
  const cells: [string, ReactNode][] = [
    ["Прошивка", <span className="font-mono">{b.fw || "—"}</span>],
    ["Режим", b.remote ? <Badge tone="ok">REMOTE — управляет ПК</Badge> : <Badge tone="warn">DIP — управляет плата</Badge>],
    ["+3,3 В (PGOOD)", b.pgood ? <Badge tone="ok">есть</Badge> : <Badge tone="err">нет</Badge>],
    ["Кнопка S4", b.apply_pressed ? <Badge tone="accent">нажата</Badge> : <Badge>отпущена</Badge>],
    ["DIP-код N", <span className="num">{b.dip ?? "—"}{b.dip != null && ` → ${(48 + b.dip / 1000).toFixed(3)} МГц`}</span>],
    ["Последняя ошибка", b.err ? <Badge tone="err">код {b.err}</Badge> : <Badge tone="ok">нет</Badge>],
    ["Перебор (HOP)", b.hop_n >= 2 ? <span className="num">{b.hop_n} профилей × {b.hop_ms} мс</span> : "выкл."],
  ];
  return (
    <Card title={sim ? "Виртуальная плата" : "Состояние платы"} icon={<HardDrive size={15} />} subtitle={sim ? "Модель платы вокруг кристалла: прошивка, питание, переключатели" : "Ответ прошивки на команду ST (опрос 350 мс)"}>
      <table className="w-full text-[12.5px]">
        <tbody>
          {cells.map(([k, v]) => (
            <tr key={k} className="border-t border-line/60 first:border-0">
              <td className="py-1.5 text-muted w-40">{k}</td>
              <td className="py-1.5">{v}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {sim && (
        <div className="mt-3 border-t border-line pt-2">
          <div className="text-[11px] uppercase tracking-wider text-muted mb-1">Воздействия на симулятор</div>
          <Row label="Такт на XW1" hint="без такта SPI молчит">
            <Toggle checked={snap.settings.sim_clock_ok} onChange={(v) => setSettings({ sim_clock_ok: v })} tone="ok" />
          </Row>
          <Row label="+3,3 В (PGOOD)" hint="провал питания">
            <Toggle checked={snap.settings.sim_pgood} onChange={(v) => setSettings({ sim_pgood: v })} tone="ok" />
          </Row>
          <Row label="DIP-код N" hint="S2 + S3, 0…4000">
            <NumField value={b.dip ?? 0} min={0} max={4095} onCommit={(v) => call("set_sim_dip", { value: v })} />
          </Row>
        </div>
      )}
    </Card>
  );
}

export function HardwareView() {
  const snap = useStore((s) => s.snap)!;
  const meta = useStore((s) => s.meta)!;
  const status = useStore((s) => s.status);
  const call = useStore((s) => s.call);
  const toast = useStore((s) => s.toast);
  const [ports, setPorts] = useState<PortInfo[]>([]);
  const [usb, setUsb] = useState<UsbDev[] | null>(null);
  const [port, setPort] = useState(snap.last_port ?? "");
  const [baud, setBaud] = useState(38400);
  const [connecting, setConnecting] = useState(false);
  const sim = snap.link.kind === "sim";

  const refresh = async () => {
    const p = await call<PortInfo[]>("list_ports", {}, true);
    if (p) {
      setPorts(p);
      if (!port && p.length) setPort((p.find((x) => x.kind === "usb") ?? p[0]).path);
    }
    const u = await call<UsbDev[]>("scan_usb", {}, true);
    setUsb(u ?? []);
  };
  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const connect = async () => {
    setConnecting(true);
    const r = await call("connect", { kind: "serial", path: port, baud });
    setConnecting(false);
    if (r) toast("ok", "Плата подключена, режим REMOTE");
  };
  const check = async () => {
    const r = await call<{ ok: boolean; value: number }>("check_link");
    if (r) toast(r.ok ? "ok" : "err", r.ok ? "DEVID = 0x0201: связь с DDS есть" : `DEVID = 0x${r.value.toString(16).toUpperCase()}: нет такта, питания DDS или связи SPI`);
  };

  const inputs = status?.inputs ?? snap.inputs;

  return (
    <div className="max-w-[1400px]">
      <PageHeader
        icon={<Usb size={20} />}
        title="Подключение"
        subtitle="Симулятор или настоящая плата: USB-UART адаптер на площадках X1 и прошивка D2 версии 1.1+ с командным протоколом (38400 8N1). SPI к 1508ПЛ8Т ведёт контроллер платы."
      />
      <div className="grid gap-4 lg:grid-cols-2">
        <Card title="Текущее подключение" icon={<PlugZap size={15} />}>
          <div className="flex items-center gap-3">
            <Led tone={snap.link_error ? "err" : "ok"} />
            <div className="min-w-0">
              <div className="text-[13px] font-semibold">{snap.link.name}</div>
              <div className="text-[11.5px] text-muted truncate">{snap.link_error ?? snap.link.detail}</div>
            </div>
            <div className="flex-1" />
            {!sim && (
              <Button size="sm" onClick={() => call("connect", { kind: "sim" })}>
                На симулятор
              </Button>
            )}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button size="sm" icon={<Stethoscope size={13} />} onClick={check} title="Прочитать DEVID через кадры SETAFT + NOP">
              Проверить связь
            </Button>
            <Button size="sm" icon={<RotateCcw size={13} />} onClick={() => call("chip_reset")} title="Импульс RSTn: регистры — по табл. 5.1">
              Сброс RSTn
            </Button>
            <Button size="sm" variant="ghost" onClick={() => call("chip_reset", { soft: true })} title="SWRST = 0x0078">
              SWRST
            </Button>
            {!sim && (
              <Button size="sm" variant="ghost" icon={<Undo2 size={13} />} onClick={() => call("release")} title="REMOTE 0: частоту снова задают DIP-переключатели">
                Вернуть плате
              </Button>
            )}
          </div>
          <div className="mt-4 grid grid-cols-2 gap-2">
            {meta.pins.map((p) => (
              <div key={p.name} className="flex items-center gap-2 rounded-lg border border-line bg-panel2 px-3 py-2">
                <span className="font-mono text-[12px] w-14">{p.name}</span>
                {p.dir === "out" ? (
                  <Toggle checked={snap.pins[p.name] ?? p.idle} onChange={(high) => call("set_pin", { name: p.name, high })} title={p.title} />
                ) : (
                  <Badge tone={inputs[p.name] == null ? "muted" : inputs[p.name] ? "ok" : "warn"}>{inputs[p.name] == null ? "—" : inputs[p.name] ? "1" : "0"}</Badge>
                )}
                <span className="text-[11px] text-muted truncate" title={p.title}>
                  {p.title}
                </span>
              </div>
            ))}
          </div>
        </Card>

        <BoardStateCard />

        <Card title="Плата через USB-UART" icon={<Cable size={15} />} actions={<Button size="sm" variant="ghost" icon={<RefreshCw size={13} />} onClick={refresh} title="Обновить список" />}>
          <div className="flex flex-wrap items-center gap-2">
            <Select
              className="min-w-[260px]"
              value={port}
              onChange={setPort}
              options={ports.length ? ports.map((p) => ({ value: p.path, label: `${p.path}${p.product ? ` — ${p.product}` : ""}` })) : [{ value: "", label: "порты не найдены" }]}
            />
            <Select value={baud} onChange={setBaud} options={[9600, 19200, 38400, 57600].map((b) => ({ value: b, label: `${b} бод${b === 38400 ? " (прошивка)" : ""}` }))} />
            <Button variant="primary" disabled={!port || connecting} onClick={connect}>
              {connecting ? "Подключение…" : "Подключить"}
            </Button>
          </div>
          <Hint>
            Приложение посылает <span className="kbd">?</span> и ждёт <span className="kbd">PL8T …</span>, затем включает <span className="kbd">REMOTE 1</span>: DIP-переключатели и кнопка S4 перестают действовать, регистры пишет ПК. Строки журнала отладочной прошивки пропускаются. Протокол — docs/PROTOCOL_ru.md.
          </Hint>
        </Card>

        <Card title="Схема соединений" icon={<Cpu size={15} />}>
          <table className="w-full text-[12.5px]">
            <thead className="text-muted text-[11px]">
              <tr className="text-left">
                <th className="py-1 font-medium">Адаптер</th>
                <th className="py-1 font-medium" />
                <th className="py-1 font-medium">Плата</th>
                <th className="py-1 font-medium">Сигнал D2</th>
                <th className="py-1 font-medium">Примечание</th>
              </tr>
            </thead>
            <tbody>
              {WIRING.map((w) => (
                <tr key={w.mcu} className="border-t border-line/60">
                  <td className="py-1 font-mono">{w.mcu}</td>
                  <td className="py-1 text-muted">{w.dir}</td>
                  <td className="py-1 font-mono">{w.board}</td>
                  <td className="py-1 font-mono text-accent">{w.signal}</td>
                  <td className="py-1 text-muted">{w.note ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="mt-3 flex flex-col gap-2">
            {SAFETY.slice(0, 3).map((s, i) => (
              <Hint key={i} tone="warn">
                <span className="inline-flex gap-1.5">
                  <ShieldAlert size={14} className="text-warn shrink-0 mt-0.5" />
                  {s}
                </span>
              </Hint>
            ))}
          </div>
        </Card>

        <Card title="USB-устройства" subtitle="Подсказки помогают найти USB-UART адаптер (CH340, CP210x, FT232, PL2303)" className="lg:col-span-2">
          {usb == null ? (
            <div className="text-muted text-[12px]">Сканирование…</div>
          ) : usb.length === 0 ? (
            <div className="text-muted text-[12px]">Устройства не найдены (или нет доступа к USB).</div>
          ) : (
            <div className="grid gap-1.5 md:grid-cols-2 max-h-[300px] overflow-y-auto scroll-thin">
              {usb.map((d) => (
                <div key={`${d.bus}-${d.address}`} className={cx("rounded-lg border px-3 py-1.5", d.likely_bridge ? "border-accent/40 bg-accent/5" : "border-line bg-panel2")}>
                  <div className="flex items-center gap-2 text-[12px]">
                    <span className="font-mono text-muted">
                      {hex4(d.vid)}:{hex4(d.pid)}
                    </span>
                    <span className="font-medium truncate">{d.product ?? "—"}</span>
                    <span className="text-muted truncate">{d.manufacturer ?? ""}</span>
                  </div>
                  {d.hint && <div className="text-[11px] text-muted">{d.hint}</div>}
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
