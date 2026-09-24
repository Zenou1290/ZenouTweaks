import { useMemo, useState } from "react";
import { useUi } from "@/store/ui";
import { useBulkApply } from "@/hooks/use-bulk-apply";
import { TweakRow } from "@/components/tweak-row";
import { ErrorBanner } from "@/components/details-section";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { cn } from "@/lib/utils";
import {
  ScanLine,
  Cpu,
  MemoryStick,
  HardDrive,
  MonitorSmartphone,
  Network,
  ShieldCheck,
  ShieldAlert,
  Zap,
  RefreshCw,
} from "lucide-react";

function InfoTile({
  icon: Icon,
  label,
  value,
  sub,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-panel px-4 py-3">
      <div className="flex items-center gap-2 text-[11px] uppercase tracking-wide text-muted">
        <Icon className="h-3.5 w-3.5 text-teal-400/80" />
        {label}
      </div>
      <p className="mt-1.5 truncate text-sm font-medium text-foreground" title={value}>
        {value}
      </p>
      {sub && <p className="truncate text-xs text-muted/80">{sub}</p>}
    </div>
  );
}

export function DashboardPage() {
  const { scan, scanning, scanError, rescan, tweaks, settings } = useUi();
  const { run, running, error: runError } = useBulkApply();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [justApplied, setJustApplied] = useState<string[] | null>(null);

  const candidates = useMemo(
    () => tweaks.filter((t) => t.meta.quickOptimize && t.state === "available"),
    [tweaks],
  );
  const appliedCount = tweaks.filter((t) => t.state === "applied").length;
  const unsupported = tweaks.filter((t) => t.state === "unsupported").length;
  const unknown = tweaks.filter((t) => t.state === "unknown").length;

  async function quickOptimize() {
    setConfirmOpen(false);
    const ids = candidates.map((t) => t.meta.id);
    const res = await run(ids, "Quick Optimize");
    if (res) setJustApplied(res.results.filter((r) => r.success).map((r) => r.tweakId));
  }

  function requestOptimize() {
    if (candidates.length === 0 || running) return;
    if (settings.confirmBulk) setConfirmOpen(true);
    else void quickOptimize();
  }

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-foreground">Dashboard</h1>
          <p className="text-sm text-muted">
            System overview and available optimizations.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void rescan()}
            disabled={scanning}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-muted hover:text-foreground transition-colors disabled:opacity-50"
          >
            <RefreshCw className={cn("h-4 w-4", scanning && "animate-spin")} />
            {scanning ? "Scanning…" : "Rescan"}
          </button>
          <button
            type="button"
            onClick={requestOptimize}
            disabled={candidates.length === 0 || running}
            className="flex items-center gap-1.5 rounded-md bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-500 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
            title={
              candidates.length === 0
                ? "No applicable optimizations right now"
                : `Review and apply ${candidates.length} optimizations`
            }
          >
            <Zap className="h-4 w-4" />
            {running ? "Applying…" : "Optimize"}
          </button>
        </div>
      </div>

      {scanError && <ErrorBanner message={scanError} onDismiss={() => void rescan()} />}
      {runError && <ErrorBanner message={runError} />}

      {/* Scan section */}
      <section className="rounded-xl border border-border bg-panel p-5">
        <div className="flex items-center justify-between">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-foreground">
            <ScanLine className="h-4 w-4 text-teal-400" />
            Zenou Scan
          </h2>
          {scan && (
            <span className="flex items-center gap-1 text-xs text-muted">
              {scan.isElevated ? (
                <>
                  <ShieldCheck className="h-3.5 w-3.5 text-teal-400" /> Elevated
                </>
              ) : (
                <>
                  <ShieldAlert className="h-3.5 w-3.5 text-amber-400" /> Standard user — admin
                  tweaks will prompt
                </>
              )}
            </span>
          )}
        </div>

        {scanning && !scan ? (
          <div className="mt-4 space-y-3">
            {[...Array(6)].map((_, i) => (
              <div key={i} className="h-12 animate-pulse rounded-lg bg-surface" />
            ))}
          </div>
        ) : scan ? (
          <>
            <div className="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
              <InfoTile
                icon={Cpu}
                label="CPU"
                value={scan.cpuName}
                sub={`${scan.cpuCores} cores · ${scan.cpuThreads} threads`}
              />
              <InfoTile
                icon={MemoryStick}
                label="Memory"
                value={scan.ramGb != null ? `${scan.ramGb} GB` : "Unknown"}
                sub={scan.architecture}
              />
              <InfoTile
                icon={MonitorSmartphone}
                label="GPU"
                value={scan.gpus[0]?.name ?? "Not detected"}
                sub={
                  scan.gpus.length > 1
                    ? `+${scan.gpus.length - 1} more`
                    : scan.gpus[0]?.driverVersion
                      ? `Driver ${scan.gpus[0].driverVersion}`
                      : undefined
                }
              />
              <InfoTile
                icon={HardDrive}
                label="Storage"
                value={
                  scan.storage[0]?.sizeGb != null
                    ? `${scan.storage[0].sizeGb} GB`
                    : "Unknown"
                }
                sub={
                  scan.storage.length > 1
                    ? `${scan.storage.length} drives`
                    : scan.storage[0]?.model
                }
              />
            </div>
            <div className="mt-3 flex flex-wrap items-center gap-x-5 gap-y-1 text-xs text-muted">
              <span>
                {scan.windowsEdition} · {scan.windowsVersion} · build {scan.windowsBuild}
              </span>
              <span className="flex items-center gap-1">
                <Network className="h-3 w-3" />
                {scan.adapters.length} network adapter{scan.adapters.length === 1 ? "" : "s"}
              </span>
              <span>Uptime {scan.uptime}</span>
              <span>
                {scan.hostname} · {scan.username}
              </span>
            </div>
          </>
        ) : (
          <p className="mt-4 text-sm text-muted">Scan could not complete. Try Rescan.</p>
        )}
      </section>

      {/* Status summary */}
      <section className="grid grid-cols-3 gap-3">
        <div className="rounded-lg border border-border bg-panel px-4 py-3">
          <p className="text-xs text-muted">Available optimizations</p>
          <p className="mt-1 text-2xl font-semibold text-foreground">{candidates.length}</p>
        </div>
        <div className="rounded-lg border border-border bg-panel px-4 py-3">
          <p className="text-xs text-muted">Applied tweaks</p>
          <p className="mt-1 text-2xl font-semibold text-teal-300">{appliedCount}</p>
        </div>
        <div className="rounded-lg border border-border bg-panel px-4 py-3">
          <p className="text-xs text-muted">Not applicable / unknown</p>
          <p className="mt-1 text-2xl font-semibold text-muted">
            {unsupported + unknown}
          </p>
        </div>
      </section>

      {justApplied && (
        <section className="rounded-xl border border-teal-900/50 bg-teal-950/20 p-4">
          <h3 className="text-sm font-medium text-teal-200">
            Quick Optimize finished — {justApplied.length} tweak
            {justApplied.length === 1 ? "" : "s"} applied and verified.
          </h3>
          <div className="mt-3 space-y-2">
            {tweaks
              .filter((t) => justApplied.includes(t.meta.id))
              .map((t) => (
                <TweakRow key={t.meta.id} tweak={t} />
              ))}
          </div>
          <button
            type="button"
            onClick={() => setJustApplied(null)}
            className="mt-3 text-xs text-muted hover:text-foreground"
          >
            Dismiss
          </button>
        </section>
      )}

      {candidates.length > 0 && !justApplied && (
        <section>
          <h2 className="mb-2 text-sm font-semibold text-foreground">
            Recommended for this system
          </h2>
          <p className="mb-3 text-xs text-muted">
            Only tweaks applicable to your detected hardware and Windows version are listed.
          </p>
          <div className="space-y-2">
            {candidates.slice(0, 6).map((t) => (
              <TweakRow key={t.meta.id} tweak={t} />
            ))}
          </div>
          {candidates.length > 6 && (
            <p className="mt-2 text-xs text-muted">
              And {candidates.length - 6} more across the category pages.
            </p>
          )}
        </section>
      )}

      {settings.confirmBulk && (
        <ConfirmDialog
          open={confirmOpen}
          title="Quick Optimize"
          confirmLabel="Apply selected tweaks"
          tweaks={candidates}
          onClose={() => setConfirmOpen(false)}
          onConfirm={quickOptimize}
        />
      )}
    </div>
  );
}
