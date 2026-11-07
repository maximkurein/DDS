import { useEffect, type ReactNode } from "react";
import { Cpu, LoaderCircle } from "lucide-react";
import { useStore } from "./lib/store";
import { TopBar } from "./components/TopBar";
import { NAV, Sidebar } from "./components/Sidebar";
import { Console } from "./components/Console";
import { Toasts } from "./components/Toasts";
import { OverviewView } from "./views/OverviewView";
import { BoardView } from "./views/BoardView";
import { RegistersView } from "./views/RegistersView";
import { FreqView } from "./views/FreqView";
import { SpectrumView } from "./views/SpectrumView";
import { ChainView } from "./views/ChainView";
import { SweepView } from "./views/SweepView";
import { HopView } from "./views/HopView";
import { ModView } from "./views/ModView";
import { HardwareView } from "./views/HardwareView";
import { LearnView } from "./views/LearnView";
import type { ViewId } from "./lib/types";

const VIEWS: Record<ViewId, () => ReactNode> = {
  overview: () => <OverviewView />,
  board: () => <BoardView />,
  registers: () => <RegistersView />,
  freq: () => <FreqView />,
  spectrum: () => <SpectrumView />,
  chain: () => <ChainView />,
  sweep: () => <SweepView />,
  hop: () => <HopView />,
  mod: () => <ModView />,
  hardware: () => <HardwareView />,
  learn: () => <LearnView />,
};

export default function App() {
  const snap = useStore((s) => s.snap);
  const meta = useStore((s) => s.meta);
  const view = useStore((s) => s.view);
  const connError = useStore((s) => s.connError);

  useEffect(() => {
    void useStore.getState().init();
  }, []);

  // live status polling (simulator state, input pins, new log entries)
  useEffect(() => {
    let busy = false;
    const id = setInterval(async () => {
      if (busy || document.hidden || !useStore.getState().snap) return;
      busy = true;
      await useStore.getState().poll();
      busy = false;
    }, 350);
    return () => clearInterval(id);
  }, []);

  // keyboard shortcuts: ⌘/Ctrl+1…9 views (first nine), ⌘/Ctrl+J bus log
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const n = Number(e.key);
      if (n >= 1 && n <= Math.min(9, NAV.length)) {
        e.preventDefault();
        useStore.getState().go(NAV[n - 1].id);
      } else if (e.key === "`" || e.key === "j") {
        e.preventDefault();
        const s = useStore.getState();
        s.setConsole(!s.consoleOpen);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!snap || !meta) {
    return (
      <div className="h-full grid place-items-center bg-bg">
        <div className="flex flex-col items-center gap-3 text-center px-6">
          <div className="h-14 w-14 rounded-2xl bg-panel border border-line grid place-items-center text-accent">
            <Cpu size={28} />
          </div>
          <div className="text-[16px] font-semibold">DDS Studio · 1508ПЛ8Т</div>
          {connError ? (
            <div className="text-[12.5px] text-muted max-w-md">
              Нет связи с движком: <span className="text-err">{connError}</span>
              <br />
              В веб-режиме запустите <span className="kbd">npm run server</span> — повторное подключение идёт автоматически.
            </div>
          ) : (
            <div className="flex items-center gap-2 text-muted text-[12.5px]">
              <LoaderCircle size={14} className="spin" /> Запуск движка…
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-bg text-fg">
      <TopBar />
      <div className="flex-1 min-h-0 flex">
        <Sidebar />
        <div className="flex-1 min-w-0 flex flex-col">
          <main key={view} className="flex-1 min-h-0 overflow-auto scroll-thin px-6 py-5">
            {VIEWS[view]()}
          </main>
          <Console />
        </div>
      </div>
      <Toasts />
    </div>
  );
}
