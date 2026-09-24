import { cn } from "@/lib/utils";
import { AlertTriangle, CheckCircle2, CircleAlert, ShieldAlert } from "lucide-react";

const RISK_STYLE: Record<string, string> = {
  low: "border-emerald-800/60 bg-emerald-950/50 text-emerald-300",
  medium: "border-amber-800/60 bg-amber-950/50 text-amber-300",
  high: "border-red-900/60 bg-red-950/50 text-red-300",
};

const RISK_LABEL: Record<string, string> = { low: "Low", medium: "Medium", high: "High" };

export function RiskBadge({ risk }: { risk: string }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide",
        RISK_STYLE[risk] ?? RISK_STYLE.medium,
      )}
    >
      {risk === "high" ? (
        <ShieldAlert className="h-3 w-3" />
      ) : risk === "medium" ? (
        <AlertTriangle className="h-3 w-3" />
      ) : (
        <CheckCircle2 className="h-3 w-3" />
      )}
      {RISK_LABEL[risk] ?? risk}
    </span>
  );
}

const STATE_STYLE: Record<string, string> = {
  available: "border-border bg-surface text-muted",
  applied: "border-teal-800/60 bg-teal-950/40 text-teal-300",
  partial: "border-amber-800/60 bg-amber-950/40 text-amber-300",
  failed: "border-red-900/60 bg-red-950/40 text-red-300",
  unsupported: "border-border bg-surface text-muted/60",
  unknown: "border-border bg-surface text-muted/70",
};

const STATE_LABEL: Record<string, string> = {
  available: "Available",
  applied: "Applied",
  partial: "Partial",
  failed: "Failed",
  unsupported: "Unsupported",
  unknown: "Unknown",
};

export function StateBadge({ state, className }: { state: string; className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide",
        STATE_STYLE[state] ?? STATE_STYLE.unknown,
        className,
      )}
    >
      {state === "unknown" && <CircleAlert className="h-3 w-3" />}
      {STATE_LABEL[state] ?? state}
    </span>
  );
}
