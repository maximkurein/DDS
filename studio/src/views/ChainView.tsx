import { Gauge } from "lucide-react";
import { useStore } from "../lib/store";
import { axisHz, fmtEng } from "../lib/format";
import { configKey, useRpcData } from "../lib/useRpc";
import type { PhaseNoiseData, ResponseData } from "../lib/types";
import { Plot } from "../components/Plot";
import { Card, Hint, PageHeader, Stat } from "../components/ui";

export function ChainView() {
  const snap = useStore((s) => s.snap)!;
  const c = snap.derived.calc;
  const main = c.ch1.on_dac1 ? c.ch1 : c.ch2;
  const key = configKey(snap.regs, snap.settings);
  const { data: r } = useRpcData<ResponseData>("response", { lo: 1e6, hi: 600e6, n: 600 }, key);
  const { data: pn } = useRpcData<PhaseNoiseData>("phase_noise", {}, key);

  return (
    <div className="max-w-[1600px]">
      <PageHeader
        icon={<Gauge size={20} />}
        title="Выходной тракт и фазовый шум"
        subtitle="АЧХ платы от выхода ЦАП до XW2 (ABCD-модель с потерями катушек), спад sin(x)/x ступенчатого ЦАП и итоговый уровень. Ниже — фазовый шум по таблице 8.1 и пересчёт шума такта."
      />
      {r && (
        <div className="grid gap-2 mb-4 grid-cols-[repeat(auto-fill,minmax(170px,1fr))]">
          <Stat label="Срез ФНЧ (−3 дБ)" value={fmtEng(r.fc_3db, "Hz", 4)} tone="ref" />
          <Stat label="Неравномерность 48–52 МГц" value={`${r.band_ripple_db.toFixed(2)} дБ`} />
          <Stat label="Потери тракта на fout" value={`${c.chain_db.toFixed(2)} дБ`} sub={fmtEng(main.fout, "Hz", 6)} />
          <Stat label="sin(x)/x на fout" value={`${c.sinc_db.toFixed(2)} дБ`} />
          <Stat label="Образ Fclk − f" value={`${c.image_dbm.toFixed(1)} дБм`} sub={fmtEng(c.image_hz, "Hz", 4)} tone="warn" />
          <Stat label="IFS" value={fmtEng(c.ifs, "A", 3)} sub={`OUTM1 до ${c.v_outm_max.toFixed(2)} В`} />
        </div>
      )}
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <Card title="АЧХ" subtitle="ФНЧ+аттенюатор и sin(x)/x — дБ (левая ось); уровень на XW2 при текущем Mul — дБм (правая ось)">
          {r && (
            <Plot
              height={340}
              series={[
                { name: "ФНЧ + аттенюатор", x: r.f, y: r.chain_db, color: "var(--ref)", width: 2 },
                { name: "sin(x)/x", x: r.f, y: r.sinc_db, color: "var(--clk)", width: 1.5, dash: "5 4" },
                { name: "Уровень на XW2", x: r.f, y: r.out_dbm, color: "var(--sig)", width: 1.5, axis: "y2" },
              ]}
              xDomain={[1e6, 600e6]}
              yDomain={[-120, 5]}
              y2Domain={[-130, 0]}
              xFmt={axisHz}
              yLabel="дБ"
              y2Label="дБм"
              shade={[48e6, 52e6]}
              vMarkers={[
                { x: main.fout, label: "f", color: "var(--sig)" },
                { x: c.image_hz, label: "Fclk − f", color: "var(--warn)", dash: true },
              ]}
              legend
              tooltipX={(v) => fmtEng(v, "Hz", 5)}
            />
          )}
        </Card>
        <Card title="Элементы тракта" subtitle="По схеме ревизии F">
          {r && (
            <table className="w-full text-[12px]">
              <tbody>
                {r.elements.map(([n, v], i) => (
                  <tr key={i} className="border-t border-line/60 first:border-0">
                    <td className="py-1 font-mono text-accent pr-3">{n}</td>
                    <td className="py-1 text-fg2">{v}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <Hint>Добротность катушек и RREF меняются на вкладке «Частота» → «Параметры платы».</Hint>
        </Card>
      </div>

      <div className="grid gap-4 mt-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <Card title="Фазовый шум на выходе" subtitle="дБн/Гц от отстройки; пунктир — точки таблицы 8.1 (такт 1 ГГц)">
          {pn && (
            <Plot
              height={300}
              xLog
              series={[
                { name: `${fmtEng(pn.fout, "Hz", 4)}`, x: pn.offsets, y: pn.dbc_hz, color: "var(--sig)", width: 2 },
                ...pn.ref_curves.map(([f, pts], i) => ({
                  name: `ТО: ${fmtEng(f, "Hz", 3)}`,
                  x: pts.map((p) => p[0]),
                  y: pts.map((p) => p[1]),
                  color: ["var(--clk)", "var(--ref)", "var(--dig)", "var(--ctl)"][i % 4],
                  width: 1,
                  dash: "4 4",
                })),
              ]}
              xFmt={axisHz}
              yLabel="дБн/Гц"
              legend
              tooltipX={(v) => fmtEng(v, "Hz", 3)}
            />
          )}
        </Card>
        <Card title="Джиттер">
          {pn && (
            <div className="flex flex-col gap-2">
              <Stat label="Интегральный джиттер" value={fmtEng(pn.jitter_s, "s", 3)} tone="sig" sub="по показанной полосе отстроек" />
              <Hint>
                DDS делит частоту такта: шум источника 480 МГц попадает на выход, уменьшенный на 20·lg(Fclk/Fout) ≈ {(20 * Math.log10(c.fclk / Math.max(main.fout, 1))).toFixed(1)} дБ. Собственный шум кристалла — по ТО (таблица задана для такта 1 ГГц, это оценка).
              </Hint>
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
