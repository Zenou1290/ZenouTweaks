import { useEffect, useMemo, useState } from "react";
import { useUi } from "@/store/ui";
import { api, toMessage } from "@/lib/api";
import type { GroupResult, TweakView } from "@/types";

/** Selectable list model for bulk actions (Quick Optimize, profiles, group apply). */
export interface SelectableTweak {
  meta: TweakView["meta"];
  state: TweakView["state"];
  disabled: boolean;
  reason: string | null;
}

export function useBulkApply() {
  const { tweaks, setTweakState, setBusy, loadTweaks, settings, scan, scanning } = useUi();
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<GroupResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const eligible = useMemo(
    () =>
      tweaks.filter(
        (t) => t.state === "available" || t.state === "partial",
      ),
    [tweaks],
  );

  async function run(ids: string[], label: string): Promise<GroupResult | null> {
    setRunning(true);
    setError(null);
    setResult(null);
    for (const id of ids) setBusy(id, "applying");
    try {
      const res = await api.applyGroup(ids, label);
      setResult(res);
      for (const r of res.results) {
        setTweakState(r.tweakId, r.success ? "applied" : r.skipped ? "available" : "unknown");
      }
      await loadTweaks();
      return res;
    } catch (err) {
      setError(toMessage(err).message);
      return null;
    } finally {
      for (const id of ids) setBusy(id, null);
      setRunning(false);
    }
  }

  return { tweaks, eligible, running, result, error, run, settings, scan, scanning };
}

/** Convenience hook: does the scan exist and is hardware relevant? */
export function useHardwareGate() {
  const { scan, scanning } = useUi();
  return { scanned: scan !== null, scanning };
}

export function useElevated() {
  const { scan } = useUi();
  return scan?.isElevated ?? false;
}

/** Hook: initial page load — fetch scan + tweaks once. */
export function useBootstrap() {
  const { scan, loadTweaks, tweaks } = useUi();
  const [booted, setBooted] = useState(false);

  useEffect(() => {
    if (booted) return;
    setBooted(true);
    void useUi.getState().rescan();
    void loadTweaks();
  }, [booted, loadTweaks]);

  return { scan, tweaks, ready: scan !== null && tweaks.length > 0 };
}
