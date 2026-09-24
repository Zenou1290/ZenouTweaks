/** Shared types mirroring the Rust backend (serde camelCase). */

export type Risk = "low" | "medium" | "high";

export type TweakCategory =
  | "performance"
  | "gaming"
  | "gpu"
  | "network"
  | "cleanup"
  | "windows";

export type TweakState =
  | "available"
  | "applied"
  | "partial"
  | "unsupported"
  | "unknown";

export type OperationKind = "applying" | "restoring";

export interface TweakMeta {
  id: string;
  name: string;
  description: string;
  category: TweakCategory;
  risk: Risk;
  requiresAdmin: boolean;
  restartRequired: boolean;
  effect: string;
  quickOptimize: boolean;
}

export interface ChangePreview {
  location: string;
  valueName: string | null;
  current: string;
  new: string;
}

export interface TweakView {
  meta: TweakMeta;
  state: TweakState;
  previews: ChangePreview[];
  favorite: boolean;
}

export interface StepOutcome {
  location: string;
  ok: boolean;
  error: string | null;
}

export interface TweakResult {
  tweakId: string;
  success: boolean;
  skipped: boolean;
  error: string | null;
  details: string | null;
  restartRequired: boolean;
  steps: StepOutcome[];
}

export interface GroupResult {
  label: string;
  results: TweakResult[];
  restorePointCreated: boolean;
}

export interface FlaggedEntry {
  kind: "blocked" | "excluded";
  operation: string;
  reason: string;
}

export interface GpuInfo {
  name: string;
  vendor: "nvidia" | "amd" | "intel" | "other";
  driverVersion: string | null;
}

export interface DiskInfo {
  model: string;
  sizeGb: number | null;
  driveLetter: string | null;
  totalGb: number | null;
  freeGb: number | null;
}

export interface NetAdapterInfo {
  name: string;
  description: string;
  interfaceGuid: string;
  status: string;
}

export interface SystemScan {
  windowsEdition: string;
  windowsVersion: string;
  windowsBuild: string;
  architecture: string;
  cpuName: string;
  cpuCores: number;
  cpuThreads: number;
  cpuVendor: string;
  ramGb: number | null;
  gpus: GpuInfo[];
  storage: DiskInfo[];
  adapters: NetAdapterInfo[];
  uptime: string;
  isElevated: boolean;
  hostname: string;
  username: string;
}

export interface Snapshot {
  id: string;
  label: string;
  createdAt: string;
  automatic: boolean;
  entries: { tweakId: string; tweakName: string }[];
}

export interface ChangeSetSummary {
  id: string;
  label: string;
  createdAt: string;
  backupId: string | null;
  applied: number;
  failed: number;
  skipped: number;
}

export interface ChangeSet {
  id: string;
  label: string;
  createdAt: string;
  backupId: string | null;
  results: TweakResult[];
}

export interface LogEntry {
  timestamp: string;
  action: string;
  tweakId: string | null;
  tweakName: string | null;
  result: string;
  message: string;
  errorDetails: string | null;
}

export interface CleanupPreview {
  id: string;
  name: string;
  description: string;
  fileCount: number;
  totalBytes: number;
  irreversible: boolean;
}

export interface Profile {
  id: string;
  name: string;
  description: string;
  tweakIds: string[];
  builtin: boolean;
}

export interface AppSettings {
  startWithWindows: boolean;
  minimizeToTray: boolean;
  theme: string;
  accent: string;
  notifications: boolean;
  restorePointRisky: boolean;
  confirmBulk: boolean;
  reducedMotion: boolean;
  favorites: string[];
}

export const DEFAULT_SETTINGS: AppSettings = {
  startWithWindows: false,
  minimizeToTray: true,
  theme: "dark",
  accent: "teal",
  notifications: true,
  restorePointRisky: true,
  confirmBulk: true,
  reducedMotion: false,
  favorites: [],
};
