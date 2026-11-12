// One RPC: POST /api/rpc → dds-server (the same Rust Engine over HTTP).
// The native desktop app (crates/dds-egui) calls the Engine directly.

export async function rpc<T = unknown>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const r = await fetch("/api/rpc", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ cmd, args }),
  });
  if (!r.ok) throw new Error(`HTTP ${r.status} — запущен ли dds-server?`);
  const j = await r.json();
  if (!j.ok) throw new Error(j.error);
  return j.data as T;
}

/** Save a text file: the browser downloads it. */
export async function saveTextFile(defaultName: string, text: string): Promise<string | null> {
  const blob = new Blob([text], { type: "text/plain" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = defaultName;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  return defaultName;
}

/** Open a text file chosen with the browser's file picker. */
export async function openTextFile(): Promise<string | null> {
  return new Promise((resolve) => {
    const inp = document.createElement("input");
    inp.type = "file";
    inp.accept = ".txt,.csv,.h,.json";
    inp.onchange = async () => {
      const f = inp.files?.[0];
      resolve(f ? await f.text() : null);
    };
    inp.click();
  });
}
