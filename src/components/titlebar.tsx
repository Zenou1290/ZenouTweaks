import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUi } from "@/store/ui";
import { ZenouMark } from "@/components/zenou-mark";
import { ShieldCheck, Minus, X } from "lucide-react";

const appWindow = getCurrentWindow();

export function TitleBar() {
  const { scan } = useUi();

  return (
    <header
      data-tauri-drag-region
      className="flex h-10 shrink-0 items-center gap-2 border-b border-subtle bg-surface px-3"
    >
      <ZenouMark className="h-5 w-5 rounded" />
      <span className="text-sm font-semibold tracking-tight text-foreground">Zenou Tweaks</span>
      <span className="rounded border border-subtle px-1 py-0.5 text-[10px] text-muted">
        v1.0.0
      </span>
      {scan?.isElevated && (
        <span className="flex items-center gap-1 rounded border border-amber-800/60 bg-amber-950/40 px-1.5 py-0.5 text-[10px] text-amber-300">
          <ShieldCheck className="h-3 w-3" /> Administrator
        </span>
      )}
      <div className="ml-auto flex items-center gap-1" data-tauri-drag-region>
        <button
          type="button"
          onClick={() => void appWindow.hide()}
          className="rounded p-1.5 text-muted hover:bg-elevated hover:text-foreground"
          title="Minimize to tray"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={() => void appWindow.close()}
          className="rounded p-1.5 text-muted hover:bg-red-900/40 hover:text-red-200"
          title="Close (minimizes to tray when enabled)"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}
