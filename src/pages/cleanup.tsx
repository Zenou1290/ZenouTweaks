import { useEffect, useState } from "react";
import { useUi } from "@/store/ui";
import { api, toMessage } from "@/lib/api";
import { ErrorBanner, DetailsSection } from "@/components/details-section";
import { cn } from "@/lib/utils";
import { Brush, Loader2, ShieldAlert } from "lucide-react";
import type { CleanupPreview } from "@/types";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function CleanupPage() {
  const [previews, setPreviews] = useState<CleanupPreview[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  const [done, setDone] = useState<{ id: string; files: number; bytes: number } | null>(null);
  const [details, setDetails] = useState<string | null>(null);
  const { settings } = useUi();

  useEffect(() => {
    api
      .cleanupPreviews()
      .then(setPreviews)
      .catch((err) => {
        const m = toMessage(err);
        setError(m.message);
        setDetails(m.details);
      });
  }, []);

  async function run(id: string) {
    setRunning(id);
    setError(null);
    setDetails(null);
    setDone(null);
    try {
      const [files, bytes] = await api.runCleanup(id);
      setDone({ id, files, bytes });
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    } finally {
      setRunning(null);
      // Refresh size estimates
      api
        .cleanupPreviews()
        .then(setPreviews)
        .catch(() => {});
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div>
        <h1 className="text-lg font-semibold text-foreground">Cleanup</h1>
        <p className="text-sm text-muted">
          Safe junk removal with exact size estimates. Each action is independent and reviewed.
        </p>
      </div>

      {error && <ErrorBanner message={error} details={details} onDismiss={() => setError(null)} />}

      {previews === null && !error ? (
        [...Array(4)].map((_, i) => (
          <div key={i} className="h-20 animate-pulse rounded-lg bg-surface" />
        ))
      ) : (
        <div className="space-y-2">
          {(previews ?? []).map((p) => (
            <div
              key={p.id}
              className="flex items-center gap-4 rounded-lg border border-border bg-panel px-4 py-3"
            >
              <Brush className="h-4 w-4 shrink-0 text-teal-400/80" />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <p className="text-sm font-medium text-foreground">{p.name}</p>
                  {p.irreversible && (
                    <span
                      className="flex items-center gap-0.5 text-[10px] uppercase text-amber-300"
                      title="Deleted files cannot be recovered"
                    >
                      <ShieldAlert className="h-3 w-3" /> permanent
                    </span>
                  )}
                </div>
                <p className="truncate text-xs text-muted">{p.description}</p>
              </div>
              <div className="shrink-0 text-right">
                <p className="text-sm font-medium text-foreground">{formatBytes(p.totalBytes)}</p>
                <p className="text-xs text-muted">
                  {p.fileCount} item{p.fileCount === 1 ? "" : "s"}
                </p>
              </div>
              <button
                type="button"
                onClick={() => void run(p.id)}
                disabled={running != null}
                className="shrink-0 rounded-md bg-teal-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-teal-500 disabled:opacity-50"
              >
                {running === p.id ? (
                  <span className="flex items-center gap-1.5">
                    <Loader2 className="h-3.5 w-3.5 animate-spin" /> Cleaning…
                  </span>
                ) : (
                  "Clean"
                )}
              </button>
            </div>
          ))}
        </div>
      )}

      {done && (
        <div
          className={cn(
            "rounded-lg border px-4 py-3 text-sm",
            done.files > 0
              ? "border-teal-900/50 bg-teal-950/20 text-teal-200"
              : "border-border bg-panel text-muted",
          )}
        >
          {done.files > 0
            ? `Cleaned ${done.files} items (${formatBytes(done.bytes)}).`
            : "Nothing to clean — already tidy."}
          {settings.notifications && null}
        </div>
      )}

      {previews && previews.every((p) => p.fileCount === 0) && (
        <DetailsSection title="About cleanup estimates">
          Estimates are computed by enumerating the target directories and may not include
          files locked by running applications.
        </DetailsSection>
      )}
    </div>
  );
}
