/** Typed invoke wrappers for the Rust backend. */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ChangeSet,
  ChangeSetSummary,
  CleanupPreview,
  FlaggedEntry,
  GroupResult,
  LogEntry,
  Profile,
  Snapshot,
  SystemScan,
  TweakResult,
  TweakView,
} from "@/types";

/** Error shape returned by the Rust ZenouError (serialized camelCase). */
export interface ZenouErrorShape {
  code: string;
  message: string;
  details: string | null;
}

export function toMessage(err: unknown): { message: string; details: string | null } {
  if (typeof err === "string") {
    try {
      const parsed = JSON.parse(err) as ZenouErrorShape;
      if (parsed.message) return { message: parsed.message, details: parsed.details ?? null };
    } catch {
      return { message: err, details: null };
    }
  }
  if (err && typeof err === "object" && "message" in err) {
    const e = err as ZenouErrorShape;
    return { message: e.message, details: e.details ?? null };
  }
  return { message: "Something went wrong. Please try again.", details: String(err) };
}

export const api = {
  // system
  scanSystem: (refresh?: boolean) => invoke<SystemScan>("scan_system", { refresh }),
  isElevated: () => invoke<boolean>("is_elevated"),
  flaggedOperations: () => invoke<FlaggedEntry[]>("flagged_operations"),

  // tweaks
  listTweaks: () => invoke<TweakView[]>("list_tweaks"),
  tweakDetails: (id: string) => invoke<TweakView>("tweak_details", { id }),
  applyTweak: (id: string) => invoke<TweakResult>("apply_tweak", { id }),
  restoreTweak: (id: string) => invoke<TweakResult>("restore_tweak", { id }),
  applyGroup: (ids: string[], label: string) =>
    invoke<GroupResult>("apply_group", { ids, label }),

  // profiles
  listProfiles: () => invoke<Profile[]>("list_profiles"),
  saveCustomProfile: (name: string, tweakIds: string[]) =>
    invoke<void>("save_custom_profile", { name, tweakIds }),
  deleteCustomProfile: (name: string) =>
    invoke<void>("delete_custom_profile", { name }),

  // backups
  createBackup: (label: string) => invoke<Snapshot>("create_backup", { label }),
  listBackups: () => invoke<Snapshot[]>("list_backups"),
  deleteBackup: (id: string) => invoke<void>("delete_backup", { id }),
  restoreBackup: (id: string) => invoke<GroupResult>("restore_backup", { id }),

  // changes
  listChanges: () => invoke<ChangeSetSummary[]>("list_changes"),
  getChange: (id: string) => invoke<ChangeSet>("get_change", { id }),
  restoreChangeSet: (id: string) => invoke<GroupResult>("restore_change_set", { id }),

  // logs
  readLogs: () => invoke<LogEntry[]>("read_logs"),
  clearLogs: () => invoke<void>("clear_logs"),
  exportLogs: (target: string) => invoke<string>("export_logs", { target }),
  pickSaveFile: (fileName: string) =>
    invoke<string | null>("pick_save_file", { fileName }),

  // settings
  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) => invoke<void>("set_settings", { settings }),
  toggleFavorite: (id: string) => invoke<string[]>("toggle_favorite", { id }),

  // cleanup
  cleanupPreviews: () => invoke<CleanupPreview[]>("cleanup_previews"),
  runCleanup: (id: string) =>
    invoke<[number, number, number]>("run_cleanup", { id }),

  // restore point / elevation
  createRestorePoint: () => invoke<string>("create_restore_point"),
  requestElevation: () => invoke<void>("request_elevation"),
};
