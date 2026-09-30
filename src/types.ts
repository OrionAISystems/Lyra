export type ActivationMode = "push_to_talk" | "toggle";
export type DictationState =
  | "idle"
  | "listening"
  | "processing"
  | "success"
  | "error";
export type LanguagePreference = "auto" | "en" | "el";
export type TranscriptionRoute =
  | "local_only"
  | "local_first_cloud_fallback"
  | "cloud_only";
export type BarPlacement =
  | "follow_active_window"
  | "primary_display"
  | "fixed_monitor";
export type ModelWarmPolicy =
  | "always_warm"
  | "unload_after_5_minutes"
  | "unload_after_15_minutes"
  | "unload_after_30_minutes"
  | "never_keep_warm";
export type LocalEngine = "parakeet_tdt" | "nemotron_streaming_en";
export type TranscriptHistoryRetention =
  | "off"
  | "seven_days"
  | "thirty_days"
  | "ninety_days"
  | "one_year"
  | "forever";
export type SessionDestination = "dictation" | "scratchpad";
export type Screen =
  | "home"
  | "dictation"
  | "personalization"
  | "audio"
  | "insights"
  | "history"
  | "scratchpad"
  | "appearance"
  | "diagnostics";
export interface CorrectionEntry {
  source: string;
  replacement: string;
  enabled: boolean;
  caseSensitive: boolean;
}
export interface WordEntry {
  phrase: string;
  enabled: boolean;
  boost: number;
}
export interface SnippetEntry {
  trigger: string;
  expansion: string;
  enabled: boolean;
  caseSensitive: boolean;
}
export interface AppProfile {
  executable: string;
  category: "terminal" | "developer" | "chat" | "editor" | "other";
  injectionStrategy: "ctrl_v" | "shift_insert" | "clipboard_only";
  chunkLongText: boolean;
  chunkChars: number;
  formattingEnabled: boolean;
}
export interface Personalization {
  schemaVersion: number;
  words: WordEntry[];
  corrections: CorrectionEntry[];
  snippets: SnippetEntry[];
  profiles: AppProfile[];
  selfCorrectionEnabled: boolean;
}
export interface RecoveryShortcuts {
  pasteLast: string | null;
  copyLast: string | null;
  cancel: string | null;
  handsFreeToggle: string | null;
  openSettings: string | null;
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
  recoveryShortcuts: RecoveryShortcuts;
  barPlacement: BarPlacement;
  barMonitorId: string | null;
  barMonitorIndex: number | null;
  modelWarmPolicy: ModelWarmPolicy;
  usageCostEstimatesEnabled: boolean;
  cloudCostPerMinuteUsd: number;
  typingWpm: number;
  theme: "dark" | "light" | "system";
  accent: "purple" | "blue" | "teal" | "rose";
  compactMode: boolean;
  showTrayIcon: boolean;
  localEngine: LocalEngine;
  adaptiveAudioEnabled: boolean;
  onboardingComplete: boolean;
  transcriptHistoryRetention: TranscriptHistoryRetention;
  historyLastRetention: TranscriptHistoryRetention;
  scratchpadHotkey: string;
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
  nemotronModelInstalled: boolean;
  nemotronModelVerified: boolean;
  nemotronModelPath: string;
  gpuName: string | null;
  gpuVramBytes: number | null;
  lastError: string | null;
}
export interface ModelRuntimeStatus {
  state: "loading" | "ready" | "unloaded" | "error";
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
export type SetupArtifactKind =
  | "runtime"
  | "model-parakeet"
  | "model-nemotron";
export interface SetupArtifact {
  kind: SetupArtifactKind;
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
export interface SetupProgress {
  kind: SetupArtifactKind | null;
  variant: string | null;
  state:
    | "idle"
    | "downloading"
    | "verifying"
    | "installing"
    | "complete"
    | "cancelled"
    | "error";
  downloadedBytes: number;
  totalBytes: number;
  error: string | null;
}
export interface BenchmarkScore {
  engine: string;
  runtime: string | null;
  latencyMs: number;
  realTimeFactor: number;
  transcriptDistance: number;
  partialCount: number;
  succeeded: boolean;
  failureCategory: string | null;
}
export interface BenchmarkResult {
  timestampMs: number;
  phraseWordCount: number;
  microphone: string | null;
  scores: BenchmarkScore[];
  recommendation: string | null;
}
export interface UpdateStatus {
  state:
    | "idle"
    | "checking"
    | "up_to_date"
    | "available"
    | "downloading"
    | "installing"
    | "error";
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
  backend: string | null;
  fallbackReason: string | null;
  destination: SessionDestination | null;
}
export interface TranscriptionPartial {
  requestId: string;
  text: string;
  processedDurationMs: number | null;
  backend: string;
  model: string;
  finality: boolean;
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
export interface DailyUsage {
  day: string;
  sessions: number;
  completed: number;
  failed: number;
  durationMs: number;
  words: number;
  parakeetWords: number;
  nemotronWords: number;
  pasteAttempts: number;
  pasteSuccesses: number;
}
export interface SessionRecord {
  timestampMs: number;
  appVersion: string;
  durationMs: number;
  wordCount: number;
  targetExecutable: string | null;
  targetCategory: string | null;
  endpoint: string;
  engine: string;
  runtime: string | null;
  backend: string | null;
  fallbackChain: string[];
  latencyMs: number | null;
  realTimeFactor: number | null;
  pasteOutcome: string;
  failureCategory: string | null;
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
  totalWords: number;
  estimatedTypingHoursSaved: number;
  detailedSince: string | null;
  daily: DailyUsage[];
  recentSessions: SessionRecord[];
  failureCounts: Record<string, number>;
}
export interface SavedItemSummary {
  id: string;
  timestampMs: number;
  durationMs: number;
  wordCount: number;
  targetApp: string | null;
  targetExecutable: string | null;
  engine: string;
  backend: string;
  model: string;
  latencyMs: number;
  pasteOutcome: string;
  pinned: boolean;
}
export interface SavedItemPage {
  items: SavedItemSummary[];
  total: number;
  offset: number;
  hasMore: boolean;
}
export interface SavedItemDetail {
  summary: SavedItemSummary;
  text: string;
}
export const defaultSettings: Settings = {
  schemaVersion: 4,
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
  recoveryShortcuts: {
    pasteLast: null,
    copyLast: null,
    cancel: "Escape",
    handsFreeToggle: null,
    openSettings: "Ctrl+Alt+Shift+L",
  },
  barPlacement: "follow_active_window",
  barMonitorId: null,
  barMonitorIndex: null,
  modelWarmPolicy: "always_warm",
  usageCostEstimatesEnabled: false,
  cloudCostPerMinuteUsd: 0.003,
  typingWpm: 40,
  theme: "dark",
  accent: "purple",
  compactMode: false,
  showTrayIcon: true,
  localEngine: "parakeet_tdt",
  adaptiveAudioEnabled: true,
  onboardingComplete: false,
  transcriptHistoryRetention: "off",
  historyLastRetention: "thirty_days",
  scratchpadHotkey: "Ctrl+Alt+Shift+Space",
};
export const defaultPersonalization: Personalization = {
  schemaVersion: 1,
  words: [],
  corrections: [
    {
      source: "orion ai systems",
      replacement: "Orion AI Systems",
      enabled: true,
      caseSensitive: false,
    },
    {
      source: "geo core",
      replacement: "GeoCore",
      enabled: true,
      caseSensitive: false,
    },
    {
      source: "war map",
      replacement: "WarMap",
      enabled: true,
      caseSensitive: false,
    },
  ],
  snippets: [],
  profiles: [],
  selfCorrectionEnabled: true,
};
