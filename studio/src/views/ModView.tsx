import { useState } from "react";
import { AudioWaveform, Pause, Play, Power, Save, Shuffle } from "lucide-react";
import { useStore } from "../lib/store";
import { axisHz, fmtEng } from "../lib/format";
import { configKey, useRpcData } from "../lib/useRpc";
import type { LfmPlan, LfmReq, ModKind, ModPlan, ModReq } from "../lib/types";
import { Plot } from "../components/Plot";
import { Badge, Button, Card, EngField, Hint, NumField, PageHeader, Row, Segmented, Select, Stat, Toggle } from "../components/ui";

const KINDS: { value: ModKind; label: string }[] = [
  { value: "fsk2", label: "2-ЧМн (2-FSK)" },
  { value: "fsk4", label: "4-ЧМн (4-FSK)" },
  { value: "bpsk", label: "ФМн-2 (BPSK)" },
  { value: "qpsk", label: "ФМн-4 (QPSK)" },
  { value: "psk8", label: "ФМн-8 (8-PSK)" },
  { value: "ask2", label: "2-АМн (OOK/ASK)" },
  { value: "ask4", label: "4-АМн (4-ASK)" },
  { value: "qam16", label: "КАМ-16 (16-QAM)" },
];

function Constellation({ plan }: { plan: ModPlan }) {
  const R = 130;
  const max = Math.max(1e-9, ...plan.symbols.map((s) => Math.hypot(s.i, s.q)));
  const fsk = plan.kind.startsWith("fsk");
  return (
    <svg viewBox="-160 -160 320 320" className="w-full max-w-[340px] mx-auto h-auto">
      <circle r={R} fill="none" stroke="var(--line)" strokeDasharray="3 4" />
      <line x1={-150} x2={150} y1={0} y2={0} stroke="var(--line-2)" />
      <line y1={-150} y2={150} x1={0} x2={0} stroke="var(--line-2)" />
      <text x={146} y={-6} fontSize="11" fill="var(--muted)" textAnchor="end">
        I
      </text>
      <text x={6} y={-146} fontSize="11" fill="var(--muted)">
        Q
      </text>
      {!fsk &&
        plan.symbols.map((s) => {
          const x = (s.i / max) * R;
          const y = -(s.q / max) * R;
          return (
            <g key={s.profile}>
              <circle cx={x} cy={y} r={6} fill="var(--sig)" />
              <text x={x} y={y - 10} fontSize="10" textAnchor="middle" fill="var(--fg2)" className="num">
                {s.bits}
              </text>
            </g>
          );
        })}
      {fsk && (
        <text x={0} y={4} textAnchor="middle" fontSize="12" fill="var(--muted)">
          ЧМн: символы различаются частотой
        </text>
      )}
    </svg>
  );
}

function LfmPanel() {
  const snap = useStore((s) => s.snap)!;
  const call = useStore((s) => s.call);
  const [req, setReq] = useState<LfmReq>({ f_start: 48e6, f_stop: 52e6, t_ramp: 1e-3, t_hold: 0.2e-3, shape: "saw", pause_off: false });
  const set = (p: Partial<LfmReq>) => setReq({ ...req, ...p });
  const { data: plan } = useRpcData<LfmPlan>("lfm_design", { req }, `${JSON.stringify(req)}:${configKey(snap.regs, snap.settings)}`);
  const on = (snap.fields["CH1_LS_on"] ?? 0) === 1;

  return (
    <div className="grid gap-4 xl:grid-cols-[380px_minmax(0,1fr)]">
      <div className="flex flex-col gap-4">
        <Card title="Параметры ЛЧМ" subtitle="Канал 1, генератор ЛЧМ (ТО, разд. 3.3)">
          <Row label="Начало">
            <EngField value={req.f_start} unit="Hz" onCommit={(v) => set({ f_start: v })} min={0} max={240e6} />
          </Row>
          <Row label="Конец">
            <EngField value={req.f_stop} unit="Hz" onCommit={(v) => set({ f_stop: v })} min={0} max={240e6} />
          </Row>
          <Row label="Нарастание" hint="стадия 1">
            <EngField value={req.t_ramp} unit="s" onCommit={(v) => set({ t_ramp: v })} min={1e-8} max={100} />
          </Row>
          <Row label="Пауза" hint="стадии 2 и 4">
            <EngField value={req.t_hold} unit="s" onCommit={(v) => set({ t_hold: v })} min={1e-8} max={100} />
          </Row>
          <Row label="Форма">
            <Segmented
              size="sm"
              options={[
                { value: "saw" as const, label: "Пила" },
                { value: "tri" as const, label: "Треугольник" },
                { value: "single" as const, label: "Однократно" },
              ]}
              value={req.shape}
              onChange={(shape) => set({ shape })}
            />
          </Row>
          <Row label="Выключать в паузах" hint="импульсный ЛЧМ">
            <Toggle checked={req.pause_off} onChange={(pause_off) => set({ pause_off })} />
          </Row>
          <div className="flex flex-wrap gap-2 mt-2">
            <Button variant="primary" icon={<Save size={13} />} onClick={() => call("lfm_apply", { req })}>
              Записать
            </Button>
            <Button icon={<Play size={13} />} onClick={() => call("lfm_start")} disabled={!on}>
              Старт
            </Button>
            <Button icon={<Pause size={13} />} onClick={() => call("lfm_stop")} disabled={!on}>
              Стоп
            </Button>
            <Button variant="ghost" icon={<Power size={13} />} onClick={() => call("lfm_off")} disabled={!on}>
              Выключить
            </Button>
          </div>
          <div className="mt-2">{on ? <Badge tone="ok">ЛЧМ включён (CH1_LS_on = 1)</Badge> : <Badge>ЛЧМ выключен — работают профили</Badge>}</div>
        </Card>
        {plan && (
          <div className="grid grid-cols-2 gap-2">
            <Stat label="Скорость" value={fmtEng(plan.slope_hz_s, "Hz", 4) + "/с"} />
            <Stat label="Шаг скорости" value={fmtEng(plan.slope_step, "Hz", 3) + "/с"} />
            <Stat label="Фактический конец" value={fmtEng(plan.f_end, "Hz", 7)} tone="sig" />
            <Stat label="Период" value={fmtEng(plan.period, "s", 4)} />
            <Stat label="dF1" value={plan.df1.toLocaleString("ru-RU")} />
            <Stat label="TPH1" value={plan.tph[0].toLocaleString("ru-RU")} sub="× 4 такта" />
          </div>
        )}
      </div>
      <div className="flex flex-col gap-4 min-w-0">
        <Card title="Частота во времени" subtitle="Два периода; серым — интервалы, где выход выключен">
          {plan && (
            <Plot
              height={320}
              series={[
                { name: "f(t)", x: plan.curve_t, y: plan.curve_f, color: "var(--sig)", width: 2 },
                { name: "выход выкл.", x: plan.curve_t, y: plan.curve_f.map((f, i) => (plan.curve_on[i] ? NaN : f)), color: "var(--muted)", width: 3 },
              ]}
              xFmt={(v) => fmtEng(v, "s", 3)}
              yFmt={axisHz}
              yLabel="частота"
              tooltipX={(v) => fmtEng(v, "s", 4)}
            />
          )}
        </Card>
        {plan && plan.notes.length > 0 && (
          <div className="flex flex-col gap-2">
            {plan.notes.map((n, i) => (
              <Hint key={i}>{n}</Hint>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function KeyingPanel() {
  const snap = useStore((s) => s.snap)!;
  const call = useStore((s) => s.call);
  const [req, setReq] = useState<ModReq>({ kind: "qpsk", f_center: 50e6, deviation: 100e3, mul: 4000 });
  const [ms, setMs] = useState(500);
  const set = (p: Partial<ModReq>) => setReq({ ...req, ...p });
  const { data: plan } = useRpcData<ModPlan>("mod_design", { req }, `${JSON.stringify(req)}:${configKey(snap.regs, snap.settings)}`);
  const fsk = req.kind.startsWith("fsk");

  return (
    <div className="grid gap-4 xl:grid-cols-[380px_minmax(0,1fr)]">
      <div className="flex flex-col gap-4">
        <Card title="Схема манипуляции" subtitle="Символ = профиль канала 1">
          <Row label="Вид">
            <Select options={KINDS} value={req.kind} onChange={(kind) => set({ kind })} className="w-full" />
          </Row>
          <Row label="Несущая">
            <EngField value={req.f_center} unit="Hz" onCommit={(v) => set({ f_center: v })} min={0} max={240e6} />
          </Row>
          {fsk && (
            <Row label="Разнос частот" hint="между соседними">
              <EngField value={req.deviation} unit="Hz" onCommit={(v) => set({ deviation: v })} min={0} max={20e6} />
            </Row>
          )}
          <Row label="Пиковый Mul">
            <NumField value={req.mul} min={1} max={4095} step={64} onCommit={(mul) => set({ mul })} />
          </Row>
          <div className="flex flex-wrap gap-2 mt-2">
            <Button variant="primary" icon={<Save size={13} />} onClick={() => call("mod_apply", { req })}>
              Записать профили
            </Button>
            {plan && (
              <Button icon={<Shuffle size={13} />} onClick={() => call("hop", { n: plan.symbols.length, ms })} title="Прошивка перебирает символы по кругу">
                Перебор символов ({plan.symbols.length})
              </Button>
            )}
          </div>
          <Row label="Длительность символа" hint="для перебора">
            <NumField value={ms} min={1} max={60000} step={50} unit="мс" onCommit={setMs} />
          </Row>
        </Card>
        {plan && (
          <div className="flex flex-col gap-2">
            {plan.notes.map((n, i) => (
              <Hint key={i}>{n}</Hint>
            ))}
          </div>
        )}
      </div>
      {plan && (
        <div className="flex flex-col gap-4 min-w-0">
          <div className="grid gap-4 lg:grid-cols-[360px_minmax(0,1fr)]">
            <Card title={plan.title} subtitle={`${plan.bits_per_symbol} бит на символ, код Грея`}>
              <Constellation plan={plan} />
            </Card>
            <Card title="Символы → профили">
              <div className="max-h-[360px] overflow-y-auto scroll-thin">
                <table className="w-full text-[12px] num">
                  <thead className="text-muted text-[11px] text-left">
                    <tr>
                      <th className="py-1 font-medium">Профиль</th>
                      <th className="py-1 font-medium">Биты</th>
                      <th className="py-1 font-medium">Частота</th>
                      <th className="py-1 font-medium text-right">Фаза</th>
                      <th className="py-1 font-medium text-right">Mul</th>
                    </tr>
                  </thead>
                  <tbody>
                    {plan.symbols.map((s) => (
                      <tr key={s.profile} className="border-t border-line/60">
                        <td className="py-1">{s.profile}</td>
                        <td className="py-1 font-mono">{s.bits}</td>
                        <td className="py-1">{fmtEng(s.f, "Hz", 7)}</td>
                        <td className="py-1 text-right">{s.phase_deg.toFixed(1)}°</td>
                        <td className="py-1 text-right">{s.mul}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          </div>
          <Card title="Пример последовательности" subtitle="Символы из псевдослучайной последовательности: так менялся бы Pr_1 при передаче">
            <Plot
              height={180}
              series={[
                {
                  name: "профиль",
                  x: plan.sequence.flatMap((_, i) => [i, i + 1]),
                  y: plan.sequence.flatMap((v) => [v, v]),
                  color: "var(--dig)",
                  width: 2,
                },
              ]}
              xFmt={(v) => v.toFixed(0)}
              yFmt={(v) => v.toFixed(0)}
              xLabel="номер символа"
              yLabel="Pr_1"
            />
          </Card>
        </div>
      )}
    </div>
  );
}

export function ModView() {
  const [tab, setTab] = useState<"lfm" | "key">("lfm");
  return (
    <div className="max-w-[1600px]">
      <PageHeader
        icon={<AudioWaveform size={20} />}
        title="Модуляция"
        subtitle="ЛЧМ — встроенный генератор линейной частотной модуляции. Манипуляции (ЧМн, ФМн, АМн, КАМ) — набор профилей, между которыми переключается SEL_REG."
        actions={
          <Segmented
            options={[
              { value: "lfm" as const, label: "ЛЧМ" },
              { value: "key" as const, label: "Манипуляции" },
            ]}
            value={tab}
            onChange={setTab}
          />
        }
      />
      {tab === "lfm" ? <LfmPanel /> : <KeyingPanel />}
    </div>
  );
}
