import { useMemo, useState } from "react";
import { useUi } from "@/store/ui";
import { TweakRow } from "@/components/tweak-row";
import { ErrorBanner } from "@/components/details-section";
import { cn } from "@/lib/utils";
import { Search, SlidersHorizontal, Star } from "lucide-react";

const CATEGORIES = [
  { id: "all", label: "All" },
  { id: "performance", label: "Performance" },
  { id: "gaming", label: "Gaming" },
  { id: "gpu", label: "GPU" },
  { id: "network", label: "Network" },
  { id: "cleanup", label: "Cleanup" },
  { id: "windows", label: "Windows" },
];

const RISKS = ["all", "low", "medium", "high"] as const;
const STATUSES = ["all", "available", "applied", "partial", "unsupported", "unknown"] as const;

export function TweaksPage() {
  const { tweaks, tweaksLoading, tweaksError } = useUi();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [risk, setRisk] = useState<(typeof RISKS)[number]>("all");
  const [status, setStatus] = useState<(typeof STATUSES)[number]>("all");
  const [favOnly, setFavOnly] = useState(false);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return tweaks.filter((t) => {
      if (favOnly && !t.favorite) return false;
      if (category !== "all" && t.meta.category !== category) return false;
      if (risk !== "all" && t.meta.risk !== risk) return false;
      if (status !== "all" && t.state !== status) return false;
      if (q && !`${t.meta.name} ${t.meta.description}`.toLowerCase().includes(q)) return false;
      return true;
    });
  }, [tweaks, query, category, risk, status, favOnly]);

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-foreground">Tweaks</h1>
          <p className="text-sm text-muted">
            Every tweak in one place — search, filter, and manage individually.
          </p>
        </div>
      </div>

      {tweaksError && <ErrorBanner message={tweaksError} />}

      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-56 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted/60" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search tweaks…"
            className="w-full rounded-md border border-border bg-surface py-2 pl-8 pr-3 text-sm text-foreground placeholder:text-muted/60 focus:border-teal-800 focus:outline-none"
          />
        </div>
        <button
          type="button"
          onClick={() => setFavOnly((f) => !f)}
          className={cn(
            "flex items-center gap-1.5 rounded-md border px-3 py-2 text-xs transition-colors",
            favOnly
              ? "border-teal-700 bg-teal-950/60 text-teal-200"
              : "border-border text-muted hover:text-foreground",
          )}
        >
          <Star className={cn("h-3.5 w-3.5", favOnly && "fill-teal-300")} /> Favorites
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {CATEGORIES.map((c) => (
          <button
            key={c.id}
            type="button"
            onClick={() => setCategory(c.id)}
            className={cn(
              "rounded-full border px-3 py-1 text-xs transition-colors",
              category === c.id
                ? "border-teal-700 bg-teal-950/60 text-teal-200"
                : "border-border text-muted hover:text-foreground",
            )}
          >
            {c.label}
          </button>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <span className="flex items-center gap-1 text-xs text-muted">
          <SlidersHorizontal className="h-3.5 w-3.5" />
          Risk:
        </span>
        {RISKS.map((r) => (
          <button
            key={r}
            type="button"
            onClick={() => setRisk(r)}
            className={cn(
              "rounded-full border px-3 py-1 text-xs capitalize transition-colors",
              risk === r
                ? "border-teal-700 bg-teal-950/60 text-teal-200"
                : "border-border text-muted hover:text-foreground",
            )}
          >
            {r}
          </button>
        ))}
        <span className="ml-3 text-xs text-muted">Status:</span>
        {STATUSES.map((s) => (
          <button
            key={s}
            type="button"
            onClick={() => setStatus(s)}
            className={cn(
              "rounded-full border px-3 py-1 text-xs capitalize transition-colors",
              status === s
                ? "border-teal-700 bg-teal-950/60 text-teal-200"
                : "border-border text-muted hover:text-foreground",
            )}
          >
            {s}
          </button>
        ))}
      </div>

      <div className="space-y-2">
        {tweaksLoading && tweaks.length === 0 ? (
          [...Array(6)].map((_, i) => (
            <div key={i} className="h-16 animate-pulse rounded-lg bg-surface" />
          ))
        ) : shown.length === 0 ? (
          <div className="rounded-xl border border-border bg-panel px-4 py-10 text-center">
            <p className="text-sm text-muted">
              {tweaks.length === 0 ? "No tweaks available." : "No tweaks match these filters."}
            </p>
          </div>
        ) : (
          shown.map((t) => <TweakRow key={t.meta.id} tweak={t} />)
        )}
      </div>
    </div>
  );
}
