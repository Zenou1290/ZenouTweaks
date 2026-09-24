import { useEffect, useMemo, useState } from "react";
import { RiskBadge, StateBadge } from "@/components/badges";
import { ShieldCheck, RefreshCw } from "lucide-react";
import type { TweakView } from "@/types";

export interface ConfirmableTweak {
  meta: TweakView["meta"];
  state: TweakView["state"];
}

/**
 * Modal previewing exactly which tweaks a bulk action will change,
 * with per-tweak deselect. Never applies anything itself.
 */
export function ConfirmDialog({
  open,
  title,
  confirmLabel,
  tweaks,
  onClose,
  onConfirm,
}: {
  open: boolean;
  title: string;
  confirmLabel: string;
  tweaks: ConfirmableTweak[];
  onClose: () => void;
  onConfirm: (ids: string[]) => void | Promise<void>;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  const ids = useMemo(() => tweaks.map((t) => t.meta.id), [tweaks]);

  useEffect(() => {
    if (open) setSelected(new Set(ids));
  }, [open, ids]);

  if (!open) return null;

  const needsRestart = tweaks.some(
    (t) => selected.has(t.meta.id) && t.meta.restartRequired,
  );
  const anyAdmin = tweaks.some((t) => selected.has(t.meta.id) && t.meta.requiresAdmin);

  function toggle(id: string) {
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function confirm() {
    setBusy(true);
    try {
      await onConfirm([...selected]);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />
      <div className="relative flex max-h-[80vh] w-full max-w-lg flex-col rounded-xl border border-border bg-panel shadow-xl">
        <header className="border-b border-border px-5 py-4">
          <h2 className="text-base font-semibold text-foreground">{title}</h2>
          <p className="mt-1 text-xs text-muted">
            Review what will change. Uncheck anything you want to skip.
          </p>
        </header>

        <div className="flex-1 space-y-1.5 overflow-y-auto px-5 py-4">
          {tweaks.length === 0 && (
            <p className="text-sm text-muted">Nothing applicable to apply.</p>
          )}
          {tweaks.map((t) => (
            <label
              key={t.meta.id}
              className="flex cursor-pointer items-center gap-3 rounded-md border border-border bg-surface px-3 py-2 hover:border-teal-900/50"
            >
              <input
                type="checkbox"
                checked={selected.has(t.meta.id)}
                onChange={() => toggle(t.meta.id)}
                className="h-3.5 w-3.5 accent-teal-500"
              />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm text-foreground">{t.meta.name}</p>
                <p className="truncate text-xs text-muted">{t.meta.description}</p>
              </div>
              <RiskBadge risk={t.meta.risk} />
              {t.state === "partial" && <StateBadge state="partial" />}
            </label>
          ))}
        </div>

        <div className="space-y-2 border-t border-border px-5 py-4">
          {(needsRestart || anyAdmin) && (
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-amber-300/90">
              {anyAdmin && (
                <span className="flex items-center gap-1">
                  <ShieldCheck className="h-3.5 w-3.5" /> Administrator permission will be
                  requested
                </span>
              )}
              {needsRestart && (
                <span className="flex items-center gap-1">
                  <RefreshCw className="h-3.5 w-3.5" /> Restart required for some tweaks
                </span>
              )}
            </div>
          )}
          <div className="flex items-center justify-end gap-2">
            <span className="mr-auto text-xs text-muted">
              {selected.size} of {ids.length} selected
            </span>
            <button
              type="button"
              onClick={onClose}
              className="rounded-md border border-border px-3 py-2 text-sm text-muted hover:text-foreground"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={confirm}
              disabled={selected.size === 0 || busy}
              className="rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
            >
              {busy ? "Applying…" : confirmLabel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
