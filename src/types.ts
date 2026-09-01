export type ActivationMode = "push_to_talk" | "toggle";
export type DictationState = "idle" | "listening" | "processing" | "success" | "error";
export type LanguagePreference = "auto" | "en" | "el";
export type TranscriptionRoute = "local_only" | "local_first_cloud_fallback" | "cloud_only";
export type BarPlacement = "follow_active_window" | "primary_display" | "fixed_monitor";
export type ModelWarmPolicy = "always_warm" | "unload_after_5_minutes" | "unload_after_15_minutes" | "unload_after_30_minutes" | "never_keep_warm";
export type ModelLoadState = "loading" | "ready" | "unloaded" | "error";

export interface CorrectionEntry {
  source: string;
  replacement: string;
  enabled: boolean;
  caseSensitive: boolean;
}

export interface Settings {
  schemaVersion: number;
  activationMode: ActivationMode;
  hotkey: string;
  showBar: boolean;
  language: LanguagePreference;
  route: TranscriptionRoute;
  audioDeviceId: string | null;
  maxDurationSeconds: number;
  autoPaste: boolean;
  launchAtLogin: boolean;
  autoStopAfterSilenceMs: number | null;
  formattingCommandsEnabled: boolean;
  corrections: CorrectionEntry[];
  barPlacement: BarPlacement;
  barMonitorId: string | null;
  barMonitorIndex: number | null;
  modelWarmPolicy: ModelWarmPolicy;
  usageCostEstimatesEnabled: boolean;
  cloudCostPerMinuteUsd: number;
  theme: "system" | "dark";
  onboardingComplete: boolean;
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  isDefault: boolean;
  sampleRate: number;
  channels: number;
  format: string;
}

export interface RuntimeStatus {
  runtimeInstalled: boolean;
  runtimeVersion: string | null;
  runtimeVariant: "cuda" | "vulkan" | "cpu" | null;
  runtimePath: string;
  runtimeSha256: string | null;
  runtimeVerified: boolean;
  modelInstalled: boolean;
  modelName: string | null;
  modelRevision: string | null;
  modelPath: string;
  modelSizeBytes: number | null;
  modelVerified: boolean;
  gpuName: string | null;
  gpuVramBytes: number | null;
  lastError: string | null;
}

export interface ModelRuntimeStatus {
  state: ModelLoadState;
  detail: string | null;
}

export interface DisplayInfo {
  id: string;
  name: string | null;
  index: number;
  primary: boolean;
  positionX: number;
  positionY: number;
  width: number;
  height: number;
  workAreaX: number;
  workAreaY: number;
  workAreaWidth: number;
  workAreaHeight: number;
  scaleFactor: number;
}

export interface BarPlacementStatus {
  placement: BarPlacement;
  monitorId: string | null;
  monitorIndex: number | null;
  fallback: boolean;
  message: string | null;
}

export interface SetupArtifact {
  kind: "runtime" | "model";
  variant: string | null;
  name: string;
  source: string;
  sizeBytes: number;
  destination: string;
  checksumSource: string;
  expectedSha256: string;
  revision: string | null;
  license: string | null;
}

export type SetupProgressState = "idle" | "downloading" | "verifying" | "installing" | "complete" | "cancelled" | "error";

export interface SetupProgress {
  kind: "runtime" | "model" | null;
  variant: string | null;
  state: SetupProgressState;
  downloadedBytes: number;
  totalBytes: number;
  error: string | null;
}

export type UpdateState = "idle" | "checking" | "up_to_date" | "available" | "downloading" | "installing" | "error";

export interface UpdateStatus {
  state: UpdateState;
  currentVersion: string;
  availableVersion: string | null;
  notes: string | null;
  publishedAt: string | null;
  progress: number | null;
  lastCheckedAt: string | null;
  error: string | null;
}

export interface SessionSnapshot {
  state: DictationState;
  durationMs: number;
  inputLevel: number;
  requestId: string | null;
  error: string | null;
  transcript: string | null;
  backend: string | null;
  fallbackReason: string | null;
}

export interface HotkeyStatus {
  hotkey: string;
  registered: boolean;
  error: string | null;
}

export interface TranscriptionResult {
  text: string;
  backend: string;
  model: string;
  language: string | null;
  audioDurationMs: number;
  transcriptionDurationMs: number;
  latencyMs: number;
  fallbackReason: string | null;
}

export interface UsageStats {
  dictationsToday: number;
  dictationsAllTime: number;
  minutesDictated: number;
  completedResults: number;
  averageTranscriptionLatencyMs: number;
  averageRealTimeFactor: number;
  localResultCount: number;
  cloudResultCount: number;
  localPercent: number;
  cloudPercent: number;
  failedTranscriptions: number;
  pasteAttempts: number;
  pasteSuccesses: number;
  pasteSuccessRate: number;
  cloudFallbacks: number;
  costEstimatesEnabled: boolean;
  cloudCostPerMinuteUsd: number;
  estimatedCloudCostUsd: number | null;
  estimatedLocalCostAvoidedUsd: number | null;
}

export interface OnboardingStatus {
  microphone: boolean;
  gpu: boolean;
  runtime: boolean;
  model: boolean;
  hotkey: boolean;
  recording: boolean;
  ready: boolean;
}

export const defaultSettings: Settings = {
  schemaVersion: 2,
  activationMode: "push_to_talk",
  hotkey: "Ctrl+Alt+Space",
  showBar: true,
  language: "auto",
  route: "local_first_cloud_fallback",
  audioDeviceId: null,
  maxDurationSeconds: 120,
  autoPaste: true,
  launchAtLogin: false,
  autoStopAfterSilenceMs: null,
  formattingCommandsEnabled: true,
  corrections: [
    { source: "orion ai systems", replacement: "Orion AI Systems", enabled: true, caseSensitive: false },
    { source: "geo core", replacement: "GeoCore", enabled: true, caseSensitive: false },
    { source: "war map", replacement: "WarMap", enabled: true, caseSensitive: false },
  ],
  barPlacement: "follow_active_window",
  barMonitorId: null,
  barMonitorIndex: null,
  modelWarmPolicy: "always_warm",
  usageCostEstimatesEnabled: false,
  cloudCostPerMinuteUsd: 0.003,
  theme: "dark",
  onboardingComplete: false,
};
