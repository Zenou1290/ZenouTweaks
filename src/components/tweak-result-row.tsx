import { StateBadge } from "@/components/badges";
import { DetailsSection } from "@/components/details-section";
import type { TweakResult } from "@/types";
import { CheckCircle2, XCircle, MinusCircle } from "lucide-react";

/** Compact per-tweak outcome row used in group results and change sets. */
export function TweakResultRow({ result }: { result: TweakResult }) {
  return (
    <div className="rounded-lg border border-border bg-surface px-3 py-2">
      <div className="flex items-center gap-2.5">
        {result.success ? (
          <CheckCircle2 className="h-4 w-4 shrink-0 text-teal-400" />
        ) : result.skipped ? (
          <MinusCircle className="h-4 w-4 shrink-0 text-amber-400" />
        ) : (
          <XCircle className="h-4 w-4 shrink-0 text-red-400" />
        )}
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm text-foreground">
            {result.tweakId.replace(/-/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}
          </p>
          <p className="truncate text-xs text-muted">
            {result.success
              ? result.restartRequired
                ? "Applied — restart required"
                : "Applied and verified"
              : result.skipped
                ? (result.error ?? "Skipped")
                : (result.error ?? "Failed")}
          </p>
        </div>
        <StateBadge
          state={result.success ? "applied" : result.skipped ? "available" : "failed"}
        />
      </div>
      {(result.steps.some((s) => !s.ok) || (result.details && !result.success)) && (
        <DetailsSection className="mt-2" title="Technical details">
          {result.details ?? ""}
          {result.steps.length > 0 && (
            <>
              {result.details ? "\n\n" : ""}
              {result.steps
                .map(
                  (s) =>
                    `${s.ok ? "OK " : "ERR"} ${s.location}${s.error ? ` — ${s.error}` : ""}`,
                )
                .join("\n")}
            </>
          )}
        </DetailsSection>
      )}
    </div>
  );
}
