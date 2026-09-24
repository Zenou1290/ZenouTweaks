import { useEffect, useState } from "react";
import { useUi } from "@/store/ui";
import { api, toMessage } from "@/lib/api";
import { ErrorBanner } from "@/components/details-section";
import { cn } from "@/lib/utils";
import { MonitorCog, ShieldCheck, Loader2 } from "lucide-react";

const ACCENTS = ["teal", "emerald", "sky", "violet"] as const;

function Toggle({
  checked,
  onChange,
  label,
  description,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  description: string;
}) {
  return (
    <label className="flex cursor-pointer items-start justify-between gap-4 rounded-lg border border-border bg-panel px-4 py-3">
      <div>
        <p className="text-sm font-medium text-foreground">{label}</p>
        <p className="mt-0.5 text-xs text-muted">{description}</p>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={cn(
          "mt-0.5 h-5 w-9 shrink-0 rounded-full border transition-colors",
          checked ? "border-teal-600 bg-teal-600/80" : "border-border bg-surface",
        )}
      >
        <span
          className={cn(
            "block h-3.5 w-3.5 translate-x-[3px] rounded-full bg-white transition-transform",
            checked && "translate-x-[19px]",
          )}
        />
      </button>
    </label>
  );
}

export function SettingsPage() {
  const { settings, updateSettings } = useUi();
  const [error, setError] = useState<string | null>(null);
  const [elevating, setElevating] = useState(false);
  const [rpMsg, setRpMsg] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.dataset.accent = settings.accent;
  }, [settings.accent]);

  async function requestElevation() {
    setElevating(true);
    setError(null);
    try {
      await api.requestElevation();
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
    } finally {
      setElevating(false);
      // Refresh scan to reflect new elevation status
      void useUi.getState().rescan();
    }
  }

  async function createRestorePoint() {
    setRpMsg(null);
    try {
      const msg = await api.createRestorePoint();
      setRpMsg(msg || "Restore point created.");
    } catch (err) {
      const m = toMessage(err);
      setError(m.message);
    }
  }

  return (
    <div className="mx-auto max-w-3xl space-y-4">
      <div>
        <h1 className="text-lg font-semibold text-foreground">Settings</h1>
        <p className="text-sm text-muted">Stored locally. No accounts, no telemetry.</p>
      </div>

      {error && <ErrorBanner message={error} onDismiss={() => setError(null)} />}
      {rpMsg && (
        <div className="rounded-lg border border-teal-900/50 bg-teal-950/20 px-4 py-3 text-sm text-teal-200">
          {rpMsg}
        </div>
      )}

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-foreground">General</h2>
        <Toggle
          checked={settings.startWithWindows}
          onChange={(v) => void updateSettings({ startWithWindows: v })}
          label="Start with Windows"
          description="Launch Zenou Tweaks automatically when you sign in."
        />
        <Toggle
          checked={settings.minimizeToTray}
          onChange={(v) => void updateSettings({ minimizeToTray: v })}
          label="Minimize to tray"
          description="Keep Zenou running in the system tray when the window is closed."
        />
        <Toggle
          checked={settings.notifications}
          onChange={(v) => void updateSettings({ notifications: v })}
          label="Notifications"
          description="Show completion toasts when operations finish."
        />
        <Toggle
          checked={settings.reducedMotion}
          onChange={(v) => void updateSettings({ reducedMotion: v })}
          label="Reduced motion"
          description="Minimize animations across the interface."
        />
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-foreground">Appearance</h2>
        <div className="rounded-lg border border-border bg-panel px-4 py-3">
          <p className="text-sm font-medium text-foreground">Theme</p>
          <div className="mt-2 flex gap-2">
            {["dark", "light"].map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => void updateSettings({ theme: t })}
                className={cn(
                  "rounded-md border px-3 py-1.5 text-xs capitalize",
                  settings.theme === t
                    ? "border-teal-700 bg-teal-950/60 text-teal-200"
                    : "border-border text-muted hover:text-foreground",
                )}
              >
                {t}
              </button>
            ))}
          </div>
        </div>
        <div className="rounded-lg border border-border bg-panel px-4 py-3">
          <p className="text-sm font-medium text-foreground">Accent color</p>
          <div className="mt-2 flex gap-2">
            {ACCENTS.map((a) => (
              <button
                key={a}
                type="button"
                onClick={() => void updateSettings({ accent: a })}
                className={cn(
                  "flex items-center gap-2 rounded-md border px-3 py-1.5 text-xs capitalize",
                  settings.accent === a
                    ? "border-teal-700 bg-teal-950/60 text-teal-200"
                    : "border-border text-muted hover:text-foreground",
                )}
              >
                <span
                  className={cn(
                    "h-3 w-3 rounded-full",
                    a === "teal" && "bg-teal-500",
                    a === "emerald" && "bg-emerald-500",
                    a === "sky" && "bg-sky-500",
                    a === "violet" && "bg-violet-500",
                  )}
                />
                {a}
              </button>
            ))}
          </div>
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-foreground">Safety</h2>
        <Toggle
          checked={settings.restorePointRisky}
          onChange={(v) => void updateSettings({ restorePointRisky: true && v })}
          label="Restore point before risky tweaks"
          description="Create a Windows restore point before applying medium/high risk tweaks."
        />
        <Toggle
          checked={settings.confirmBulk}
          onChange={(v) => void updateSettings({ confirmBulk: v })}
          label="Confirm before bulk changes"
          description="Always review the exact list of tweaks before a group apply."
        />
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-panel px-4 py-3">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-foreground">Windows restore point</p>
            <p className="mt-0.5 text-xs text-muted">
              Create a restore point now. Requires System Protection to be enabled on C:.
            </p>
          </div>
          <button
            type="button"
            onClick={() => void createRestorePoint()}
            className="rounded-md border border-border px-3 py-2 text-sm text-muted hover:text-foreground"
          >
            Create now
          </button>
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-foreground">Permissions</h2>
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-panel px-4 py-3">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-foreground">Administrator permission</p>
            <p className="mt-0.5 text-xs text-muted">
              Zenou runs without elevation and only asks when a tweak needs it. You can also
              grant it up front — the app itself stays lightweight.
            </p>
          </div>
          <button
            type="button"
            onClick={() => void requestElevation()}
            disabled={elevating}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-muted hover:text-foreground disabled:opacity-50"
          >
            {elevating ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <ShieldCheck className="h-4 w-4" />
            )}
            Run as administrator
          </button>
        </div>
        <div className="flex items-center gap-2 rounded-lg border border-border bg-panel px-4 py-3">
          <MonitorCog className="h-4 w-4 text-teal-400/80" />
          <p className="text-xs text-muted">
            Zenou Tweaks v1.0.0 — fully offline. Settings and backups live in{" "}
            <span className="font-mono">%APPDATA%\ZenouTweaks</span>.
          </p>
        </div>
      </section>
    </div>
  );
}
