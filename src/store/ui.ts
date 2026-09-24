import { create } from "zustand";
import { api, toMessage } from "@/lib/api";
import type { AppSettings, SystemScan, TweakView, OperationKind } from "@/types";
import { DEFAULT_SETTINGS } from "@/types";

interface UiState {
  page: string;
  setPage: (p: string) => void;

  scan: SystemScan | null;
  scanning: boolean;
  scanError: string | null;
  rescan: () => Promise<void>;

  tweaks: TweakView[];
  tweaksLoading: boolean;
  tweaksError: string | null;
  loadTweaks: () => Promise<void>;
  setTweakState: (id: string, state: TweakView["state"]) => void;

  /** Per-tweak transient operation state (applying/restoring). */
  busy: Record<string, OperationKind>;
  setBusy: (id: string, op: OperationKind | null) => void;

  /** Tweak ids that require restart after a successful apply. */
  restartPending: string[];
  addRestartPending: (id: string) => void;

  settings: AppSettings;
  settingsLoaded: boolean;
  loadSettings: () => Promise<void>;
  updateSettings: (patch: Partial<AppSettings>) => Promise<void>;

  favorites: string[];
  toggleFavorite: (id: string) => Promise<void>;

  /** Selected tweak id for the details drawer. */
  selectedTweakId: string | null;
  setSelectedTweak: (id: string | null) => void;
}

export const useUi = create<UiState>((set, get) => ({
  page: "dashboard",
  setPage: (p) => set({ page: p }),

  scan: null,
  scanning: false,
  scanError: null,
  rescan: async () => {
    set({ scanning: true, scanError: null });
    try {
      const scan = await api.scanSystem(true);
      set({ scan, scanning: false });
    } catch (err) {
      const { message } = toMessage(err);
      set({ scanError: message, scanning: false });
    }
  },

  tweaks: [],
  tweaksLoading: false,
  tweaksError: null,
  loadTweaks: async () => {
    set({ tweaksLoading: true, tweaksError: null });
    try {
      const tweaks = await api.listTweaks();
      set({ tweaks, tweaksLoading: false });
    } catch (err) {
      const { message } = toMessage(err);
      set({ tweaksError: message, tweaksLoading: false });
    }
  },
  setTweakState: (id, state) =>
    set((s) => ({
      tweaks: s.tweaks.map((t) => (t.meta.id === id ? { ...t, state } : t)),
    })),

  busy: {},
  setBusy: (id, op) =>
    set((s) => {
      const busy = { ...s.busy };
      if (op) busy[id] = op;
      else delete busy[id];
      return { busy };
    }),

  restartPending: [],
  addRestartPending: (id) =>
    set((s) =>
      s.restartPending.includes(id) ? s : { restartPending: [...s.restartPending, id] },
    ),

  settings: DEFAULT_SETTINGS,
  settingsLoaded: false,
  loadSettings: async () => {
    try {
      const settings = await api.getSettings();
      set({ settings, settingsLoaded: true });
      applyTheme(settings);
    } catch {
      set({ settingsLoaded: true });
      applyTheme(get().settings);
    }
  },
  updateSettings: async (patch) => {
    const next = { ...get().settings, ...patch };
    set({ settings: next });
    applyTheme(next);
    try {
      await api.setSettings(next);
    } catch (err) {
      const { message } = toMessage(err);
      console.error(message);
    }
  },

  favorites: [],
  toggleFavorite: async (id) => {
    try {
      const favorites = await api.toggleFavorite(id);
      set({ favorites });
      set((s) => ({
        tweaks: s.tweaks.map((t) => ({ ...t, favorite: favorites.includes(t.meta.id) })),
      }));
    } catch {
      // best-effort
    }
  },

  selectedTweakId: null,
  setSelectedTweak: (id) => set({ selectedTweakId: id }),
}));

export function applyTheme(settings: AppSettings) {
  const root = document.documentElement;
  root.dataset.theme = settings.theme === "light" ? "light" : "dark";
  if (settings.reducedMotion) {
    root.dataset.motion = "reduced";
  } else {
    delete root.dataset.motion;
  }
}
