import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ThemeContent, ThemeStatus } from "./themes";
import type {
  AudioDeviceInfo,
  BenchmarkResult,
  BarPlacementStatus,
  DisplayInfo,
  HotkeyStatus,
  ModelRuntimeStatus,
  Personalization,
  RuntimeStatus,
  SavedItemDetail,
  SavedItemPage,
  SavedItemSummary,
  SessionSnapshot,
  Settings,
  SetupArtifact,
  SetupArtifactKind,
  SetupProgress,
  TranscriptionPartial,
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
export const getThemeStatus = () => command<ThemeStatus>("get_theme_status");
export const getThemeContent = (themeId?: string) => command<ThemeContent>("get_theme_content", { themeId: themeId ?? null });
export const refreshThemes = () => command<ThemeStatus>("refresh_themes");
export const listenToThemeChanges = async (onChange: (status: ThemeStatus) => void) =>
  isTauri ? listen<ThemeStatus>("lyra://theme-changed", (event) => onChange(event.payload)) : () => {};
export const saveSettings = (settings: Settings) =>
  command<Settings>("save_settings", { settings });
export const setShortcutRecording = (recording: boolean) =>
  command<void>("set_shortcut_recording", { recording });
export const listenToRecordedShortcut = async (onRecorded: (shortcut: string) => void) =>
  isTauri ? listen<string>("lyra://shortcut-recorded", (event) => onRecorded(event.payload)) : () => {};
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
export const getScratchpadHotkeyStatus = () =>
  command<HotkeyStatus>("get_scratchpad_hotkey_status");
export const retryHotkey = () => command<HotkeyStatus>("retry_hotkey");
export const getSetupArtifacts = async () =>
  (await command<SetupArtifact[]>("get_setup_artifacts")) ?? [];
export const getSetupProgress = () =>
  command<SetupProgress>("get_setup_progress");
export const startSetupArtifact = (
  kind: SetupArtifactKind,
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
export const startMicrophoneTest = () => command<void>("start_microphone_test");
export const stopMicrophoneTest = () => command<void>("stop_microphone_test");
export const startDictation = (source: "hotkey" | "bar" = "bar") =>
  command<void>("start_dictation", { source });
export const stopDictation = (source: "hotkey" | "bar" = "bar") =>
  command<void>("stop_dictation", { source });
export const cancelDictation = () => command<void>("cancel_dictation");
export const showSettings = () => command<void>("show_settings");
export const hideBar = () => command<void>("hide_bar");
export const setBarMenuVisible = (open: boolean) =>
  command<void>("set_bar_menu_visible", { open });
export const quitLyra = () => command<void>("quit_lyra");
export const copyLastTranscript = () =>
  command<string | null>("copy_last_transcript");
export const listHistory = (offset = 0, limit = 50) =>
  command<SavedItemPage>("list_history", { offset, limit });
export const getHistoryItem = (id: string) =>
  command<SavedItemDetail>("get_history_item", { id });
export const deleteHistoryItem = (id: string) =>
  command<void>("delete_history_item", { id });
export const clearHistory = () => command<void>("clear_history");
export const promoteHistoryItem = (id: string) =>
  command<SavedItemSummary>("promote_history_item", { id });
export const listScratchpad = (offset = 0, limit = 50) =>
  command<SavedItemPage>("list_scratchpad", { offset, limit });
export const getScratchpadItem = (id: string) =>
  command<SavedItemDetail>("get_scratchpad_item", { id });
export const deleteScratchpadItem = (id: string) =>
  command<void>("delete_scratchpad_item", { id });
export const setScratchpadPinned = (id: string, pinned: boolean) =>
  command<SavedItemSummary>("set_scratchpad_pinned", { id, pinned });
export const copySavedItem = (collection: "history" | "scratchpad", id: string) =>
  command<void>("copy_saved_item", { collection, id });
export const pasteSavedItem = (collection: "history" | "scratchpad", id: string) =>
  command<void>("paste_saved_item", { collection, id });
export const startScratchpad = () => command<void>("start_scratchpad");
export const stopScratchpad = () => command<void>("stop_scratchpad");
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
export async function listenToMicrophoneTestEvents(h: {
  onLevel: (level: number) => void;
  onStopped: () => void;
  onError: (message: string) => void;
}): Promise<UnlistenFn[]> {
  if (!isTauri) return [];
  return Promise.all([
    listen("lyra://microphone-test-level", (e) => h.onLevel(e.payload as number)),
    listen("lyra://microphone-test-stopped", () => h.onStopped()),
    listen("lyra://microphone-test-error", (e) => h.onError(e.payload as string)),
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
  onScratchpadHotkeyStatus: (v: HotkeyStatus) => void;
  onSetupProgress: (v: SetupProgress) => void;
  onUpdateStatus: (v: UpdateStatus) => void;
  onPartial: (v: TranscriptionPartial) => void;
  onSavedItemsUpdated: () => void;
  onScratchpadOpened: (id: string) => void;
  onSavedItemNotice: (notice: string) => void;
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
    listen("lyra://scratchpad-hotkey-status", (e) =>
      h.onScratchpadHotkeyStatus(e.payload as HotkeyStatus),
    ),
    listen("lyra://setup-progress", (e) =>
      h.onSetupProgress(e.payload as SetupProgress),
    ),
    listen("lyra://update-status", (e) =>
      h.onUpdateStatus(e.payload as UpdateStatus),
    ),
    listen("lyra://transcription-partial", (e) =>
      h.onPartial(e.payload as TranscriptionPartial),
    ),
    listen("lyra://saved-items-updated", () => h.onSavedItemsUpdated()),
    listen("lyra://open-scratchpad", (e) =>
      h.onScratchpadOpened(e.payload as string),
    ),
    listen("lyra://saved-item-notice", (e) =>
      h.onSavedItemNotice(e.payload as string),
    ),
  ]);
}
