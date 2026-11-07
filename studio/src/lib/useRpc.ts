import { useEffect, useRef, useState } from "react";
import { rpc } from "./api";

/**
 * Read-only analysis RPC (plan, spectrum, response …) recomputed when `key`
 * changes. Keeps the last good result while a new one is computed; debounced so
 * typing does not flood the engine.
 */
export function useRpcData<T>(cmd: string, args: Record<string, unknown>, key: string, delay = 120) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const seq = useRef(0);
  const argsRef = useRef(args);
  argsRef.current = args;

  useEffect(() => {
    const my = ++seq.current;
    setBusy(true);
    const id = setTimeout(async () => {
      try {
        const r = await rpc<T>(cmd, argsRef.current);
        if (my === seq.current) {
          setData(r);
          setError(null);
        }
      } catch (e) {
        if (my === seq.current) setError(String(e).replace(/^Error:\s*/, ""));
      } finally {
        if (my === seq.current) setBusy(false);
      }
    }, delay);
    return () => clearTimeout(id);
  }, [cmd, key, delay]);

  return { data, error, busy };
}

/** Stable key of the chip configuration + board settings for analysis hooks. */
export function configKey(regs: number[] | undefined, settings: unknown): string {
  if (!regs) return "";
  let h = 0;
  for (let i = 0; i < regs.length; i++) h = (Math.imul(h, 31) + regs[i]) | 0;
  return `${h}:${JSON.stringify(settings)}`;
}
