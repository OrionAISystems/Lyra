import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AudioDeviceInfo, BarPlacementStatus, DisplayInfo, HotkeyStatus, ModelRuntimeStatus, RuntimeStatus, SessionSnapshot, Settings, SetupArtifact, SetupProgress, TranscriptionResult, UpdateStatus, UsageStats } from "./types";

export const isTauri = "__TAURI_INTERNALS__" in window;

export async function command<T>(name: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauri) return null;
  return invoke<T>(name, args);
}

export async function getSettings(): Promise<Settings | null> {
  return command<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<Settings | null> {
  return command<Settings>("save_settings", { settings });
}

export async function getRuntimeStatus(): Promise<RuntimeStatus | null> {
  return command<RuntimeStatus>("get_runtime_status");
}

export async function getModelRuntimeStatus(): Promise<ModelRuntimeStatus | null> {
  return command<ModelRuntimeStatus>("get_model_runtime_status");
}

export async function listDisplays(): Promise<DisplayInfo[]> {
  return (await command<DisplayInfo[]>("list_displays")) ?? [];
}

export async function getBarPlacementStatus(): Promise<BarPlacementStatus | null> {
  return command<BarPlacementStatus>("get_bar_placement_status");
}

export async function getUsageStats(): Promise<UsageStats | null> {
  return command<UsageStats>("get_usage_stats");
}

export async function resetUsageStats(): Promise<UsageStats | null> {
  return command<UsageStats>("reset_usage_stats");
}

export async function getHotkeyStatus(): Promise<HotkeyStatus | null> {
  return command<HotkeyStatus>("get_hotkey_status");
}

export async function retryHotkey(): Promise<HotkeyStatus | null> {
  return command<HotkeyStatus>("retry_hotkey");
}

export async function getSetupArtifacts(): Promise<SetupArtifact[]> {
  return (await command<SetupArtifact[]>("get_setup_artifacts")) ?? [];
}

export async function getSetupProgress(): Promise<SetupProgress | null> {
  return command<SetupProgress>("get_setup_progress");
}

export async function startSetupArtifact(kind: "runtime" | "model", variant: string | null = null): Promise<void> {
  await command("start_setup_artifact", { kind, variant });
}

export async function cancelSetupArtifact(): Promise<void> {
  await command("cancel_setup_artifact");
}

export async function getUpdateStatus(): Promise<UpdateStatus | null> {
  return command<UpdateStatus>("get_update_status");
}

export async function checkForUpdates(): Promise<UpdateStatus | null> {
  return command<UpdateStatus>("check_for_updates");
}

export async function installUpdate(): Promise<UpdateStatus | null> {
  return command<UpdateStatus>("install_update");
}

export async function deferUpdate(): Promise<UpdateStatus | null> {
  return command<UpdateStatus>("defer_update");
}

export async function getSessionSnapshot(): Promise<SessionSnapshot | null> {
  return command<SessionSnapshot>("get_session_snapshot");
}

export async function listAudioDevices(): Promise<AudioDeviceInfo[]> {
  return (await command<AudioDeviceInfo[]>("list_audio_devices")) ?? [];
}

export async function startDictation(source: "hotkey" | "bar" = "bar"): Promise<void> {
  await command("start_dictation", { source });
}

export async function stopDictation(source: "hotkey" | "bar" = "bar"): Promise<void> {
  await command("stop_dictation", { source });
}

export async function cancelDictation(): Promise<void> {
  await command("cancel_dictation");
}

export async function showSettings(): Promise<void> {
  await command("show_settings");
}

export async function hideBar(): Promise<void> {
  await command("hide_bar");
}

export async function quitLyra(): Promise<void> {
  await command("quit_lyra");
}

export async function copyLastTranscript(): Promise<string | null> {
  return command<string | null>("copy_last_transcript");
}

export async function hasOpenAiCredential(): Promise<boolean> {
  return (await command<boolean>("has_openai_credential")) ?? false;
}

export async function saveOpenAiCredential(secret: string): Promise<void> {
  await command("save_openai_credential", { secret });
}

export async function clearOpenAiCredential(): Promise<void> {
  await command("clear_openai_credential");
}

export async function listenToLyraEvents(handlers: {
  onState: (snapshot: SessionSnapshot) => void;
  onInputLevel: (level: number) => void;
  onRuntimeStatus: (status: RuntimeStatus) => void;
  onModelRuntimeStatus: (status: ModelRuntimeStatus) => void;
  onSettings: (settings: Settings) => void;
  onPlacementStatus: (status: BarPlacementStatus) => void;
  onUsage: (stats: UsageStats) => void;
  onHotkeyStatus: (status: HotkeyStatus) => void;
  onSetupProgress: (progress: SetupProgress) => void;
  onUpdateStatus: (status: UpdateStatus) => void;
  onResult: (result: TranscriptionResult) => void;
}): Promise<UnlistenFn[]> {
  if (!isTauri) return [];
  const unlisteners = await Promise.all([
    listen<SessionSnapshot>("lyra://state", (event) => handlers.onState(event.payload)),
    listen<number>("lyra://input-level", (event) => handlers.onInputLevel(event.payload)),
    listen<RuntimeStatus>("lyra://runtime-status", (event) => handlers.onRuntimeStatus(event.payload)),
    listen<ModelRuntimeStatus>("lyra://model-status", (event) => handlers.onModelRuntimeStatus(event.payload)),
    listen<Settings>("lyra://settings", (event) => handlers.onSettings(event.payload)),
    listen<BarPlacementStatus>("lyra://placement-status", (event) => handlers.onPlacementStatus(event.payload)),
    listen<UsageStats>("lyra://usage", (event) => handlers.onUsage(event.payload)),
    listen<HotkeyStatus>("lyra://hotkey-status", (event) => handlers.onHotkeyStatus(event.payload)),
    listen<SetupProgress>("lyra://setup-progress", (event) => handlers.onSetupProgress(event.payload)),
    listen<UpdateStatus>("lyra://update-status", (event) => handlers.onUpdateStatus(event.payload)),
    listen<TranscriptionResult>("lyra://transcription-result", (event) => handlers.onResult(event.payload)),
  ]);
  return unlisteners;
}
