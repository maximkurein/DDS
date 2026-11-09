import { AudioWaveform, Binary, ChartSpline, CircuitBoard, Gauge, GitBranch, GraduationCap, Microchip, MoveRight, Shuffle, SlidersHorizontal, Usb } from "lucide-react";
import type { ReactNode } from "react";
import { useSim, useStore } from "../lib/store";
import { fmtEng } from "../lib/format";
import type { ViewId } from "../lib/types";
import { cx } from "./ui";
import versionInfo from "../../config/version.json";

// Views of the app. Domain views (planner, loop, noise, power, capture …) go in
// the "Расчёт" group; keep the order in sync with App.tsx (⌘/Ctrl+1…9).
export const NAV: { id: ViewId; label: string; hint: string; icon: ReactNode; group: string }[] = [
  { id: "overview", label: "Обзор", hint: "Блок-схема кристалла", icon: <Microchip size={16} />, group: "Устройство" },
  { id: "board", label: "Плата", hint: "Разъёмы, переключатели, тракт", icon: <CircuitBoard size={16} />, group: "Устройство" },
  { id: "registers", label: "Регистры", hint: "Биты, запись, чтение", icon: <Binary size={16} />, group: "Устройство" },
  { id: "freq", label: "Частота", hint: "FTW, DIP-код, уровень", icon: <SlidersHorizontal size={16} />, group: "Синтез" },
  { id: "spectrum", label: "Спектр", hint: "Образы, гармоники, SFDR", icon: <ChartSpline size={16} />, group: "Синтез" },
  { id: "chain", label: "Тракт и шум", hint: "ФНЧ, sin(x)/x, фазовый шум", icon: <Gauge size={16} />, group: "Синтез" },
  { id: "sweep", label: "Свип", hint: "Перестройка по сетке", icon: <MoveRight size={16} />, group: "Опыты" },
  { id: "hop", label: "Перебор", hint: "Профили, прыжки частоты", icon: <Shuffle size={16} />, group: "Опыты" },
  { id: "mod", label: "Модуляция", hint: "ЛЧМ, ЧМн, ФМн, КАМ", icon: <AudioWaveform size={16} />, group: "Опыты" },
  { id: "hardware", label: "Подключение", hint: "USB-UART, симулятор", icon: <Usb size={16} />, group: "Практика" },
  { id: "learn", label: "Академия", hint: "Как работает DDS", icon: <GraduationCap size={16} />, group: "Практика" },
  { id: "eco", label: "Ревизии платы", hint: "Исправления и ворота KiCad", icon: <GitBranch size={16} />, group: "Практика" },
];

export function Sidebar() {
  const view = useStore((s) => s.view);
  const go = useStore((s) => s.go);
  const snap = useStore((s) => s.snap)!;
  const sim = useSim();
  const version = useStore((s) => s.meta?.version);
  const side = snap.derived.metrics.filter((m) => !m.top).slice(0, 4);
  let group = "";

  return (
    <nav className="w-[212px] shrink-0 border-r border-line bg-bg2/60 flex flex-col">
      <div className="flex-1 overflow-y-auto scroll-thin px-2.5 py-3">
        {NAV.map((n, i) => {
          const header = n.group !== group;
          group = n.group;
          return (
            <div key={n.id}>
              {header && <div className={cx("px-2 pb-1 text-[10.5px] uppercase tracking-wider text-muted font-medium", i > 0 && "pt-3")}>{n.group}</div>}
              <button
                onClick={() => go(n.id)}
                className={cx(
                  "group w-full flex items-center gap-2.5 rounded-lg px-2 py-1.5 text-left transition-colors",
                  view === n.id ? "bg-panel text-fg shadow-sm ring-1 ring-line" : "text-fg2 hover:bg-panel3/70 hover:text-fg"
                )}
              >
                <span className={cx("shrink-0", view === n.id ? "text-accent" : "text-muted group-hover:text-fg2")}>{n.icon}</span>
                <span className="min-w-0">
                  <span className="block text-[12.5px] font-medium leading-tight">{n.label}</span>
                  <span className="block text-[10.5px] text-muted leading-tight truncate">{n.hint}</span>
                </span>
                <span className="ml-auto text-[10px] text-muted opacity-0 group-hover:opacity-100 num">{i + 1}</span>
              </button>
            </div>
          );
        })}
      </div>

      <div className="m-2.5 rounded-xl border border-line bg-panel p-3 text-[11.5px] space-y-1.5">
        {side.map((m) => (
          <div key={m.key} className="flex items-center justify-between gap-2">
            <span className="text-muted truncate">{m.label}</span>
            <span className="num font-medium">{fmtEng(m.value, m.unit, 3)}</span>
          </div>
        ))}
        {sim?.signals.map((s) => (
          <div key={s.name} className="flex items-center justify-between gap-2">
            <span className="text-muted truncate">{s.name}</span>
            <span className="num">{fmtEng(s.value, s.unit, 3)}</span>
          </div>
        ))}
      </div>
      {version && (
        <div className="px-4 pb-3 -mt-1 text-[10.5px] text-muted num" title="Версия приложения и разработчик">
          DDS Studio v{version} · {versionInfo.app_info.developer}
        </div>
      )}
    </nav>
  );
}
