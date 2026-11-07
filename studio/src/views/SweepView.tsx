import { useEffect, useRef, useState } from "react";
import { MoveRight, Play, Square } from "lucide-react";
import { useStore } from "../lib/store";
import { axisHz, fmtEng } from "../lib/format";
import { configKey, useRpcData } from "../lib/useRpc";
import { Plot } from "../components/Plot";
import { Badge, Button, Card, EngField, Hint, NumField, PageHeader, Row, Segmented, Stat } from "../components/ui";

type Mode = "up" | "updown";

/**
 * Stepped sweep driven from the PC: each point is a seamless frequency change
 * (write the idle profile, switch SEL_REG). Over the USB-UART link one point costs
 * roughly 10 commands ≈ 30–60 мс, so the dwell is limited from below.
 */
export function SweepView() {
  const snap = useStore((s) => s.snap)!;
  const call = useStore((s) => s.call);
  const [lo, setLo] = useState(48e6);
  const [hi, setHi] = useState(52e6);
  const [step, setStep] = useState(250e3);
  const [dwell, setDwell] = useState(200);
  const [mode, setMode] = useState<Mode>("up");
  const [running, setRunning] = useState(false);
  const [idx, setIdx] = useState<number | null>(null);
  const [passes, setPasses] = useState(0);
  const timer = useRef<number | null>(null);
  const pos = useRef({ i: 0, dir: 1 });
  const key = `${lo}:${hi}:${step}:${configKey(snap.regs, snap.settings)}`;
  const { data: plan, error } = useRpcData<{ f: number[]; dbm: number[] }>("sweep_plan", { lo, hi, step }, key);
  const n = plan?.f.length ?? 0;
  const hw = snap.link.kind !== "sim";
  const minDwell = hw ? 50 : 20;

  const cfg = useRef({ plan, mode, dwell });
  cfg.current = { plan, mode, dwell };
  const live = useRef(false);

  const stop = () => {
    live.current = false;
    if (timer.current != null) window.clearTimeout(timer.current);
    timer.current = null;
    setRunning(false);
  };
  useEffect(() => () => {
    live.current = false;
    if (timer.current != null) window.clearTimeout(timer.current);
  }, []);

  const tick = async () => {
    const { plan: p, mode: m, dwell: dw } = cfg.current;
    if (!live.current) return;
    if (!p || !p.f.length) return stop();
    const { i } = pos.current;
    setIdx(i);
    const t0 = performance.now();
    const ok = await useStore.getState().call("set_frequency", { f: p.f[i], seamless: true }, true);
    if (!live.current) return;
    if (ok == null) {
      useStore.getState().toast("err", `Свип остановлен на ${fmtEng(p.f[i], "Hz", 6)}: ошибка записи`);
      return stop();
    }
    // next index
    let { dir } = pos.current;
    let j = i + dir;
    if (j >= p.f.length || j < 0) {
      setPasses((x) => x + 1);
      if (m === "updown" && p.f.length > 1) {
        dir = -dir;
        j = i + dir;
      } else j = 0;
    }
    pos.current = { i: j, dir };
    const spent = performance.now() - t0;
    timer.current = window.setTimeout(tick, Math.max(0, dw - spent));
  };

  const start = () => {
    if (!n) return;
    pos.current = { i: 0, dir: 1 };
    setPasses(0);
    live.current = true;
    setRunning(true);
    timer.current = window.setTimeout(tick, 0);
  };

  const cur = idx != null && plan ? plan.f[idx] : null;

  return (
    <div className="max-w-[1500px]">
      <PageHeader
        icon={<MoveRight size={20} />}
        title="Свип частоты"
        subtitle="Ступенчатая перестройка по сетке. Каждая точка — бесшовная смена частоты через профили 0/1. Для непрерывной перестройки без ступенек используйте ЛЧМ на вкладке «Модуляция»."
        actions={
          running ? (
            <Button variant="danger" icon={<Square size={13} />} onClick={stop}>
              Стоп
            </Button>
          ) : (
            <Button variant="primary" icon={<Play size={13} />} onClick={start} disabled={!n}>
              Запустить
            </Button>
          )
        }
      />
      <div className="grid gap-4 xl:grid-cols-[380px_minmax(0,1fr)]">
        <div className="flex flex-col gap-4">
          <Card title="Сетка">
            <Row label="От">
              <EngField value={lo} unit="Hz" onCommit={setLo} min={0} max={240e6} />
            </Row>
            <Row label="До">
              <EngField value={hi} unit="Hz" onCommit={setHi} min={0} max={240e6} />
            </Row>
            <Row label="Шаг">
              <EngField value={step} unit="Hz" onCommit={setStep} min={1e-3} max={100e6} />
            </Row>
            <Row label="Время на точке" hint={`не меньше ${minDwell} мс`}>
              <NumField value={dwell} min={minDwell} max={60000} step={10} unit="мс" onCommit={setDwell} />
            </Row>
            <Row label="Проход">
              <Segmented size="sm" options={[{ value: "up" as Mode, label: "Пила ↗" }, { value: "updown" as Mode, label: "Туда-обратно ↗↘" }]} value={mode} onChange={setMode} />
            </Row>
            {error && <Hint tone="err">{error}</Hint>}
          </Card>
          <div className="grid grid-cols-2 gap-2">
            <Stat label="Точек" value={n} />
            <Stat label="Период прохода" value={fmtEng((n * dwell) / 1000, "s", 3)} />
            <Stat label="Сейчас" value={cur != null ? fmtEng(cur, "Hz", 7) : "—"} tone="sig" />
            <Stat label="Проходов" value={passes} />
          </div>
          {hw ? (
            <Hint tone="warn">Плата: одна точка — около десяти команд по UART (запись трёх слов dPh, проверка, SEL_REG). При шаге меньше 50 мс точки будут растягиваться.</Hint>
          ) : (
            <Hint>Симулятор: смотрите, как меняются «Частота выхода» в верхней строке и номер профиля 0/1 в журнале шины (⌘/Ctrl + J).</Hint>
          )}
        </div>
        <Card title="Ожидаемый уровень по сетке" subtitle="Модель тракта: sin(x)/x × ФНЧ × аттенюатор при текущем Mul">
          {plan && (
            <Plot
              height={380}
              series={[{ name: "XW2", x: plan.f, y: plan.dbm, color: "var(--sig)", width: 2 }]}
              xFmt={axisHz}
              yLabel="дБм"
              shade={[48e6, 52e6]}
              vMarkers={cur != null ? [{ x: cur, label: fmtEng(cur, "Hz", 6), color: "var(--accent)" }] : []}
              points={cur != null && idx != null ? [{ x: cur, y: plan.dbm[idx], color: "var(--accent)" }] : []}
              tooltipX={(v) => fmtEng(v, "Hz", 6)}
            />
          )}
          <div className="mt-2 flex gap-2 flex-wrap">
            {running && <Badge tone="ok">идёт свип</Badge>}
            <Button size="sm" variant="ghost" onClick={() => call("set_frequency", { f: lo, seamless: true })}>
              Встать на начало
            </Button>
          </div>
        </Card>
      </div>
    </div>
  );
}
