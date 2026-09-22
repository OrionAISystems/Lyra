import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AudioDeviceInfo,
  BenchmarkResult,
  BarPlacementStatus,
  DisplayInfo,
  HotkeyStatus,
  ModelRuntimeStatus,
  Personalization,
  RuntimeStatus,
  SessionSnapshot,
  Settings,
  SetupArtifact,
  SetupProgress,
  TranscriptionPartial,
  TranscriptionResult,
  UpdateStatus,
  UsageStats,
} from "./types";
export const isTauri = "__TAURI_INTERNALS__" in window;
export async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  if (!isTauri) return null;
  return invoke<T>(name, args);
}
export const getSettings = () => command<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  command<Settings>("save_settings", { settings });
export const getPersonalization = () =>
  command<Personalization>("get_personalization");
export const savePersonalization = (personalization: Personalization) =>
  command<Personalization>("save_personalization", { personalization });
export const getRuntimeStatus = () =>
  command<RuntimeStatus>("get_runtime_status");
export const getModelRuntimeStatus = () =>
  command<ModelRuntimeStatus>("get_model_runtime_status");
export const listDisplays = async () =>
  (await command<DisplayInfo[]>("list_displays")) ?? [];
export const getBarPlacementStatus = () =>
  command<BarPlacementStatus>("get_bar_placement_status");
export const getUsageStats = () => command<UsageStats>("get_usage_stats");
export const resetUsageStats = () => command<UsageStats>("reset_usage_stats");
export const getHotkeyStatus = () => command<HotkeyStatus>("get_hotkey_status");
export const retryHotkey = () => command<HotkeyStatus>("retry_hotkey");
export const getSetupArtifacts = async () =>
  (await command<SetupArtifact[]>("get_setup_artifacts")) ?? [];
export const getSetupProgress = () =>
  command<SetupProgress>("get_setup_progress");
export const startSetupArtifact = (
  kind: string,
  variant: string | null = null,
) => command<void>("start_setup_artifact", { kind, variant });
export const cancelSetupArtifact = () => command<void>("cancel_setup_artifact");
export const getUpdateStatus = () => command<UpdateStatus>("get_update_status");
export const checkForUpdates = () => command<UpdateStatus>("check_for_updates");
export const installUpdate = () => command<UpdateStatus>("install_update");
export const deferUpdate = () => command<UpdateStatus>("defer_update");
export const getSessionSnapshot = () =>
  command<SessionSnapshot>("get_session_snapshot");
export const listAudioDevices = async () =>
  (await command<AudioDeviceInfo[]>("list_audio_devices")) ?? [];
export const startDictation = (source: "hotkey" | "bar" = "bar") =>
  command<void>("start_dictation", { source });
export const stopDictation = (source: "hotkey" | "bar" = "bar") =>
  command<void>("stop_dictation", { source });
export const cancelDictation = () => command<void>("cancel_dictation");
export const showSettings = () => command<void>("show_settings");
export const hideBar = () => command<void>("hide_bar");
export const quitLyra = () => command<void>("quit_lyra");
export const copyLastTranscript = () =>
  command<string | null>("copy_last_transcript");
export const hasOpenAiCredential = async () =>
  (await command<boolean>("has_openai_credential")) ?? false;
export const saveOpenAiCredential = (secret: string) =>
  command<void>("save_openai_credential", { secret });
export const clearOpenAiCredential = () =>
  command<void>("clear_openai_credential");
export const runBenchmark = () => command<void>("run_benchmark");
export const getBenchmarkResult = () =>
  command<BenchmarkResult | null>("get_benchmark_result");
export async function listenToBenchmarkEvents(h: {
  onState: (state: string) => void;
  onResult: (result: BenchmarkResult) => void;
}): Promise<UnlistenFn[]> {
  if (!isTauri) return [];
  return Promise.all([
    listen("lyra://benchmark-state", (e) => h.onState(e.payload as string)),
    listen("lyra://benchmark-result", (e) =>
      h.onResult(e.payload as BenchmarkResult),
    ),
  ]);
}
export async function listenToLyraEvents(h: {
  onState: (v: SessionSnapshot) => void;
  onInputLevel: (v: number) => void;
  onRuntimeStatus: (v: RuntimeStatus) => void;
  onModelRuntimeStatus: (v: ModelRuntimeStatus) => void;
  onSettings: (v: Settings) => void;
  onPersonalization: (v: Personalization) => void;
  onPlacementStatus: (v: BarPlacementStatus) => void;
  onUsage: (v: UsageStats) => void;
  onHotkeyStatus: (v: HotkeyStatus) => void;
  onSetupProgress: (v: SetupProgress) => void;
  onUpdateStatus: (v: UpdateStatus) => void;
  onResult: (v: TranscriptionResult) => void;
  onPartial: (v: TranscriptionPartial) => void;
}): Promise<UnlistenFn[]> {
  if (!isTauri) return [];
  return Promise.all([
    listen("lyra://state", (e) => h.onState(e.payload as SessionSnapshot)),
    listen("lyra://input-level", (e) => h.onInputLevel(e.payload as number)),
    listen("lyra://runtime-status", (e) =>
      h.onRuntimeStatus(e.payload as RuntimeStatus),
    ),
    listen("lyra://model-status", (e) =>
      h.onModelRuntimeStatus(e.payload as ModelRuntimeStatus),
    ),
    listen("lyra://settings", (e) => h.onSettings(e.payload as Settings)),
    listen("lyra://personalization", (e) =>
      h.onPersonalization(e.payload as Personalization),
    ),
    listen("lyra://placement-status", (e) =>
      h.onPlacementStatus(e.payload as BarPlacementStatus),
    ),
    listen("lyra://usage", (e) => h.onUsage(e.payload as UsageStats)),
    listen("lyra://hotkey-status", (e) =>
      h.onHotkeyStatus(e.payload as HotkeyStatus),
    ),
    listen("lyra://setup-progress", (e) =>
      h.onSetupProgress(e.payload as SetupProgress),
    ),
    listen("lyra://update-status", (e) =>
      h.onUpdateStatus(e.payload as UpdateStatus),
    ),
    listen("lyra://transcription-result", (e) =>
      h.onResult(e.payload as TranscriptionResult),
    ),
    listen("lyra://transcription-partial", (e) =>
      h.onPartial(e.payload as TranscriptionPartial),
    ),
  ]);
}
