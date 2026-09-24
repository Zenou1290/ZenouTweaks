import { useCallback, useEffect, useMemo, useState } from "react";
import { useUi } from "@/store/ui";
import { useBulkApply } from "@/hooks/use-bulk-apply";
import { TweakRow } from "@/components/tweak-row";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { ErrorBanner } from "@/components/details-section";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";
import type { Profile } from "@/types";
import { Gamepad2, Zap } from "lucide-react";

export function CategoryPage({
  category,
  title,
  description,
}: {
  category: string;
  title: string;
  description?: string;
}) {
  const { tweaks, tweaksLoading, tweaksError, settings, busy } = useUi();
  const { run, running, error: runError } = useBulkApply();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [filter, setFilter] = useState<"all" | "available" | "applied">("all");
  const isGaming = category === "gaming";

  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedProfile, setSelectedProfile] = useState<string | null>(null);
  useEffect(() => {
    if (!isGaming) return;
    api
      .listProfiles()
      .then(setProfiles)
      .catch(() => setProfiles([]));
  }, [isGaming]);

  const categoryTweaks = useMemo(
    () => tweaks.filter((t) => t.meta.category === category),
    [tweaks, category],
  );

  const shown = useMemo(() => {
    if (filter === "all") return categoryTweaks;
    if (filter === "available")
      return categoryTweaks.filter((t) => t.state === "available" || t.state === "partial");
    return categoryTweaks.filter((t) => t.state === "applied");
  }, [categoryTweaks, filter]);

  const applicable = useMemo(
    () =>
      categoryTweaks.filter(
        (t) => (t.state === "available" || t.state === "partial") && busy[t.meta.id] == null,
      ),
    [categoryTweaks, busy],
  );

  const profile = profiles.find((p) => p.name === selectedProfile) ?? null;
  const profileTweaks = useMemo(
    () =>
      profile
        ? profile.tweakIds
            .map((id) => tweaks.find((t) => t.meta.id === id))
            .filter(
              (t): t is NonNullable<typeof t> =>
                !!t && (t.state === "available" || t.state === "partial"),
            )
        : [],
    [profile, tweaks],
  );

  const confirmTweaks = profile ? profileTweaks : applicable;
  const confirmLabel = profile ? profile.name : `${title} tweaks`;

  const requestApply = useCallback(() => {
    if (confirmTweaks.length === 0 || running) return;
    if (settings.confirmBulk) {
      setConfirmOpen(true);
    } else {
      void run(
        confirmTweaks.map((t) => t.meta.id),
        profile ? `${profile.name} profile` : confirmLabel,
      ).then(() => setSelectedProfile(null));
    }
  }, [confirmTweaks, running, settings.confirmBulk, run, profile, confirmLabel]);

  async function onConfirm() {
    setConfirmOpen(false);
    await run(
      confirmTweaks.map((t) => t.meta.id),
      profile ? `${profile.name} profile` : confirmLabel,
    );
    setSelectedProfile(null);
  }

  if (tweaksLoading && categoryTweaks.length === 0) {
    return (
      <div className="mx-auto max-w-4xl space-y-2">
        <h1 className="text-lg font-semibold text-foreground">{title}</h1>
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-16 animate-pulse rounded-lg bg-surface" />
        ))}
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-foreground">{title}</h1>
          {description && <p className="text-sm text-muted">{description}</p>}
        </div>
        <div className="flex items-center gap-2">
          <div className="flex rounded-md border border-border p-0.5">
            {(["all", "available", "applied"] as const).map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setFilter(f)}
                className={cn(
                  "rounded px-2.5 py-1 text-xs capitalize transition-colors",
                  filter === f
                    ? "bg-teal-950/70 text-teal-200"
                    : "text-muted hover:text-foreground",
                )}
              >
                {f}
              </button>
            ))}
          </div>
          <button
            type="button"
            onClick={requestApply}
            disabled={applicable.length === 0 || running}
            className="flex items-center gap-1.5 rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Zap className="h-4 w-4" />
            {running ? "Applying…" : `Apply ${title}`}
          </button>
        </div>
      </div>

      {tweaksError && <ErrorBanner message={tweaksError} />}
      {runError && <ErrorBanner message={runError} />}

      {isGaming && profiles.length > 0 && (
        <section className="rounded-xl border border-border bg-panel p-4">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Gamepad2 className="h-4 w-4 text-teal-400" />
            Optimization profiles
          </h2>
          <p className="mt-1 text-xs text-muted">
            Profiles are fixed collections of individual tweaks — review exactly what each one
            changes before applying.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            {profiles.map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => setSelectedProfile(p.name === selectedProfile ? null : p.name)}
                className={cn(
                  "rounded-md border px-3 py-1.5 text-xs transition-colors",
                  selectedProfile === p.name
                    ? "border-teal-700 bg-teal-950/60 text-teal-200"
                    : "border-border text-muted hover:text-foreground",
                )}
              >
                {p.name}
                <span className="ml-1.5 text-muted/70">{p.tweakIds.length}</span>
              </button>
            ))}
          </div>

          {profile && (
            <div className="mt-3 space-y-2">
              {profileTweaks.map((t) => (
                <TweakRow key={t.meta.id} tweak={t} />
              ))}
              {profileTweaks.length === 0 && (
                <p className="text-xs text-muted">
                  All tweaks in this profile are already applied.
                </p>
              )}
              <button
                type="button"
                onClick={requestApply}
                disabled={profileTweaks.length === 0 || running}
                className="rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
              >
                Apply {profile.name} ({profileTweaks.length})
              </button>
            </div>
          )}
        </section>
      )}

      <div className="space-y-2">
        {shown.length === 0 ? (
          <div className="rounded-xl border border-border bg-panel px-4 py-10 text-center">
            <p className="text-sm text-muted">No tweaks in this view.</p>
          </div>
        ) : (
          shown.map((t) => <TweakRow key={t.meta.id} tweak={t} />)
        )}
      </div>

      {settings.confirmBulk && (
        <ConfirmDialog
          open={confirmOpen}
          title={profile ? `Apply ${profile.name} profile` : `Apply ${title} tweaks`}
          confirmLabel="Apply selected tweaks"
          tweaks={confirmTweaks}
          onClose={() => setConfirmOpen(false)}
          onConfirm={onConfirm}
        />
      )}
    </div>
  );
}
