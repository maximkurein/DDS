import { useState } from "react";
import { Grid3x3, Play, Shuffle, Square } from "lucide-react";
import { useBoard, useStore } from "../lib/store";
import { fmtEng } from "../lib/format";
import { configKey, useRpcData } from "../lib/useRpc";
import type { Profile } from "../lib/types";
import { Badge, Button, Card, EngField, Hint, NumField, PageHeader, Row, Stat, cx } from "../components/ui";

export function HopView() {
  const snap = useStore((s) => s.snap)!;
  const meta = useStore((s) => s.meta)!;
  const call = useStore((s) => s.call);
  const status = useStore((s) => s.status);
  const board = useBoard();
  const nProf = meta.chip.n_prof[0];
  const { data: rows } = useRpcData<Profile[]>("profiles", { ch: 1 }, configKey(snap.regs, snap.settings), 30);
  const pr = snap.fields["Pr_1"] ?? 0;
  const live = status?.live_profile ?? pr;
  const hopping = (board?.hop_n ?? 0) >= 2;
  const [n, setN] = useState(8);
  const [ms, setMs] = useState(250);
  const [gLo, setGLo] = useState(48.5e6);
  const [gHi, setGHi] = useState(51.5e6);
  const [busy, setBusy] = useState(false);

  const fillGrid = async () => {
    setBusy(true);
    for (let y = 0; y < n; y++) {
      const f = n > 1 ? gLo + ((gHi - gLo) * y) / (n - 1) : gLo;
      if ((await call("set_profile", { ch: 1, y, f })) == null) break;
    }
    setBusy(false);
  };

  return (
    <div className="max-w-[1500px]">
      <PageHeader
        icon={<Shuffle size={20} />}
        title="Профили и перебор частот"
        subtitle={`Канал 1, профили 0…${nProf - 1} (в кристалле их 64). Выбор профиля — одна запись SEL_REG: частота, фаза и амплитуда меняются за такт. Перебор ведёт прошивка платы (команда HOP), ПК только задаёт профили.`}
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <Card title="Профили канала 1" subtitle="Щелчок по номеру — сделать профиль активным. Поля применяются по Enter.">
          <div className="overflow-x-auto">
            <table className="w-full text-[12px]">
              <thead className="text-muted text-[11px] text-left">
                <tr>
                  <th className="py-1 font-medium w-12">#</th>
                  <th className="py-1 font-medium">Частота</th>
                  <th className="py-1 font-medium">Фаза, °</th>
                  <th className="py-1 font-medium">Mul</th>
                  <th className="py-1 font-medium">Offset</th>
                  <th className="py-1 font-medium">dPh</th>
                </tr>
              </thead>
              <tbody>
                {rows?.map((r) => (
                  <tr key={r.y} className={cx("border-t border-line/60", r.y === live && "bg-accent/8")}>
                    <td className="py-1">
                      <button
                        onClick={() => call("select_profile", { ch: 1, y: r.y })}
                        className={cx("h-6 w-8 rounded-md num text-[12px] border", r.y === live ? "bg-accent text-accentfg border-accent" : r.y === pr ? "border-accent text-accent" : "border-line hover:border-line2")}
                        title={`Pr_1 = ${r.y}`}
                      >
                        {r.y}
                      </button>
                    </td>
                    <td className="py-1">
                      <EngField value={r.f} unit="Hz" width="w-40" min={0} max={snap.derived.calc.fclk / 2} onCommit={(f) => call("set_profile", { ch: 1, y: r.y, f })} />
                    </td>
                    <td className="py-1">
                      <NumField value={Math.round(r.phase_deg * 100) / 100} integer={false} min={-360} max={360} step={45} width="w-16" onCommit={(p) => call("set_profile", { ch: 1, y: r.y, phase_deg: p })} />
                    </td>
                    <td className="py-1">
                      <NumField value={r.mul} min={-4096} max={4095} step={64} width="w-14" onCommit={(m) => call("set_profile", { ch: 1, y: r.y, mul: m })} />
                    </td>
                    <td className="py-1">
                      <NumField value={r.offset} min={-2048} max={2047} width="w-14" onCommit={(o) => call("set_profile", { ch: 1, y: r.y, offset: o })} />
                    </td>
                    <td className="py-1 font-mono text-[11px] text-muted">{r.ftw.toString(16).toUpperCase().padStart(12, "0")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>

        <div className="flex flex-col gap-4">
          <Card title="Перебор прошивкой" icon={<Shuffle size={15} />} subtitle="Профили 0…n−1 по кругу, шаг задаёт таймер прошивки">
            <Row label="Профилей n">
              <NumField value={n} min={2} max={nProf} onCommit={setN} />
            </Row>
            <Row label="На профиле">
              <NumField value={ms} min={1} max={60000} step={10} unit="мс" onCommit={setMs} />
            </Row>
            <div className="flex gap-2 mt-2">
              <Button variant="primary" icon={<Play size={13} />} onClick={() => call("hop", { n, ms })}>
                Запустить
              </Button>
              <Button icon={<Square size={13} />} onClick={() => call("hop", { n: 0 })} disabled={!hopping}>
                Остановить
              </Button>
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2">
              <Stat label="Сейчас играет" value={`профиль ${live}`} tone="sig" sub={rows?.[live] ? fmtEng(rows[live].f, "Hz", 7) : undefined} />
              <Stat label="Перебор" value={hopping ? `${board?.hop_n} × ${board?.hop_ms} мс` : "выкл."} />
            </div>
            {hopping && <Badge tone="ok">прошивка переключает SEL_REG.Pr_1</Badge>}
            <Hint>Остановка возвращает профиль, выбранный в приложении (Pr_1 = {pr}). Перебор не мешает менять неактивные профили — так строится «прыгающая» частота с новой сеткой на лету.</Hint>
          </Card>

          <Card title="Заполнить сеткой" icon={<Grid3x3 size={15} />} subtitle={`Профили 0…${n - 1} равномерно от и до`}>
            <Row label="От">
              <EngField value={gLo} unit="Hz" onCommit={setGLo} min={0} max={240e6} />
            </Row>
            <Row label="До">
              <EngField value={gHi} unit="Hz" onCommit={setGHi} min={0} max={240e6} />
            </Row>
            <Button className="mt-2" onClick={fillGrid} disabled={busy}>
              {busy ? "Запись…" : `Записать ${n} частот`}
            </Button>
          </Card>
        </div>
      </div>
    </div>
  );
}
