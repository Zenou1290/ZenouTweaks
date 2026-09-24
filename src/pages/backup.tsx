import { useEffect, useState } from "react";
import { api, toMessage } from "@/lib/api";
import { useUi } from "@/store/ui";
import { ErrorBanner } from "@/components/details-section";
import { TweakResultRow } from "@/components/tweak-result-row";
import { Save, Trash2, Loader2, HardDriveDownload, ShieldCheck } from "lucide-react";
import type { Snapshot, GroupResult } from "@/types";

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function BackupPage() {
  const { loadTweaks } = useUi();
  const [backups, setBackups] = useState<Snapshot[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [details, setDetails] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [label, setLabel] = useState("");
  const [restoring, setRestoring] = useState<string | null>(null);
  const [restoreResult, setRestoreResult] = useState<GroupResult | null>(null);

  async function refresh() {
    try {
      setBackups(await api.listBackups());
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function createBackup() {
    setCreating(true);
    setError(null);
    setDetails(null);
    try {
      await api.createBackup(label.trim() || "Manual backup");
      setLabel("");
      await refresh();
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    } finally {
      setCreating(false);
    }
  }

  async function restore(id: string) {
    setRestoring(id);
    setError(null);
    setDetails(null);
    setRestoreResult(null);
    try {
      const res = await api.restoreBackup(id);
      setRestoreResult(res);
      await loadTweaks();
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    } finally {
      setRestoring(null);
    }
  }

  async function remove(id: string) {
    try {
      await api.deleteBackup(id);
      await refresh();
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div>
        <h1 className="text-lg font-semibold text-foreground">Backup & Restore</h1>
        <p className="text-sm text-muted">
          Snapshots of every setting Zenou recorded, stored locally in the app data directory.
        </p>
      </div>

      {error && <ErrorBanner message={error} details={details} onDismiss={() => setError(null)} />}

      {/* Create */}
      <section className="rounded-xl border border-border bg-panel p-4">
        <h2 className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <Save className="h-4 w-4 text-teal-400" />
          Create backup
        </h2>
        <p className="mt-1 text-xs text-muted">
          Captures the current value of every registry value, service, task, and power setting
          Zenou knows about. Automatic backups are also made before risky changes.
        </p>
        <div className="mt-3 flex items-center gap-2">
          <input
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="Label (optional)"
            className="min-w-0 flex-1 rounded-md border border-border bg-surface px-3 py-2 text-sm text-foreground placeholder:text-muted/60 focus:border-teal-800 focus:outline-none"
          />
          <button
            type="button"
            onClick={() => void createBackup()}
            disabled={creating}
            className="flex items-center gap-1.5 rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
          >
            {creating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
            {creating ? "Creating…" : "Create backup"}
          </button>
        </div>
      </section>

      {/* Restore result */}
      {restoreResult && (
        <section className="rounded-xl border border-teal-900/50 bg-teal-950/20 p-4">
          <h3 className="text-sm font-medium text-teal-200">
            Restore finished — {restoreResult.results.filter((r) => r.success).length} of{" "}
            {restoreResult.results.length} tweaks restored.
          </h3>
          <div className="mt-3 max-h-72 space-y-2 overflow-y-auto">
            {restoreResult.results.map((r) => (
              <TweakResultRow key={r.tweakId} result={r} />
            ))}
          </div>
        </section>
      )}

      {/* List */}
      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-foreground">Existing backups</h2>
        {backups === null ? (
          [...Array(3)].map((_, i) => (
            <div key={i} className="h-16 animate-pulse rounded-lg bg-surface" />
          ))
        ) : backups.length === 0 ? (
          <div className="rounded-xl border border-border bg-panel px-4 py-10 text-center">
            <HardDriveDownload className="mx-auto h-8 w-8 text-muted/40" />
            <p className="mt-2 text-sm text-muted">No backups yet.</p>
          </div>
        ) : (
          backups.map((b) => (
            <div
              key={b.id}
              className="flex items-center gap-4 rounded-lg border border-border bg-panel px-4 py-3"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <p className="truncate text-sm font-medium text-foreground">{b.label}</p>
                  {b.automatic && (
                    <span className="rounded border border-border px-1 py-0.5 text-[10px] uppercase text-muted">
                      auto
                    </span>
                  )}
                </div>
                <p className="text-xs text-muted">
                  {formatDate(b.createdAt)} · {b.entries.length} setting
                  {b.entries.length === 1 ? "" : "s"} captured
                </p>
              </div>
              <button
                type="button"
                onClick={() => void restore(b.id)}
                disabled={restoring != null}
                className="flex shrink-0 items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-muted hover:border-teal-900/60 hover:text-foreground disabled:opacity-50"
              >
                {restoring === b.id ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <ShieldCheck className="h-3.5 w-3.5" />
                )}
                Restore
              </button>
              <button
                type="button"
                onClick={() => void remove(b.id)}
                disabled={restoring != null}
                className="shrink-0 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted hover:border-red-900/60 hover:text-red-300 disabled:opacity-50"
                title="Delete backup"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
          ))
        )}
      </section>
    </div>
  );
}
