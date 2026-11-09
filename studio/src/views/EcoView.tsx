import { useEffect, useRef } from "react";
import { GitBranch } from "lucide-react";
import { useStore } from "../lib/store";
import { PageHeader } from "../components/ui";

// Справочник по ревизиям платы: самостоятельная страница из public/eco/,
// встроенная в приложение. Тема передаётся в неё при загрузке и при смене.
export function EcoView() {
  const theme = useStore((s) => s.theme);
  const ref = useRef<HTMLIFrameElement>(null);

  const applyTheme = () => {
    const doc = ref.current?.contentDocument;
    if (doc) doc.documentElement.dataset.theme = theme;
  };
  useEffect(applyTheme, [theme]);

  return (
    <div className="flex flex-col h-full min-h-[640px]">
      <PageHeader
        icon={<GitBranch size={20} />}
        title="Ревизии платы"
        subtitle="Как вносить исправления в плату DDS 48–52: мастер изменения T1–T8, режимы, ворота, роли и грабли."
      />
      <iframe
        ref={ref}
        src="eco/index.html"
        title="Ревизии платы"
        onLoad={applyTheme}
        className="flex-1 w-full rounded-xl border border-line bg-panel"
        style={{ minHeight: 600 }}
      />
    </div>
  );
}
