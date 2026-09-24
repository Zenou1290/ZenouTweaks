import { useEffect, useState } from "react";
import { api, toMessage } from "@/lib/api";
import { useUi } from "@/store/ui";
import { ErrorBanner } from "@/components/details-section";
import { TweakResultRow } from "@/components/tweak-result-row";
import { Undo2, Loader2, History, RotateCcw } from "lucide-react";
import type { ChangeSet, ChangeSetSummary } from "@/types";

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function ChangesPage() {
  const [summaries, setSummaries] = useState<ChangeSetSummary[] | null>(null);
  const [openSet, setOpenSet] = useState<ChangeSet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [details, setDetails] = useState<string | null>(null);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [restoredMsg, setRestoredMsg] = useState<string | null>(null);
  const { loadTweaks } = useUi();

  useEffect(() => {
    api
      .listChanges()
      .then(setSummaries)
      .catch((err) => {
        const m = toMessage(err);
        setError(m.message);
        setDetails(m.details);
      });
  }, []);

  async function openChange(id: string) {
    setError(null);
    setRestoredMsg(null);
    try {
      setOpenSet(await api.getChange(id));
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    }
  }

  async function restoreSet(id: string) {
    setRestoring(id);
    setError(null);
    setDetails(null);
    try {
      const res = await api.restoreChangeSet(id);
      const ok = res.results.filter((r) => r.success).length;
      setRestoredMsg(
        `Restored ${ok} of ${res.results.length} tweaks from this change set.`,
      );
      setOpenSet(null);
      await loadTweaks();
      setSummaries(await api.listChanges());
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    } finally {
      setRestoring(null);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div>
        <h1 className="text-lg font-semibold text-foreground">What Changed?</h1>
        <p className="text-sm text-muted">
          A timeline of every modification Zenou made, with full restoration support.
        </p>
      </div>

      {error && <ErrorBanner message={error} details={details} onDismiss={() => setError(null)} />}
      {restoredMsg && (
        <div className="rounded-lg border border-teal-900/50 bg-teal-950/20 px-4 py-3 text-sm text-teal-200">
          {restoredMsg}
        </div>
      )}

      {summaries === null && !error ? (
        [...Array(3)].map((_, i) => (
          <div key={i} className="h-20 animate-pulse rounded-lg bg-surface" />
        ))
      ) : (summaries ?? []).length === 0 ? (
        <div className="rounded-xl border border-border bg-panel px-4 py-10 text-center">
          <History className="mx-auto h-8 w-8 text-muted/40" />
          <p className="mt-2 text-sm text-muted">No changes yet.</p>
          <p className="text-xs text-muted/70">
            Tweak applications will appear here as a reviewable timeline.
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {(summaries ?? []).map((cs) => (
            <div key={cs.id} className="rounded-xl border border-border bg-panel p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-sm font-medium text-foreground">{cs.label}</p>
                  <p className="mt-0.5 text-xs text-muted">{formatDate(cs.createdAt)}</p>
                  <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs">
                    <span className="text-teal-300">{cs.applied} successful</span>
                    {cs.failed > 0 && <span className="text-red-300">{cs.failed} failed</span>}
                    {cs.skipped > 0 && <span className="text-amber-300">{cs.skipped} skipped</span>}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <button
                    type="button"
                    onClick={() => void openChange(cs.id)}
                    className="rounded-md border border-border px-2.5 py-1.5 text-xs text-muted hover:text-foreground"
                  >
                    Inspect
                  </button>
                  <button
                    type="button"
                    onClick={() => void restoreSet(cs.id)}
                    disabled={restoring != null}
                    className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted hover:border-teal-900/60 hover:text-foreground disabled:opacity-50"
                  >
                    {restoring === cs.id ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Undo2 className="h-3.5 w-3.5" />
                    )}
                    Restore all
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {openSet && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
          <div className="absolute inset-0 bg-black/60" onClick={() => setOpenSet(null)} />
          <div className="relative flex max-h-[80vh] w-full max-w-2xl flex-col rounded-xl border border-border bg-panel shadow-xl">
            <header className="flex items-center justify-between border-b border-border px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-foreground">{openSet.label}</h2>
                <p className="mt-0.5 text-xs text-muted">{formatDate(openSet.createdAt)}</p>
              </div>
              <button
                type="button"
                onClick={() => setOpenSet(null)}
                className="rounded p-1 text-muted hover:text-foreground"
              >
                ✕
              </button>
            </header>
            <div className="flex-1 space-y-2 overflow-y-auto px-5 py-4">
              {openSet.results.map((r) => (
                <TweakResultRow key={r.tweakId} result={r} />
              ))}
            </div>
            <footer className="flex items-center justify-between border-t border-border px-5 py-3">
              <span className="text-xs text-muted">
                {openSet.backupId
                  ? "Pre-change backup recorded — restoration uses recorded values only."
                  : "No backup was recorded for this change set."}
              </span>
              <button
                type="button"
                onClick={() => void restoreSet(openSet.id)}
                disabled={restoring != null}
                className="flex items-center gap-1.5 rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
              >
                {restoring === openSet.id ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <RotateCcw className="h-4 w-4" />
                )}
                Restore all
              </button>
            </footer>
          </div>
        </div>
      )}
    </div>
  );
}
