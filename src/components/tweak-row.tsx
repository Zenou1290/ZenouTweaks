import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { api, toMessage } from "@/lib/api";
import { useUi } from "@/store/ui";
import { RiskBadge, StateBadge } from "@/components/badges";
import { DetailsSection } from "@/components/details-section";
import {
  Loader2,
  RotateCcw,
  Star,
  Undo2,
  ChevronRight,
  ShieldCheck,
  RefreshCw,
} from "lucide-react";
import type { TweakView } from "@/types";

function StateDot({ state }: { state: string }) {
  const color =
    state === "applied"
      ? "bg-teal-400"
      : state === "partial"
        ? "bg-amber-400"
        : state === "failed"
          ? "bg-red-400"
          : state === "unsupported"
            ? "bg-zinc-600"
            : "bg-zinc-500";
  return <span className={cn("inline-block h-2 w-2 rounded-full", color)} />;
}

export function TweakRow({ tweak }: { tweak: TweakView }) {
  const { busy, setSelectedTweak, toggleFavorite, setTweakState, setBusy, addRestartPending, settings } =
    useUi();
  const [error, setError] = useState<string | null>(null);
  const [details, setDetails] = useState<string | null>(null);
  const op = busy[tweak.meta.id];

  async function apply() {
    setError(null);
    setDetails(null);
    setBusy(tweak.meta.id, "applying");
    try {
      const res = await api.applyTweak(tweak.meta.id);
      if (res.success) {
        setTweakState(tweak.meta.id, "applied");
        if (res.restartRequired) addRestartPending(tweak.meta.id);
        if (settings.notifications)
          console.info(`${tweak.meta.name} applied${res.restartRequired ? " (restart required)" : ""}`);
      } else if (res.skipped) {
        setTweakState(tweak.meta.id, "available");
        setError(res.error ?? "Skipped: not applicable to this system.");
      } else {
        setTweakState(tweak.meta.id, "unknown");
        setError(res.error ?? "The tweak failed to apply.");
        setDetails(res.details);
      }
    } catch (err) {
      setTweakState(tweak.meta.id, "unknown");
      setError(toMessage(err).message);
      setDetails(toMessage(err).details);
    } finally {
      setBusy(tweak.meta.id, null);
    }
  }

  async function restore() {
    setError(null);
    setDetails(null);
    setBusy(tweak.meta.id, "restoring");
    try {
      const res = await api.restoreTweak(tweak.meta.id);
      if (res.success) {
        setTweakState(tweak.meta.id, "available");
      } else {
        setTweakState(tweak.meta.id, "unknown");
        setError(res.error ?? "The tweak could not be restored.");
        setDetails(res.details);
      }
    } catch (err) {
      setTweakState(tweak.meta.id, "unknown");
      setError(toMessage(err).message);
      setDetails(toMessage(err).details);
    } finally {
      setBusy(tweak.meta.id, null);
    }
  }

  const unsupported = tweak.state === "unsupported";

  return (
    <div
      className={cn(
        "rounded-lg border border-border bg-panel px-4 py-3 transition-colors",
        !unsupported && "hover:border-teal-900/50",
        unsupported && "opacity-60",
      )}
    >
      <div className="flex items-center gap-3">
        <button
          type="button"
          onClick={() => toggleFavorite(tweak.meta.id)}
          className="shrink-0 text-muted/50 hover:text-teal-300"
          title={tweak.favorite ? "Remove from favorites" : "Add to favorites"}
        >
          <Star className={cn("h-4 w-4", tweak.favorite && "fill-teal-400 text-teal-400")} />
        </button>

        <button
          type="button"
          onClick={() => setSelectedTweak(tweak.meta.id)}
          className="min-w-0 flex-1 text-left"
        >
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-medium text-foreground">
              {tweak.meta.name}
            </span>
            <RiskBadge risk={tweak.meta.risk} />
            {tweak.meta.requiresAdmin && (
              <span
                className="inline-flex items-center gap-0.5 text-[10px] uppercase text-muted/70"
                title="Requires administrator permission"
              >
                <ShieldCheck className="h-3 w-3" /> admin
              </span>
            )}
            {tweak.meta.restartRequired && (
              <span className="text-[10px] uppercase text-muted/70" title="Restart required to take effect">
                <RefreshCw className="h-3 w-3" />
              </span>
            )}
          </div>
          <p className="mt-0.5 truncate text-xs text-muted">{tweak.meta.description}</p>
        </button>

        <div className="flex shrink-0 items-center gap-2">
          <span className="flex items-center gap-1.5 text-xs text-muted">
            <StateDot state={tweak.state} />
            <StateBadge state={tweak.state} />
          </span>
          {op ? (
            <span className="flex w-24 items-center justify-center gap-1.5 text-xs text-teal-300">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              {op === "applying" ? "Applying…" : "Restoring…"}
            </span>
          ) : unsupported ? (
            <span className="w-24 text-center text-xs text-muted/60">N/A</span>
          ) : (
            <>
              {tweak.state === "applied" || tweak.state === "partial" ? (
                <button
                  type="button"
                  onClick={restore}
                  className="flex w-24 items-center justify-center gap-1.5 rounded-md border border-border px-2 py-1.5 text-xs text-muted hover:border-teal-900/60 hover:text-foreground transition-colors"
                >
                  <Undo2 className="h-3.5 w-3.5" /> Restore
                </button>
              ) : (
                <button
                  type="button"
                  onClick={apply}
                  className="flex w-24 items-center justify-center gap-1.5 rounded-md bg-teal-600 px-2 py-1.5 text-xs font-medium text-white hover:bg-teal-500 transition-colors"
                >
                  Apply
                </button>
              )}
            </>
          )}
          <button
            type="button"
            onClick={() => setSelectedTweak(tweak.meta.id)}
            className="text-muted/50 hover:text-foreground"
            title="Details"
          >
            <ChevronRight className="h-4 w-4" />
          </button>
        </div>
      </div>

      {error && (
        <div className="mt-2">
          <p className="text-xs text-red-300">{error}</p>
          {details && (
            <DetailsSection className="mt-1" title="Technical details">
              {details}
            </DetailsSection>
          )}
        </div>
      )}
    </div>
  );
}

export function TweakDetailsDrawer() {
  const { selectedTweakId, setSelectedTweak, tweaks, setTweakState, setBusy, busy } = useUi();
  const [full, setFull] = useState<TweakView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [details, setDetails] = useState<string | null>(null);

  const selected = tweaks.find((t) => t.meta.id === selectedTweakId) ?? null;

  useEffect(() => {
    if (!selectedTweakId) {
      setFull(null);
      return;
    }
    setError(null);
    setDetails(null);
    api
      .tweakDetails(selectedTweakId)
      .then(setFull)
      .catch((err) => {
        const m = toMessage(err);
        setError(m.message);
        setDetails(m.details);
      });
  }, [selectedTweakId]);

  if (!selectedTweakId) return null;

  const tv = full ?? selected;
  const op = busy[selectedTweakId];

  async function apply() {
    if (!selectedTweakId) return;
    setBusy(selectedTweakId, "applying");
    try {
      const res = await api.applyTweak(selectedTweakId);
      setTweakState(selectedTweakId, res.success ? "applied" : res.skipped ? "available" : "unknown");
      if (!res.success) {
        setError(res.error ?? "The tweak failed to apply.");
        setDetails(res.details);
      }
      const fresh = await api.tweakDetails(selectedTweakId);
      setFull(fresh);
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    } finally {
      setBusy(selectedTweakId, null);
    }
  }

  async function restore() {
    if (!selectedTweakId) return;
    setBusy(selectedTweakId, "restoring");
    try {
      const res = await api.restoreTweak(selectedTweakId);
      setTweakState(selectedTweakId, res.success ? "available" : "unknown");
      if (!res.success) {
        setError(res.error ?? "The tweak could not be restored.");
        setDetails(res.details);
      }
      const fresh = await api.tweakDetails(selectedTweakId);
      setFull(fresh);
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
      setDetails(m.details);
    } finally {
      setBusy(selectedTweakId, null);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex">
      <div
        className="absolute inset-0 bg-black/60"
        onClick={() => setSelectedTweak(null)}
      />
      <aside className="relative ml-auto flex h-full w-full max-w-md flex-col border-l border-border bg-panel shadow-xl">
        <header className="flex items-start justify-between gap-3 border-b border-border px-5 py-4">
          <div className="min-w-0">
            <h2 className="text-base font-semibold text-foreground">
              {tv?.meta.name ?? selected?.meta.name}
            </h2>
            <p className="mt-1 text-xs text-muted">
              {tv?.meta.description ?? selected?.meta.description}
            </p>
            <div className="mt-2 flex items-center gap-2">
              <RiskBadge risk={tv?.meta.risk ?? selected?.meta.risk ?? "medium"} />
              <StateBadge state={tv?.state ?? selected?.state ?? "unknown"} />
            </div>
          </div>
          <button
            type="button"
            onClick={() => setSelectedTweak(null)}
            className="rounded p-1 text-muted hover:text-foreground"
          >
            ✕
          </button>
        </header>

        <div className="flex-1 space-y-4 overflow-y-auto px-5 py-4">
          {error && (
            <div>
              <p className="text-sm text-red-300">{error}</p>
              {details && <DetailsSection className="mt-1">{details}</DetailsSection>}
            </div>
          )}

          <section>
            <h3 className="text-xs font-semibold uppercase tracking-wide text-muted">
              What this does
            </h3>
            <p className="mt-1 text-sm text-foreground">
              {tv?.meta.effect ?? selected?.meta.effect ?? "—"}
            </p>
          </section>

          <section>
            <h3 className="text-xs font-semibold uppercase tracking-wide text-muted">
              Requirements
            </h3>
            <ul className="mt-1 space-y-1 text-xs text-muted">
              <li className="flex items-center gap-2">
                <ShieldCheck className="h-3.5 w-3.5" />
                {(tv?.meta.requiresAdmin ?? selected?.meta.requiresAdmin)
                  ? "Administrator permission required"
                  : "No administrator permission required"}
              </li>
              <li className="flex items-center gap-2">
                <RefreshCw className="h-3.5 w-3.5" />
                {(tv?.meta.restartRequired ?? selected?.meta.restartRequired)
                  ? "Windows restart required for full effect"
                  : "Takes effect immediately (or on next app launch)"}
              </li>
            </ul>
          </section>

          <section>
            <h3 className="text-xs font-semibold uppercase tracking-wide text-muted">
              Changes preview
            </h3>
            {tv && tv.previews.length > 0 ? (
              <div className="mt-2 space-y-2">
                {tv.previews.map((p, i) => (
                  <div
                    key={i}
                    className="rounded-md border border-border bg-surface px-3 py-2 font-mono text-xs"
                  >
                    <p className="break-all text-muted/80">{p.location}</p>
                    {p.valueName && <p className="break-all text-muted/60">{p.valueName}</p>}
                    <div className="mt-1 flex flex-col gap-0.5">
                      <span className="text-red-300/80">− {p.current}</span>
                      <span className="text-teal-300">+ {p.new}</span>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="mt-1 text-xs text-muted">
                {tv?.previews.length === 0 && tv.state === "unsupported"
                  ? "Not applicable to this hardware."
                  : "No change preview available."}
              </p>
            )}
          </section>
        </div>

        <footer className="flex items-center gap-2 border-t border-border px-5 py-4">
          {op ? (
            <span className="flex flex-1 items-center justify-center gap-2 text-sm text-teal-300">
              <Loader2 className="h-4 w-4 animate-spin" />
              {op === "applying" ? "Applying…" : "Restoring…"}
            </span>
          ) : (
            <>
              <button
                type="button"
                onClick={restore}
                className="flex flex-1 items-center justify-center gap-2 rounded-md border border-border px-3 py-2 text-sm text-muted hover:border-teal-900/60 hover:text-foreground transition-colors"
              >
                <RotateCcw className="h-4 w-4" /> Restore
              </button>
              <button
                type="button"
                onClick={apply}
                className="flex flex-1 items-center justify-center gap-2 rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 transition-colors"
              >
                Apply
              </button>
            </>
          )}
        </footer>
      </aside>
    </div>
  );
}
