import { useEffect, useState } from "react";

/** Show a toast message for a few seconds. */
export function useToast() {
  const [msg, setMsg] = useState<{ text: string; kind: "ok" | "error" } | null>(null);

  useEffect(() => {
    if (!msg) return;
    const t = setTimeout(() => setMsg(null), 4000);
    return () => clearTimeout(t);
  }, [msg]);

  return {
    msg,
    show: (text: string, kind: "ok" | "error" = "ok") => setMsg({ text, kind }),
    clear: () => setMsg(null),
  };
}
