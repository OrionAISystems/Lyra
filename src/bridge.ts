import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AudioDeviceInfo, RuntimeStatus, SessionSnapshot, Settings, SetupArtifact, TranscriptionResult } from "./types";

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

export async function getSetupArtifacts(): Promise<SetupArtifact[]> {
  return (await command<SetupArtifact[]>("get_setup_artifacts")) ?? [];
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
  onResult: (result: TranscriptionResult) => void;
}): Promise<UnlistenFn[]> {
  if (!isTauri) return [];
  const unlisteners = await Promise.all([
    listen<SessionSnapshot>("lyra://state", (event) => handlers.onState(event.payload)),
    listen<number>("lyra://input-level", (event) => handlers.onInputLevel(event.payload)),
    listen<RuntimeStatus>("lyra://runtime-status", (event) => handlers.onRuntimeStatus(event.payload)),
    listen<TranscriptionResult>("lyra://transcription-result", (event) => handlers.onResult(event.payload)),
  ]);
  return unlisteners;
}
