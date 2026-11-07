import { useState } from "react";
import { ChartSpline, LoaderCircle } from "lucide-react";
import { useStore } from "../lib/store";
import { axisHz, fmtEng } from "../lib/format";
import { configKey, useRpcData } from "../lib/useRpc";
import type { SpectrumData, Tone } from "../lib/types";
import { Plot } from "../components/Plot";
import { Badge, Card, Hint, PageHeader, Segmented, Stat } from "../components/ui";

const SPANS = [
  { value: "band", label: "40–60 МГц" },
  { value: "nyq", label: "0…Fclk/2" },
  { value: "1g", label: "0…1 ГГц" },
  { value: "2g", label: "0…2 ГГц" },
] as const;
type Span = (typeof SPANS)[number]["value"];

const KIND: Record<Tone["kind"], { label: string; tone: "sig" | "warn" | "clk" | "muted"; color: string }> = {
  carrier: { label: "несущая", tone: "sig", color: "var(--sig)" },
  image: { label: "образ", tone: "warn", color: "var(--warn)" },
  harmonic: { label: "гармоника", tone: "clk", color: "var(--clk)" },
  spur: { label: "спур", tone: "muted", color: "var(--muted)" },
};

export function SpectrumView() {
  const snap = useStore((s) => s.snap)!;
  const fclk = snap.derived.calc.fclk;
  const [span, setSpan] = useState<Span>("1g");
  const [log2n, setLog2n] = useState(14);
  const [lo, hi] = span === "band" ? [40e6, 60e6] : span === "nyq" ? [0, fclk / 2] : span === "1g" ? [0, 1e9] : [0, 2e9];
  const { data: sp, busy, error } = useRpcData<SpectrumData>("spectrum", { lo, hi, log2n, points: 1600 }, `${lo}:${hi}:${log2n}:${configKey(snap.regs, snap.settings)}`, 200);
  const floor = sp ? Math.min(...sp.dbm.filter(isFinite)) : -160;
  const yLo = Math.max(-170, Math.floor(floor / 10) * 10);
  const tones = sp?.tones.filter((t) => t.f >= lo && t.f <= hi) ?? [];

  return (
    <div className="max-w-[1600px]">
      <PageHeader
        icon={<ChartSpline size={20} />}
        title="Спектр на выходе"
        subtitle="Бит-в-бит модель канала (аккумулятор, усечение фазы, дизеринг, 10-битный ЦАП) + БПФ. Затем каждый бин переносится во все зоны Найквиста через sin(x)/x, ФНЧ и аттенюатор платы. Гармоники ЦАП — оценка по SFDR (ТО, табл. 8.1)."
        actions={
          <>
            {busy && <LoaderCircle size={15} className="spin text-muted" />}
            <Segmented options={SPANS.map((s) => ({ value: s.value, label: s.label }))} value={span} onChange={setSpan} />
            <Segmented size="sm" options={[12, 14, 16].map((n) => ({ value: n, label: `2^${n}` }))} value={log2n} onChange={setLog2n} />
          </>
        }
      />
      {error && <Hint tone="err">{error}</Hint>}
      {sp && (
        <div className="flex flex-col gap-4">
          <div className="grid gap-2 grid-cols-[repeat(auto-fill,minmax(170px,1fr))]">
            <Stat label="Несущая на XW2" value={`${sp.carrier_dbm.toFixed(2)} дБм`} tone="sig" />
            <Stat label="SFDR в окне" value={`${sp.sfdr_db.toFixed(1)} дБн`} sub="худшая помеха относительно несущей" />
            <Stat label="SFDR цифровой" value={`${sp.sfdr_digital_db.toFixed(1)} дБн`} sub="только отсчёты, 0…Fclk/2" />
            <Stat label="Полоса бина" value={fmtEng(sp.rbw_hz, "Hz", 3)} sub={`БПФ ${sp.n} точек`} />
            <Stat label="Такт" value={fmtEng(sp.fs, "Hz", 4)} />
          </div>
          <Card title="Анализатор на XW2" subtitle="дБм в полосе бина; заштрихована рабочая полоса 48–52 МГц">
            <Plot
              variant="instrument"
              height={340}
              series={[{ name: "XW2", x: sp.f, y: sp.dbm, color: "var(--sig)", width: 1.3 }]}
              xDomain={[lo, hi]}
              yDomain={[yLo, 10]}
              xFmt={axisHz}
              yFmt={(v) => v.toFixed(0)}
              yLabel="дБм"
              shade={[48e6, 52e6]}
              vMarkers={span !== "band" ? [{ x: fclk / 2, label: "Fclk/2", dash: true, color: "var(--muted)" }, ...(fclk <= hi ? [{ x: fclk, label: "Fclk", dash: true, color: "var(--muted)" }] : [])] : []}
              points={tones.slice(0, 14).map((t) => ({ x: t.f, y: t.dbm, label: t.name, color: KIND[t.kind].color }))}
              tooltipX={(v) => fmtEng(v, "Hz", 6)}
            />
          </Card>
          <div className="grid gap-4 xl:grid-cols-2">
            <Card title="Цифровые отсчёты" subtitle="Спектр кодов ЦАП в первой зоне Найквиста, дБ от полной шкалы">
              <Plot
                height={240}
                series={[{ name: "коды", x: sp.dig_f, y: sp.dig_dbfs, color: "var(--dig)", width: 1.1 }]}
                xDomain={[0, sp.fs / 2]}
                yDomain={[-140, 5]}
                xFmt={axisHz}
                yLabel="дБFS"
                tooltipX={(v) => fmtEng(v, "Hz", 6)}
              />
            </Card>
            <Card title="Составляющие" subtitle="Найденные моделью тона в окне">
              <div className="max-h-[250px] overflow-y-auto scroll-thin">
                <table className="w-full text-[12px]">
                  <thead className="text-muted text-[11px] text-left">
                    <tr>
                      <th className="py-1 font-medium">Что</th>
                      <th className="py-1 font-medium">Частота</th>
                      <th className="py-1 font-medium text-right">дБм</th>
                      <th className="py-1 font-medium text-right">дБн</th>
                    </tr>
                  </thead>
                  <tbody className="num">
                    {tones.map((t, i) => (
                      <tr key={i} className="border-t border-line/60">
                        <td className="py-1">
                          <Badge tone={KIND[t.kind].tone}>{KIND[t.kind].label}</Badge> <span className="font-sans">{t.name}</span>
                        </td>
                        <td className="py-1">{fmtEng(t.f, "Hz", 6)}</td>
                        <td className="py-1 text-right">{t.dbm.toFixed(1)}</td>
                        <td className="py-1 text-right">{t.kind === "carrier" ? "—" : t.dbc.toFixed(1)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          </div>
          <Hint>
            Попробуйте: поднимите частоту к 0,4·Fclk на вкладке «Частота» и посмотрите, как образ Fclk − f подходит к несущей; выключите дизеринг фазы — в цифровом спектре появятся дискретные спуры усечения фазы.
          </Hint>
        </div>
      )}
    </div>
  );
}
