import { useEffect, useState } from "react";
import { useUi } from "@/store/ui";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";
import {
  LayoutDashboard,
  Gauge,
  Gamepad2,
  MonitorSmartphone,
  Network,
  Brush,
  AppWindow,
  SlidersHorizontal,
  History,
  Save,
  ScrollText,
  Settings,
  ShieldAlert,
} from "lucide-react";

const NAV = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "performance", label: "Performance", icon: Gauge },
  { id: "gaming", label: "Gaming", icon: Gamepad2 },
  { id: "gpu", label: "GPU", icon: MonitorSmartphone },
  { id: "network", label: "Network", icon: Network },
  { id: "cleanup", label: "Cleanup", icon: Brush },
  { id: "windows", label: "Windows", icon: AppWindow },
  { id: "tweaks", label: "Tweaks", icon: SlidersHorizontal },
  { id: "changes", label: "Changes", icon: History },
  { id: "backup", label: "Backup & Restore", icon: Save },
  { id: "logs", label: "Logs", icon: ScrollText },
  { id: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const { page, setPage, tweaks } = useUi();
  const [flagged, setFlagged] = useState(0);

  useEffect(() => {
    api
      .flaggedOperations()
      .then((f) => setFlagged(f.length))
      .catch(() => setFlagged(0));
  }, []);

  const appliedCount = tweaks.filter((t) => t.state === "applied").length;
  const categoryCounts = new Map<string, { applied: number; total: number }>();
  for (const t of tweaks) {
    const c = categoryCounts.get(t.meta.category) ?? { applied: 0, total: 0 };
    c.total += 1;
    if (t.state === "applied") c.applied += 1;
    categoryCounts.set(t.meta.category, c);
  }

  return (
    <nav className="flex w-56 shrink-0 flex-col border-r border-border bg-surface/50 px-2 py-3">
      {NAV.map(({ id, label, icon: Icon }) => {
        const active = page === id;
        const counts = categoryCounts.get(id);
        return (
          <button
            key={id}
            type="button"
            onClick={() => setPage(id)}
            className={cn(
              "flex items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm transition-colors",
              active
                ? "bg-teal-950/60 text-teal-200"
                : "text-muted hover:bg-surface hover:text-foreground",
            )}
          >
            <Icon className={cn("h-4 w-4", active && "text-teal-300")} />
            <span className="flex-1 truncate">{label}</span>
            {counts && counts.applied > 0 && (
              <span className="rounded bg-teal-950/80 px-1.5 py-0.5 text-[10px] text-teal-300">
                {counts.applied}/{counts.total}
              </span>
            )}
          </button>
        );
      })}

      <div className="mt-auto space-y-2 px-3 pt-2">
        {flagged > 0 && (
          <button
            type="button"
            onClick={() => setPage("tweaks")}
            className="flex w-full items-center gap-1.5 rounded-md border border-amber-900/50 bg-amber-950/30 px-2.5 py-1.5 text-left text-[11px] text-amber-300"
            title="Blocked .bat operations are documented in the Tweaks page"
          >
            <ShieldAlert className="h-3.5 w-3.5 shrink-0" />
            {flagged} risky ops blocked
          </button>
        )}
        <p className="text-[11px] text-muted/60">
          {appliedCount} tweak{appliedCount === 1 ? "" : "s"} applied
        </p>
      </div>
    </nav>
  );
}
