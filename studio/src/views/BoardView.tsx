import { useState } from "react";
import { CircuitBoard, ShieldAlert } from "lucide-react";
import { useStore } from "../lib/store";
import { BOARD, CHECKLIST, HOTSPOTS, SAFETY, type Hotspot } from "../lib/board";
import { Badge, Button, Card, Hint, PageHeader, cx } from "../components/ui";

const KIND: Record<Hotspot["kind"], { label: string; color: string }> = {
  chip: { label: "кристалл", color: "var(--accent)" },
  connector: { label: "разъём", color: "var(--clk)" },
  jumper: { label: "джампер", color: "var(--warn)" },
  power: { label: "питание", color: "var(--pwr)" },
  testpoint: { label: "контр. точка", color: "var(--ref)" },
  led: { label: "индикатор", color: "var(--ok)" },
  switch: { label: "переключатель", color: "var(--ctl)" },
  filter: { label: "фильтр", color: "var(--ref)" },
};

/** Top view of the board traced from DDS_48_52.kicad_pcb (1 мм = 8 px). */
export function BoardView() {
  const go = useStore((s) => s.go);
  const [sel, setSel] = useState<string>(HOTSPOTS[0]?.id ?? "");
  const [done, setDone] = useState<boolean[]>(() => CHECKLIST.map(() => false));
  const h = HOTSPOTS.find((x) => x.id === sel);

  return (
    <div className="max-w-[1600px]">
      <PageHeader icon={<CircuitBoard size={20} />} title={`Плата ${BOARD.name}`} subtitle={`${BOARD.size} · питание: ${BOARD.supply}. Нажмите на элемент платы, чтобы узнать его назначение.`} />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_400px]">
        <Card bodyClass="!p-3">
          <svg viewBox="0 0 1000 440" className="w-full h-auto">
            <rect x="20" y="20" width="960" height="400" rx="8" fill="color-mix(in srgb, var(--ok) 12%, var(--panel-2))" stroke="var(--line-2)" strokeWidth="2" />
            {/* RF path: XW1 → D1 → filter → XW2 */}
            <path d="M656 392 V300 H788 V268" fill="none" stroke="var(--clk)" strokeOpacity="0.5" strokeWidth="3" strokeDasharray="6 5" />
            <path d="M852 236 H900 V318" fill="none" stroke="var(--sig)" strokeOpacity="0.5" strokeWidth="3" />
            <path d="M938 350 H944" fill="none" stroke="var(--sig)" strokeOpacity="0.5" strokeWidth="3" />
            <path d="M460 170 H508 M548 168 V196 H724" fill="none" stroke="var(--dig)" strokeOpacity="0.45" strokeWidth="2.5" />
            <text x="990" y="436" fontSize="11" fill="var(--muted)" textAnchor="end">
              вид сверху, слой F.Cu · 120 × 50 мм
            </text>
            {HOTSPOTS.map((p) => {
              const k = KIND[p.kind];
              const on = p.id === sel;
              return (
                <g key={p.id} onClick={() => setSel(p.id)} className="cursor-pointer">
                  <rect x={p.x} y={p.y} width={p.w} height={p.h} rx="8" fill={`color-mix(in srgb, ${k.color} ${on ? 35 : 16}%, var(--panel))`} stroke={k.color} strokeWidth={on ? 3 : 1.5} />
                  <text x={p.x + p.w / 2} y={p.y + p.h / 2 + 4} textAnchor="middle" fill="var(--fg)" fontSize={p.w < 44 ? 10 : 13} fontWeight="600">
                    {p.ref}
                  </text>
                </g>
              );
            })}
          </svg>
        </Card>
        <div className="flex flex-col gap-4">
          {h && (
            <Card title={`${h.ref} — ${h.title}`} subtitle={KIND[h.kind].label}>
              <p className="text-[12.5px] text-fg2 leading-relaxed">{h.desc}</p>
              {h.block && (
                <Button size="sm" className="mt-3" onClick={() => go("overview", h.block)}>
                  Показать на блок-схеме
                </Button>
              )}
            </Card>
          )}
          <Card title="Первое включение" subtitle="Отмечайте шаги по мере выполнения">
            <div className="flex flex-col gap-1.5">
              {CHECKLIST.map((c, i) => (
                <label key={i} className="flex items-start gap-2 text-[12.5px] cursor-pointer">
                  <input type="checkbox" className="mt-0.5 accent-[var(--accent)]" checked={done[i]} onChange={() => setDone(done.map((d, j) => (j === i ? !d : d)))} />
                  <span className={cx(done[i] ? "text-muted line-through" : "text-fg2")}>{c}</span>
                </label>
              ))}
            </div>
          </Card>
          <Card title="Безопасность" icon={<ShieldAlert size={15} />}>
            <div className="flex flex-col gap-2">
              {SAFETY.map((s, i) => (
                <Hint key={i} tone="warn">
                  {s}
                </Hint>
              ))}
            </div>
          </Card>
          <div className="flex flex-wrap gap-1.5">
            {Object.entries(KIND).map(([k, v]) => (
              <Badge key={k}>
                <span className="h-2 w-2 rounded-full" style={{ background: v.color }} />
                {v.label}
              </Badge>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
