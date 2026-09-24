import { useEffect, useMemo, useState } from "react";
import { api, toMessage } from "@/lib/api";
import { ErrorBanner, DetailsSection } from "@/components/details-section";
import { Search, Download, Trash2, ScrollText } from "lucide-react";
import { cn } from "@/lib/utils";
import type { LogEntry } from "@/types";

const FILTERS = ["all", "success", "failed", "skipped", "warning"] as const;

function resultTone(result: string): string {
  switch (result) {
    case "success":
      return "text-teal-300";
    case "failed":
    case "error":
      return "text-red-300";
    case "skipped":
      return "text-amber-300";
    default:
      return "text-muted";
  }
}

export function LogsPage() {
  const [logs, setLogs] = useState<LogEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<(typeof FILTERS)[number]>("all");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    try {
      setLogs(await api.readLogs());
    } catch (err) {
      setError(toMessage(err).message);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (logs ?? []).filter((l) => {
      if (filter !== "all" && !l.result.toLowerCase().includes(filter)) return false;
      if (
        q &&
        !`${l.action} ${l.tweakName ?? ""} ${l.message}`.toLowerCase().includes(q)
      )
        return false;
      return true;
    });
  }, [logs, query, filter]);

  async function exportLogs() {
    setBusy(true);
    try {
      const target = await api.pickSaveFile("zenou-tweaks-logs.json");
      if (target) {
        await api.exportLogs(target);
      }
    } catch (err) {
      setError(toMessage(err).message);
    } finally {
      setBusy(false);
    }
  }

  async function clearLogs() {
    setBusy(true);
    try {
      await api.clearLogs();
      setLogs([]);
    } catch (err) {
      setError(toMessage(err).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-foreground">Logs</h1>
          <p className="text-sm text-muted">Every operation Zenou performed, newest first.</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void exportLogs()}
            disabled={busy}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-muted hover:text-foreground disabled:opacity-50"
          >
            <Download className="h-4 w-4" /> Export
          </button>
          <button
            type="button"
            onClick={() => void clearLogs()}
            disabled={busy}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-muted hover:border-red-900/60 hover:text-red-300 disabled:opacity-50"
          >
            <Trash2 className="h-4 w-4" /> Clear
          </button>
        </div>
      </div>

      {error && <ErrorBanner message={error} onDismiss={() => setError(null)} />}

      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-56 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted/60" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search logs…"
            className="w-full rounded-md border border-border bg-surface py-2 pl-8 pr-3 text-sm text-foreground placeholder:text-muted/60 focus:border-teal-800 focus:outline-none"
          />
        </div>
        <div className="flex rounded-md border border-border p-0.5">
          {FILTERS.map((f) => (
            <button
              key={f}
              type="button"
              onClick={() => setFilter(f)}
              className={cn(
                "rounded px-2.5 py-1 text-xs capitalize transition-colors",
                filter === f ? "bg-teal-950/70 text-teal-200" : "text-muted hover:text-foreground",
              )}
            >
              {f}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-1.5">
        {logs === null && !error ? (
          [...Array(6)].map((_, i) => (
            <div key={i} className="h-14 animate-pulse rounded-lg bg-surface" />
          ))
        ) : shown.length === 0 ? (
          <div className="rounded-xl border border-border bg-panel px-4 py-10 text-center">
            <ScrollText className="mx-auto h-8 w-8 text-muted/40" />
            <p className="mt-2 text-sm text-muted">
              {logs === null || logs.length === 0 ? "No log entries yet." : "No matches."}
            </p>
          </div>
        ) : (
          shown.map((l, i) => (
            <div
              key={`${l.timestamp}-${i}`}
              className="rounded-lg border border-border bg-panel px-4 py-2.5"
            >
              <div className="flex items-center gap-3 text-xs">
                <span className="shrink-0 font-mono text-muted/70">
                  {new Date(l.timestamp).toLocaleString(undefined, {
                    month: "short",
                    day: "numeric",
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  })}
                </span>
                <span className={cn("shrink-0 font-medium capitalize", resultTone(l.result))}>
                  {l.result}
                </span>
                <span className="shrink-0 text-muted">{l.action}</span>
                <span className="truncate text-foreground">
                  {l.tweakName ?? "—"}
                </span>
              </div>
              {l.message && l.message !== l.tweakName && (
                <p className="mt-1 truncate pl-1 text-xs text-muted/80">{l.message}</p>
              )}
              {l.errorDetails && (
                <DetailsSection className="mt-1.5" title="Error details">
                  {l.errorDetails}
                </DetailsSection>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
