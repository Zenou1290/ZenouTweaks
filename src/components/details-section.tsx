import { useState } from "react";
import { cn } from "@/lib/utils";
import { ChevronDown, Info } from "lucide-react";

/** Collapsible technical details section (used for errors, logs, tweak steps). */
export function DetailsSection({
  title = "View details",
  children,
  defaultOpen = false,
  className,
}: {
  title?: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  className?: string;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className={cn("rounded-md border border-subtle bg-panel", className)}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full items-center gap-1.5 px-3 py-2 text-xs text-muted hover:text-foreground transition-colors"
      >
        <Info className="h-3.5 w-3.5" />
        {title}
        <ChevronDown
          className={cn("ml-auto h-3.5 w-3.5 transition-transform", open && "rotate-180")}
        />
      </button>
      {open && (
        <div className="border-t border-subtle px-3 py-2 text-xs text-muted whitespace-pre-wrap break-words">
          {children}
        </div>
      )}
    </div>
  );
}

/** Full-width error banner with optional expandable details. */
export function ErrorBanner({
  message,
  details,
  onDismiss,
}: {
  message: string;
  details?: string | null;
  onDismiss?: () => void;
}) {
  return (
    <div className="rounded-lg border border-red-900/60 bg-red-950/40 p-3 text-sm">
      <div className="flex items-start justify-between gap-2">
        <p className="text-red-300">{message}</p>
        {onDismiss && (
          <button
            type="button"
            onClick={onDismiss}
            className="text-xs text-red-400/70 hover:text-red-300"
          >
            Dismiss
          </button>
        )}
      </div>
      {details && (
        <DetailsSection className="mt-2 border-red-900/40 bg-red-950/20" title="Technical details">
          {details}
        </DetailsSection>
      )}
    </div>
  );
}
