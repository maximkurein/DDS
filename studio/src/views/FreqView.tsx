import { useEffect, useState } from "react";
import { ArrowRight, Crosshair, Minus, Plus, SlidersHorizontal, Target } from "lucide-react";
import { useStore } from "../lib/store";
import { fmtEng, hexW } from "../lib/format";
import { configKey, useRpcData } from "../lib/useRpc";
import type { FreqPlan, Settings } from "../lib/types";
import { Badge, Button, Card, EngField, Hint, NumField, PageHeader, Row, Segmented, Slider, Stat, Toggle, cx } from "../components/ui";

/** DIP switch bank: ON = bit 1 (the lever is down, the pin is grounded). */
function DipBank({ title, bits, offset }: { title: string; bits: boolean[]; offset: number }) {
  return (
    <div className="flex flex-col gap-1">
      <div className="text-[11px] text-muted">{title}</div>
      <div className="flex gap-1 rounded-lg bg-[#c03a2b] p-1.5 shadow-inner">
        {bits.map((on, i) => (
          <div key={i} className="flex flex-col items-center gap-0.5">
            <div className="h-10 w-5 rounded-[3px] bg-[#f4efe6] relative">
              <div className={cx("absolute left-0.5 right-0.5 h-4 rounded-[2px] bg-[#3b3b3b] transition-all", on ? "bottom-0.5" : "top-0.5")} />
            </div>
            <span className="text-[9px] text-white/90 num">{i + 1}</span>
          </div>
        ))}
      </div>
      <div className="text-[10px] text-muted num">
        биты {offset}…{offset + bits.length - 1} · ON = 1
      </div>
    </div>
  );
}

export function FreqView() {
  const snap = useStore((s) => s.snap)!;
  const meta = useStore((s) => s.meta)!;
  const call = useStore((s) => s.call);
  const setSettings = useStore((s) => s.setSettings);
  const c = snap.derived.calc;
  const st = snap.settings;
  const main = c.ch1.on_dac1 ? c.ch1 : c.ch2;
  const [f, setF] = useState(() => (main.fout > 0 ? Math.round(main.fout) : 50e6));
  const { data: plan } = useRpcData<FreqPlan>("plan_frequency", { f }, `${f}:${configKey(snap.regs, st)}`);
  const pr = snap.fields["Pr_1"] ?? 0;
  const mulName = `CH1_Mul${Math.min(pr, meta.chip.n_prof[0] - 1)}`;
  const mulRaw = snap.fields[mulName] ?? 0;
  const mul = mulRaw >= 4096 ? mulRaw - 8192 : mulRaw;
  const [mulDraft, setMulDraft] = useState(mul);
  useEffect(() => setMulDraft(mul), [mul]);

  const apply = (x = f) => call("set_frequency", { f: x, seamless: true });
  const nudge = (k: number) => {
    const x = Math.max(st.band_lo_hz - 2e6, f + k * st.step_hz);
    setF(x);
    void apply(x);
  };
  const s2 = plan?.dip_on.slice(0, 8) ?? Array(8).fill(false);
  const s3 = plan?.dip_on.slice(8, 12) ?? Array(4).fill(false);
  const sim = snap.link.kind === "sim";
  const S = (k: keyof Settings) => (v: number) => setSettings({ [k]: v } as Partial<Settings>);

  return (
    <div className="max-w-[1500px]">
      <PageHeader
        icon={<SlidersHorizontal size={20} />}
        title="Частота и уровень"
        subtitle="Планировщик частоты: приращение фазы dPh, ошибка сетки, код DIP-переключателей и ожидаемый уровень на XW2. «Установить» меняет частоту бесшовно — через неактивный профиль 0/1."
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_420px]">
        <div className="flex flex-col gap-4 min-w-0">
          <Card title="Желаемая частота" icon={<Target size={15} />}>
            <div className="flex flex-wrap items-center gap-2">
              <EngField big value={f} unit="Hz" onCommit={setF} width="w-60" min={0} max={c.fclk / 2} />
              <Button icon={<Minus size={14} />} onClick={() => nudge(-1)} title={`−${fmtEng(st.step_hz, "Hz")} и установить`} />
              <Button icon={<Plus size={14} />} onClick={() => nudge(1)} title={`+${fmtEng(st.step_hz, "Hz")} и установить`} />
              <Button variant="primary" icon={<ArrowRight size={14} />} onClick={() => apply()}>
                Установить
              </Button>
              <div className="flex-1" />
              <Segmented
                options={[48e6, 49e6, 50e6, 51e6, 52e6].map((x) => ({ value: x, label: `${x / 1e6}` }))}
                value={f}
                onChange={(x) => {
                  setF(x);
                  void apply(x);
                }}
              />
              <span className="text-[11px] text-muted">МГц</span>
            </div>
            {plan && (
              <div className="mt-4 grid gap-2 grid-cols-[repeat(auto-fill,minmax(170px,1fr))]">
                <Stat label="Будет на выходе" value={fmtEng(plan.actual, "Hz", 12)} tone="sig" />
                <Stat label="Ошибка сетки" value={fmtEng(plan.error_hz, "Hz", 3)} sub={`шаг ${fmtEng(plan.resolution, "Hz", 4)}`} />
                <Stat label="dPh (48 бит)" value={`0x${plan.ftw_hex}`} sub={plan.ftw.toLocaleString("ru-RU")} />
                <Stat label="Уровень на XW2" value={`${plan.level_dbm.toFixed(2)} дБм`} tone={Math.abs(plan.level_dbm - st.target_dbm) < 1 ? "ok" : "warn"} sub={`цель ${st.target_dbm} дБм`} />
              </div>
            )}
            {plan && (
              <div className="mt-3 flex flex-wrap gap-2 items-center text-[12px]">
                <span className="text-muted">Регистры профиля:</span>
                {(["L", "M", "H"] as const).map((p, i) => (
                  <Badge key={p} tone="dig">
                    dPh_{p} = {hexW(plan.words[i], 16)}
                  </Badge>
                ))}
                {!plan.in_band && <Badge tone="warn">вне полосы платы {fmtEng(st.band_lo_hz, "Hz")}…{fmtEng(st.band_hi_hz, "Hz")}</Badge>}
              </div>
            )}
          </Card>

          <Card title="DIP-переключатели для этой частоты" subtitle="f = 48 МГц + 1 кГц · N. Так плата задаёт частоту без ПК (режим DIP).">
            {plan?.dip_n != null ? (
              <div className="flex flex-wrap items-end gap-6">
                <DipBank title="S2 — N[7:0]" bits={s2} offset={0} />
                <DipBank title="S3 — N[11:8]" bits={s3} offset={8} />
                <div className="flex flex-col gap-2">
                  <Stat label="Код N" value={plan.dip_n} sub={`0x${plan.dip_n.toString(16).toUpperCase().padStart(3, "0")}`} />
                  {sim && (
                    <Button size="sm" icon={<Crosshair size={13} />} onClick={() => call("set_sim_dip", { value: plan.dip_n })}>
                      Выставить на виртуальной плате
                    </Button>
                  )}
                </div>
              </div>
            ) : (
              <Hint tone="warn">Частота не лежит на сетке 1 кГц в диапазоне 48–52 МГц (N 0…4000): переключателями её не задать, только с ПК.</Hint>
            )}
          </Card>
        </div>

        <div className="flex flex-col gap-4">
          <Card title={`Амплитуда профиля ${pr}`} subtitle="A = Mul / 4096. Отрицательный Mul — инверсия (180°).">
            <div className="flex items-center gap-3">
              <Slider value={mulDraft} min={0} max={4095} onChange={setMulDraft} />
              <NumField value={mul} min={-4096} max={4095} onCommit={(v) => call("set_profile", { ch: 1, y: pr, mul: v })} width="w-16" />
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              <Button size="sm" variant="primary" disabled={mulDraft === mul} onClick={() => call("set_profile", { ch: 1, y: pr, mul: mulDraft })}>
                Применить {mulDraft}
              </Button>
              {c.mul_for_target != null && (
                <Button size="sm" onClick={() => call("set_profile", { ch: 1, y: pr, mul: c.mul_for_target })} title="По модели тракта">
                  Mul для {st.target_dbm} дБм ≈ {c.mul_for_target}
                </Button>
              )}
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2">
              <Stat label="Сейчас на XW2" value={`${c.p_out_dbm.toFixed(2)} дБм`} tone="sig" />
              <Stat label="При полной шкале" value={`${c.p_fullscale_dbm.toFixed(2)} дБм`} />
            </div>
            <Hint>Модель тракта — оценка. Точный Mul подбирайте по анализатору спектра (design_notes п. 5) и запишите его в DDS_AMPLITUDE_MUL прошивки.</Hint>
          </Card>

          <Card title="Параметры платы" subtitle="Не регистры: то, что модель знает о плате и стенде">
            <Row label="Такт Fclk" hint="XW1, после S1">
              <EngField value={st.fclk_hz} unit="Hz" onCommit={S("fclk_hz")} min={1e6} max={1.2e9} />
            </Row>
            <Row label="Уровень такта" hint="−10…+5 дБм">
              <NumField value={st.clk_dbm} integer={false} step={0.5} unit="дБм" onCommit={S("clk_dbm")} />
            </Row>
            <Row label="Источник такта" hint="перемычка S1">
              <Segmented size="sm" options={[{ value: 1, label: "XW1 (S1 замкн.)" }, { value: 0, label: "G1" }]} value={st.csel} onChange={S("csel")} />
            </Row>
            <Row label="Цель на XW2">
              <NumField value={st.target_dbm} integer={false} step={0.5} unit="дБм" onCommit={S("target_dbm")} />
            </Row>
            <Row label="RREF (R11)" hint="IFS = 72 В / RREF">
              <NumField value={st.rref_ohm} min={1000} max={20000} unit="Ом" onCommit={S("rref_ohm")} width="w-20" />
            </Row>
            <Row label="Добротность L" hint="катушки фильтра">
              <NumField value={st.inductor_q} min={2} max={500} onCommit={S("inductor_q")} />
            </Row>
            <Row label="Шаг ±">
              <EngField value={st.step_hz} unit="Hz" onCommit={S("step_hz")} min={1e-6} max={10e6} />
            </Row>
            <Row label="Дизеринг фазы" hint="ROUTE.pdith1">
              <Toggle checked={(snap.fields["pdith1"] ?? 0) === 1} onChange={(v) => call("set_field", { name: "pdith1", value: v ? 1 : 0 })} />
            </Row>
          </Card>
        </div>
      </div>
    </div>
  );
}
