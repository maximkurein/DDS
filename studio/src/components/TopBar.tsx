import { Cpu, Moon, RotateCcw, Send, Sun, SquareTerminal } from "lucide-react";
import { stateInfo, useSim, useStore } from "../lib/store";
import { fmtEng } from "../lib/format";
import { blockById } from "../lib/blocks";
import { Button, Led, Toggle, cx } from "./ui";

function Readout({ label, value, color, onClick, className }: { label: string; value: string; color: string; onClick?: () => void; className?: string }) {
  return (
    <button
      onClick={onClick}
      className={cx("flex-col items-start leading-none shrink-0 rounded-lg px-2 py-1 hover:bg-panel3 transition-colors", className ?? "flex")}
    >
      <span className="text-[10px] uppercase tracking-wider text-muted whitespace-nowrap">{label}</span>
      <span className="num font-semibold whitespace-nowrap text-[13.5px] mt-0.5" style={{ color }}>
        {value}
      </span>
    </button>
  );
}

export function TopBar() {
  const snap = useStore((s) => s.snap)!;
  const meta = useStore((s) => s.meta)!;
  const status = useStore((s) => s.status);
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);
  const call = useStore((s) => s.call);
  const consoleOpen = useStore((s) => s.consoleOpen);
  const setConsole = useStore((s) => s.setConsole);
  const go = useStore((s) => s.go);
  const sim = useSim();
  const d = snap.derived;
  const si = stateInfo(sim, status?.link_error ?? snap.link_error);
  const dirty = snap.dirty.filter(Boolean).length;
  const errors = d.issues.filter((i) => i.level === "error").length;
  const top = d.metrics.filter((m) => m.top).slice(0, 5);

  return (
    <header className="h-[52px] shrink-0 flex items-center gap-3 border-b border-line bg-bg2/80 backdrop-blur pr-3 pl-4">
      <div className="flex items-center gap-2.5 shrink-0">
        <div className="h-7 w-7 rounded-lg grid place-items-center text-accentfg" style={{ background: "linear-gradient(135deg, var(--accent), var(--clk))" }}>
          <Cpu size={15} strokeWidth={2.5} />
        </div>
        <div className="leading-tight">
          <div className="text-[13px] font-semibold tracking-tight">
            DDS Studio
          </div>
          <div className="text-[10.5px] text-muted">
            {meta.chip.vendor} {meta.chip.name} · {meta.chip.board}
          </div>
        </div>
      </div>

      <div className="h-7 w-px bg-line" />

      <div className="hidden xl:flex items-center gap-1 min-w-0 overflow-hidden">
        {top.map((m, i) => (
          <Readout
            key={m.key}
            className={i < 3 ? "flex" : i < 4 ? "hidden min-[1600px]:flex" : "hidden min-[1800px]:flex"}
            label={m.label}
            value={fmtEng(m.value, m.unit)}
            color={`var(--${blockById(m.block)?.tone ?? "fg"})`}
            onClick={() => go("overview", m.block)}
          />
        ))}
      </div>

      <button
        onClick={() => go("overview")}
        className="flex items-center gap-2 rounded-full border border-line bg-panel px-3 h-8 hover:border-line2 shrink-0"
        title="Состояние кристалла"
      >
        <Led tone={si.tone} pulse={si.tone === "warn"} />
        <span className="text-[12px] font-medium whitespace-nowrap">{si.text}</span>
      </button>

      {errors > 0 && (
        <button onClick={() => go("overview")} className="text-[11.5px] rounded-full px-2.5 h-7 bg-err/12 text-err border border-err/30 whitespace-nowrap">
          {errors} {errors === 1 ? "ошибка" : "ошибки"} конфигурации
        </button>
      )}

      <div className="flex-1" />

      <button
        onClick={() => go("hardware")}
        className={cx(
          "hidden md:flex items-center gap-1.5 rounded-full px-2.5 h-7 text-[11.5px] border whitespace-nowrap",
          snap.link_error ? "border-err/40 text-err bg-err/10" : snap.link.kind === "sim" ? "border-ref/30 text-ref bg-ref/10" : "border-ok/30 text-ok bg-ok/10"
        )}
        title={snap.link_error ?? snap.link.detail}
      >
        <span className="h-1.5 w-1.5 rounded-full bg-current" />
        {snap.link.kind === "sim" ? "Симулятор" : snap.link.name}
      </button>

      <Toggle
        checked={snap.auto_apply}
        onChange={(on) => call("set_auto_apply", { on })}
        label={<span className="hidden xl:inline whitespace-nowrap">Авто-запись</span>}
        title="Автоматически записывать изменённые регистры в кристалл"
      />
      <Button
        variant={dirty ? "primary" : "subtle"}
        size="sm"
        icon={<Send size={13} />}
        onClick={() => call("apply")}
        title="Записать изменённые регистры"
        disabled={!dirty}
      >
        Записать{dirty ? ` (${dirty})` : ""}
      </Button>

      <div className="flex items-center gap-1">
        <Button size="sm" variant="ghost" icon={<RotateCcw size={14} />} onClick={() => call("chip_reset")} title="Сброс кристалла (вывод RESET, если есть)" />
        <Button size="sm" variant="ghost" icon={<SquareTerminal size={14} />} active={consoleOpen} onClick={() => setConsole(!consoleOpen)} title="Журнал шины (⌘/Ctrl + J)" />
        <Button
          size="sm"
          variant="ghost"
          icon={theme === "dark" ? <Sun size={14} /> : <Moon size={14} />}
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          title="Тема оформления"
        />
      </div>
    </header>
  );
}
