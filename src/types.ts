export type ActivationMode = "push_to_talk" | "toggle";
export type DictationState = "idle" | "listening" | "processing" | "success" | "error";
export type LanguagePreference = "auto" | "en" | "el";
export type TranscriptionRoute = "local_only" | "local_first_cloud_fallback" | "cloud_only";

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
  keepModelWarm: boolean;
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

export interface SetupArtifact {
  kind: "runtime" | "model";
  name: string;
  source: string;
  sizeBytes: number;
  destination: string;
  checksumSource: string;
  revision: string | null;
  license: string | null;
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
  schemaVersion: 1,
  activationMode: "push_to_talk",
  hotkey: "Ctrl+Alt+Space",
  showBar: true,
  language: "auto",
  route: "local_first_cloud_fallback",
  audioDeviceId: null,
  maxDurationSeconds: 120,
  autoPaste: true,
  launchAtLogin: false,
  keepModelWarm: true,
  theme: "dark",
  onboardingComplete: false,
};
