/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class", "[data-theme='dark']"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Zenou token mapping (see index.css). Semantic aliases used across the UI.
        background: "var(--bg-base)",
        surface: "var(--bg-elevated)",
        panel: "var(--bg-panel)",
        behind: "var(--bg-behind)",
        border: {
          DEFAULT: "var(--border)",
          subtle: "var(--border-subtle)",
          strong: "var(--border-strong)",
        },
        foreground: "var(--text-primary)",
        muted: {
          DEFAULT: "var(--text-muted)",
          dim: "var(--text-dim)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          strong: "var(--accent-strong)",
          soft: "var(--accent-soft)",
          border: "var(--accent-border)",
        },
      },
      borderRadius: {
        sm: "8px",
        md: "12px",
        lg: "16px",
      },
      fontFamily: {
        base: "Inter, 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif",
      },
      boxShadow: {
        card: "0 1px 2px rgba(0,0,0,0.35), 0 8px 24px -12px rgba(0,0,0,0.45)",
      },
      transitionTimingFunction: {
        zen: "cubic-bezier(0.3, 0, 0.2, 1)",
      },
    },
  },
  plugins: [],
};
