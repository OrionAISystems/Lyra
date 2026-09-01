import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import {
  Activity,
  ArrowLeft,
  BarChart3,
  Check,
  ChevronRight,
  CircleHelp,
  Clipboard,
  Cloud,
  Cpu,
  Download,
  Gauge,
  Globe2,
  Keyboard,
  Mic,
  Monitor,
  MoreHorizontal,
  Plus,
  Play,
  Power,
  Radio,
  RotateCcw,
  RefreshCw,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Square,
  Timer,
  Trash2,
  Volume2,
  X,
} from "lucide-react";
import {
  cancelDictation,
  copyLastTranscript,
  getRuntimeStatus,
  getModelRuntimeStatus,
  getBarPlacementStatus,
  getSetupArtifacts,
  getSessionSnapshot,
  getSettings,
  hasOpenAiCredential,
  hideBar,
  listDisplays,
  listAudioDevices,
  isTauri,
  listenToLyraEvents,
  quitLyra,
  retryHotkey,
  getUsageStats,
  resetUsageStats,
  saveSettings,
  saveOpenAiCredential,
  clearOpenAiCredential,
  cancelSetupArtifact,
  checkForUpdates,
  deferUpdate,
  getHotkeyStatus,
  getSetupProgress,
  getUpdateStatus,
  installUpdate,
  showSettings,
  startSetupArtifact,
  startDictation,
  stopDictation,
} from "./bridge";
import type {
  ActivationMode,
  BarPlacement,
  BarPlacementStatus,
  DictationState,
  DisplayInfo,
  LanguagePreference,
  AudioDeviceInfo,
  HotkeyStatus,
  ModelRuntimeStatus,
  ModelWarmPolicy,
  RuntimeStatus,
  SessionSnapshot,
  Settings,
  SetupArtifact,
  SetupProgress,
  TranscriptionRoute,
  TranscriptionResult,
  UpdateStatus,
  UsageStats,
  CorrectionEntry,
} from "./types";
import { defaultSettings } from "./types";
import { isPrimaryPointerButton, primaryBarGesture } from "./barActivation";
import lyraIcon from "./assets/lyra-icon.png";
import packageJson from "../package.json";

type SettingsSection = "general" | "dictation" | "audio" | "language" | "transcription" | "cloud" | "appearance" | "vocabulary" | "usage" | "diagnostics";

const initialRuntime: RuntimeStatus = {
  runtimeInstalled: false,
  runtimeVersion: null,
  runtimeVariant: null,
  runtimePath: "%LOCALAPPDATA%\\Lyra\\runtime",
  runtimeSha256: null,
  runtimeVerified: false,
  modelInstalled: false,
  modelName: null,
  modelRevision: null,
  modelPath: "%LOCALAPPDATA%\\Lyra\\models",
  modelSizeBytes: null,
  modelVerified: false,
  gpuName: "Detecting hardware…",
  gpuVramBytes: null,
  lastError: null,
};

const initialSession: SessionSnapshot = {
  state: "idle",
  durationMs: 0,
  inputLevel: 0,
  requestId: null,
  error: null,
  transcript: null,
  backend: null,
  fallbackReason: null,
};

const initialModelRuntime: ModelRuntimeStatus = {
  state: "unloaded",
  detail: null,
};

const initialPlacement: BarPlacementStatus = {
  placement: "follow_active_window",
  monitorId: null,
  monitorIndex: null,
  fallback: false,
  message: null,
};

const initialUsage: UsageStats = {
  dictationsToday: 0,
  dictationsAllTime: 0,
  minutesDictated: 0,
  completedResults: 0,
  averageTranscriptionLatencyMs: 0,
  averageRealTimeFactor: 0,
  localResultCount: 0,
  cloudResultCount: 0,
  localPercent: 0,
  cloudPercent: 0,
  failedTranscriptions: 0,
  pasteAttempts: 0,
  pasteSuccesses: 0,
  pasteSuccessRate: 0,
  cloudFallbacks: 0,
  costEstimatesEnabled: false,
  cloudCostPerMinuteUsd: 0.003,
  estimatedCloudCostUsd: null,
  estimatedLocalCostAvoidedUsd: null,
};

const initialHotkey: HotkeyStatus = {
  hotkey: defaultSettings.hotkey,
  registered: false,
  error: "Waiting for Lyra to finish startup",
};

const initialSetupProgress: SetupProgress = {
  kind: null,
  variant: null,
  state: "idle",
  downloadedBytes: 0,
  totalBytes: 0,
  error: null,
};

const initialUpdateStatus: UpdateStatus = {
  state: "idle",
  currentVersion: packageJson.version,
  availableVersion: null,
  notes: null,
  publishedAt: null,
  progress: null,
  lastCheckedAt: null,
  error: null,
};

const sectionLabels: Record<SettingsSection, string> = {
  general: "General",
  dictation: "Dictation",
  audio: "Audio",
  language: "Language",
  transcription: "Transcription",
  cloud: "Cloud",
  appearance: "Appearance",
  usage: "Usage",
  vocabulary: "Vocabulary",
  diagnostics: "Diagnostics",
};

function useLyraState() {
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [runtime, setRuntime] = useState<RuntimeStatus>(initialRuntime);
  const [modelRuntime, setModelRuntime] = useState<ModelRuntimeStatus>(initialModelRuntime);
  const [placementStatus, setPlacementStatus] = useState<BarPlacementStatus>(initialPlacement);
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [usage, setUsage] = useState<UsageStats>(initialUsage);
  const [session, setSession] = useState<SessionSnapshot>(initialSession);
  const [lastResult, setLastResult] = useState<TranscriptionResult | null>(null);
  const [devices, setDevices] = useState<AudioDeviceInfo[]>([]);
  const [credentialConfigured, setCredentialConfigured] = useState(false);
  const [artifacts, setArtifacts] = useState<SetupArtifact[]>([]);
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus>(initialHotkey);
  const [setupProgress, setSetupProgress] = useState<SetupProgress>(initialSetupProgress);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>(initialUpdateStatus);

  const refresh = useCallback(async () => {
    const [loadedSettings, loadedRuntime, loadedModelRuntime, loadedPlacement, loadedUsage, loadedDisplays, loadedHotkey, loadedArtifacts, loadedSession, loadedDevices, loadedCredential, loadedSetupProgress, loadedUpdateStatus] = await Promise.all([
      getSettings(),
      getRuntimeStatus(),
      getModelRuntimeStatus(),
      getBarPlacementStatus(),
      getUsageStats(),
      listDisplays(),
      getHotkeyStatus(),
      getSetupArtifacts(),
      getSessionSnapshot(),
      listAudioDevices(),
      hasOpenAiCredential(),
      getSetupProgress(),
      getUpdateStatus(),
    ]);
    if (loadedSettings) setSettings(loadedSettings);
    if (loadedRuntime) setRuntime(loadedRuntime);
    if (loadedModelRuntime) setModelRuntime(loadedModelRuntime);
    if (loadedPlacement) setPlacementStatus(loadedPlacement);
    if (loadedUsage) setUsage(loadedUsage);
    setDisplays(loadedDisplays);
    if (loadedHotkey) setHotkeyStatus(loadedHotkey);
    setArtifacts(loadedArtifacts);
    if (loadedSession) setSession(loadedSession);
    setDevices(loadedDevices);
    setCredentialConfigured(loadedCredential);
    if (loadedSetupProgress) setSetupProgress(loadedSetupProgress);
    if (loadedUpdateStatus) setUpdateStatus(loadedUpdateStatus);
  }, []);

  useEffect(() => {
    void refresh();
    let unlisten: (() => void)[] = [];
    void listenToLyraEvents({
      onState: setSession,
      onInputLevel: (inputLevel) => setSession((current) => ({ ...current, inputLevel })),
      onRuntimeStatus: setRuntime,
      onModelRuntimeStatus: setModelRuntime,
      onSettings: setSettings,
      onPlacementStatus: setPlacementStatus,
      onUsage: setUsage,
      onHotkeyStatus: setHotkeyStatus,
      onSetupProgress: setSetupProgress,
      onUpdateStatus: setUpdateStatus,
      onResult: (result) => {
        setLastResult(result);
        setSession((current) => ({
          ...current,
          state: "success",
          transcript: result.text,
          backend: result.backend,
          fallbackReason: result.fallbackReason,
        }));
      },
    }).then((cleanup) => {
      unlisten = cleanup;
    });
    return () => unlisten.forEach((stop) => stop());
  }, [refresh]);

  const updateSettings = useCallback(async (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    try {
      const saved = await saveSettings(next);
      if (saved) setSettings(saved);
    } catch (error) {
      setSettings(settings);
      throw error;
    }
  }, [settings]);

  const retryHotkeyRegistration = useCallback(async () => {
    const status = await retryHotkey();
    if (status) setHotkeyStatus(status);
    return status;
  }, []);

  const startSetup = useCallback(async (kind: "runtime" | "model", variant: string | null) => {
    await startSetupArtifact(kind, variant);
  }, []);

  const cancelSetup = useCallback(async () => {
    await cancelSetupArtifact();
  }, []);

  const checkUpdates = useCallback(async () => {
    const status = await checkForUpdates();
    if (status) setUpdateStatus(status);
    return status;
  }, []);

  const installAvailableUpdate = useCallback(async () => {
    const status = await installUpdate();
    if (status) setUpdateStatus(status);
    return status;
  }, []);

  const deferAvailableUpdate = useCallback(async () => {
    const status = await deferUpdate();
    if (status) setUpdateStatus(status);
    return status;
  }, []);

  return { settings, runtime, modelRuntime, placementStatus, displays, usage, session, lastResult, devices, credentialConfigured, artifacts, hotkeyStatus, setupProgress, updateStatus, refresh, updateSettings, retryHotkey: retryHotkeyRegistration, setCredentialConfigured, startSetup, cancelSetup, checkUpdates, installUpdate: installAvailableUpdate, deferUpdate: deferAvailableUpdate, resetUsage: async () => { const stats = await resetUsageStats(); if (stats) setUsage(stats); return stats; } };
}

function useWindowLabel() {
  const [label, setLabel] = useState(() => new URLSearchParams(window.location.search).get("window") ?? "settings");
  useEffect(() => {
    if (!isTauri) return;
    void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      setLabel(getCurrentWindow().label);
    });
  }, []);
  return label;
}

function formatDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.floor(toFiniteNumber(durationMs) / 1000));
  return `${String(Math.floor(totalSeconds / 60)).padStart(2, "0")} : ${String(totalSeconds % 60).padStart(2, "0")}`;
}

function toFiniteNumber(value: number | null | undefined, fallback = 0) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : fallback;
}

function formatSeconds(seconds: number) {
  return `${Math.max(0, Math.floor(toFiniteNumber(seconds)))}s`;
}

function formatBytes(bytes: number | null) {
  if (bytes === null) return "—";
  return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
}

function formatUpdateProgress(progress: number | null) {
  if (progress === null || !Number.isFinite(progress)) return "—";
  return `${Math.round(Math.max(0, Math.min(1, progress)) * 100)}%`;
}

function StatusDot({ tone = "muted" }: { tone?: "good" | "warn" | "muted" }) {
  return <span className={`status-dot status-dot--${tone}`} aria-hidden="true" />;
}

function DictationBar({ session, settings, runtime }: { session: SessionSnapshot; settings: Settings; runtime: RuntimeStatus }) {
  const [menuOpen, setMenuOpen] = useState(false);
  const sessionDurationMs = Math.max(0, toFiniteNumber(session.durationMs));
  const maxDurationSeconds = Math.max(1, Math.floor(toFiniteNumber(settings.maxDurationSeconds, 120)));
  const inputLevel = Math.max(0, toFiniteNumber(session.inputLevel));
  const [displayDurationMs, setDisplayDurationMs] = useState(sessionDurationMs);
  const barRef = useRef<HTMLElement>(null);
  const displayStartedAtRef = useRef<{ requestId: string | null; startedAt: number } | null>(null);
  const pttActiveRef = useRef(false);
  const pttReleaseInFlightRef = useRef(false);
  const pttStartPromiseRef = useRef<Promise<void> | null>(null);
  const isListening = session.state === "listening";
  const isProcessing = session.state === "processing";
  const isError = session.state === "error";
  const usesPushToTalk = primaryBarGesture(settings.activationMode) === "pointer_press_release";

  useEffect(() => {
    if (!isListening) {
      setDisplayDurationMs(sessionDurationMs);
      displayStartedAtRef.current = null;
      return;
    }

    if (displayStartedAtRef.current?.requestId !== session.requestId) {
      displayStartedAtRef.current = {
        requestId: session.requestId,
        startedAt: performance.now() - sessionDurationMs,
      };
    }
    const maxDurationMs = maxDurationSeconds * 1_000;
    const startedAt = displayStartedAtRef.current.startedAt;
    const updateDuration = () => {
      const elapsed = Math.max(0, performance.now() - startedAt);
      setDisplayDurationMs(Math.min(elapsed, maxDurationMs));
    };
    updateDuration();
    const interval = window.setInterval(updateDuration, 250);
    return () => window.clearInterval(interval);
  }, [isListening, maxDurationSeconds, sessionDurationMs, session.requestId]);

  useEffect(() => {
    if (!menuOpen) return;
    const onKeyDown = (event: KeyboardEvent) => { if (event.key === "Escape") setMenuOpen(false); };
    const onPointerDown = (event: PointerEvent) => {
      if (barRef.current && !barRef.current.contains(event.target as Node)) setMenuOpen(false);
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("pointerdown", onPointerDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("pointerdown", onPointerDown);
    };
  }, [menuOpen]);

  const onPrimaryAction = async () => {
    if (isListening) {
      await stopDictation("bar");
      return;
    }
    if (isProcessing) return;
    await startDictation("bar");
  };

  const startPushToTalk = () => {
    if (isProcessing || pttActiveRef.current) return;
    pttActiveRef.current = true;
    const start = startDictation("bar");
    pttStartPromiseRef.current = start;
    void start.catch(() => undefined).finally(() => {
      pttStartPromiseRef.current = null;
    });
  };

  const releasePushToTalk = async () => {
    if (!pttActiveRef.current || pttReleaseInFlightRef.current) return;
    pttActiveRef.current = false;
    pttReleaseInFlightRef.current = true;
    try {
      await pttStartPromiseRef.current;
      await stopDictation("bar");
    } finally {
      pttReleaseInFlightRef.current = false;
    }
  };

  const waveform = Array.from({ length: 18 }, (_, index) => {
    const base = isListening ? 4 + ((index * 7) % 10) : 3;
    const level = isListening ? Math.max(3, Math.round(base * (0.35 + inputLevel))) : base;
    return <span key={index} style={{ height: `${level}px` }} />;
  });

  return (
    <main ref={barRef} className={`bar-shell bar-shell--${session.state}`} onContextMenu={(event) => { event.preventDefault(); setMenuOpen(true); }}>
      <button
        className="bar-surface"
        aria-label={isListening ? "Stop dictation" : "Start dictation"}
        aria-pressed={isListening}
        onClick={!usesPushToTalk ? () => void onPrimaryAction() : undefined}
        onPointerDown={usesPushToTalk ? (event) => {
          if (!isPrimaryPointerButton(event.button)) return;
          event.currentTarget.setPointerCapture(event.pointerId);
          startPushToTalk();
        } : undefined}
        onPointerUp={settings.activationMode === "push_to_talk" ? () => { void releasePushToTalk(); } : undefined}
        onPointerCancel={settings.activationMode === "push_to_talk" ? () => { void releasePushToTalk(); } : undefined}
        onLostPointerCapture={settings.activationMode === "push_to_talk" ? () => { void releasePushToTalk(); } : undefined}
      >
        <span className="bar-leading" aria-hidden="true">
          {isListening ? <span className="recording-dot" /> : isProcessing ? <span className="mini-spinner" /> : isError ? <X size={11} /> : <Mic size={11} />}
        </span>
        {isListening ? (
          <>
            <span className="bar-waveform">{waveform}</span>
            <span className="bar-timer">{formatSeconds(displayDurationMs / 1_000)}:{formatSeconds(maxDurationSeconds)}</span>
          </>
        ) : isProcessing ? (
          <span className="bar-label">Transcribing…</span>
        ) : isError ? (
          <span className="bar-label">Try again</span>
        ) : (
          <span className="bar-idle-mark" aria-hidden="true" />
        )}
      </button>
      {menuOpen && (
        <div className="bar-menu" role="menu" onMouseLeave={() => setMenuOpen(false)}>
          <button onClick={() => { setMenuOpen(false); void showSettings(); }}><Settings2 size={13} /> Settings</button>
          <button onClick={() => { setMenuOpen(false); void hideBar(); }}><X size={13} /> Hide bar</button>
          <button className="bar-menu-danger" onClick={() => void quitLyra()}><Power size={13} /> Quit Lyra</button>
        </div>
      )}
      {isError && <button className="bar-recovery" onClick={() => void copyLastTranscript()} title="Copy the last transcript">{session.error ?? "Clipboard recovery available"}</button>}
      {!runtime.runtimeInstalled && settings.route !== "cloud_only" && <span className="bar-runtime-hint">Local runtime not installed</span>}
    </main>
  );
}

function SetupChecklist({ runtime, settings, devices, artifacts, hotkeyStatus, retryHotkey, setupProgress, startSetup, cancelSetup }: { runtime: RuntimeStatus; settings: Settings; devices: AudioDeviceInfo[]; artifacts: SetupArtifact[]; hotkeyStatus: HotkeyStatus; retryHotkey: () => Promise<HotkeyStatus | null>; setupProgress: SetupProgress; startSetup: (kind: "runtime" | "model", variant: string | null) => Promise<void>; cancelSetup: () => Promise<void> }) {
  const [setupError, setSetupError] = useState<string | null>(null);
  const checks = [
    { label: "Microphone available", done: devices.length > 0 || !isTauri, icon: Mic },
    { label: "GPU detected", done: Boolean(runtime.gpuName && runtime.gpuName !== "Detecting hardware…"), icon: Cpu },
    { label: "Native runtime verified", done: runtime.runtimeInstalled && runtime.runtimeVerified, icon: ShieldCheck },
    { label: "Parakeet model ready", done: runtime.modelInstalled && runtime.modelVerified, icon: Sparkles },
    { label: `Hotkey · ${hotkeyStatus.hotkey || settings.hotkey}`, done: !isTauri || hotkeyStatus.registered, icon: Keyboard },
    { label: "Recording path ready", done: true, icon: Radio },
  ];
  const ready = checks.every((check) => check.done);
  return (
    <div className="checklist">
      <div className="checklist-heading">
        <div>
          <span className="eyebrow">First-run readiness</span>
          <h2>{ready ? "Lyra is ready" : "A quiet start"}</h2>
        </div>
        <span className={`readiness-badge ${ready ? "readiness-badge--good" : ""}`}><StatusDot tone={ready ? "good" : "warn"} />{ready ? "Ready" : "Setup needed"}</span>
      </div>
      <div className="checklist-items">
        {checks.map(({ label, done, icon: Icon }) => (
          <div className="checklist-item" key={label}>
            <span className={`check-icon ${done ? "check-icon--done" : ""}`}>{done ? <Check size={13} /> : <Icon size={13} />}</span>
            <span>{label}</span>
            <span className={`check-state ${done ? "" : "check-state--pending"}`}>{done ? "Done" : "Pending"}</span>
          </div>
        ))}
      </div>
      <p className="muted-copy">Lyra keeps audio in memory and does not keep a transcript history. Runtime and model downloads are always explicit.</p>
      {isTauri && !hotkeyStatus.registered && <div className="info-callout"><CircleHelp size={16} /><span>The global shortcut is unavailable: {hotkeyStatus.error ?? "another application may already be using it"}. The tray and dictation bar remain available.</span><button className="secondary-button" onClick={() => void retryHotkey()}>Retry</button></div>}
      {artifacts.length > 0 && <div className="artifact-setup-list">
        {artifacts.map((artifact) => {
          const installed = artifact.kind === "runtime"
            ? runtime.runtimeInstalled && runtime.runtimeVerified && runtime.runtimeVariant === artifact.variant
            : runtime.modelInstalled && runtime.modelVerified;
          const isCurrentArtifact = setupProgress.kind === artifact.kind && setupProgress.variant === artifact.variant;
          const active = isCurrentArtifact && ["downloading", "verifying", "installing"].includes(setupProgress.state);
          const failed = isCurrentArtifact && setupProgress.state === "error";
          const percent = isCurrentArtifact && setupProgress.totalBytes > 0
            ? Math.round((setupProgress.downloadedBytes / setupProgress.totalBytes) * 100)
            : 0;
          const variantLabel = artifact.variant === "cuda" ? "CUDA" : artifact.variant === "vulkan" ? "Vulkan" : artifact.variant === "cpu" ? "CPU" : null;
          const title = artifact.kind === "runtime" ? `Native runtime · ${variantLabel ?? "fallback"}` : "Parakeet model";
          const actionLabel = artifact.kind === "runtime" ? `Install ${variantLabel ?? "runtime"}` : "Install model";
          return <div className="artifact-setup" key={`${artifact.kind}-${artifact.variant ?? "default"}`}>
            <div className="artifact-setup-copy">
              <div><strong>{title}</strong><span>{formatBytes(artifact.sizeBytes)} · {artifact.license ?? "licensed"}</span></div>
              <span>Source: {artifact.source}</span>
              <span>Stored at: {artifact.destination}</span>
              <code>SHA-256 {artifact.expectedSha256}</code>
            </div>
            <div className="artifact-setup-action">
              {active ? <>
                <span className="setup-progress-label">{setupProgress.state === "downloading" ? `${percent}%` : setupProgress.state}</span>
                <button className="secondary-button" onClick={() => void cancelSetup()}>Cancel</button>
              </> : installed ? <span className="status-pill"><StatusDot tone="good" />Verified</span> : <button className="secondary-button" onClick={() => { setSetupError(null); void startSetup(artifact.kind, artifact.variant).catch((error) => setSetupError(error instanceof Error ? error.message : "Could not start setup")); }}>{failed ? "Try again" : actionLabel}</button>}
            </div>
            {active && <div className="setup-progress-track"><span style={{ width: `${Math.max(0, Math.min(100, percent))}%` }} /></div>}
            {failed && <span className="artifact-setup-error">{setupProgress.error ?? "Setup failed; try again."}</span>}
          </div>;
        })}
        {setupError && <span className="artifact-setup-error">{setupError}</span>}
      </div>}
    </div>
  );
}

function SettingRow({ icon: Icon, label, description, children }: { icon: ComponentType<{ size?: number }>; label: string; description: string; children: ReactNode }) {
  return (
    <div className="setting-row">
      <span className="setting-icon"><Icon size={16} /></span>
      <div className="setting-copy"><strong>{label}</strong><span>{description}</span></div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function UpdateSection({ updateStatus, checkUpdates, installUpdate, deferUpdate }: { updateStatus: UpdateStatus; checkUpdates: () => Promise<UpdateStatus | null>; installUpdate: () => Promise<UpdateStatus | null>; deferUpdate: () => Promise<UpdateStatus | null> }) {
  const [actionError, setActionError] = useState<string | null>(null);
  const busy = ["checking", "downloading", "installing"].includes(updateStatus.state);
  const stateLabel: Record<UpdateStatus["state"], string> = {
    idle: "Ready to check",
    checking: "Checking",
    up_to_date: "Up to date",
    available: "Update available",
    downloading: "Downloading",
    installing: "Installing",
    error: "Unable to check",
  };
  const run = async (action: () => Promise<UpdateStatus | null>) => {
    setActionError(null);
    try {
      await action();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "The update action could not be completed");
    }
  };

  return <div className="update-card">
    <div className="update-card-icon"><Download size={16} /></div>
    <div className="update-card-copy">
      <div className="update-card-title"><span className="eyebrow">Software updates</span><span className={`update-state update-state--${updateStatus.state}`}><StatusDot tone={updateStatus.state === "available" ? "good" : updateStatus.state === "error" ? "warn" : "muted"} />{stateLabel[updateStatus.state]}</span></div>
      <strong>{updateStatus.availableVersion ? `Lyra ${updateStatus.availableVersion} is ready` : `Lyra ${updateStatus.currentVersion}`}</strong>
      <p>{updateStatus.notes ?? "Lyra checks the public signed release feed after startup and once per hour."}</p>
      {updateStatus.progress !== null && ["downloading", "installing"].includes(updateStatus.state) && <div className="update-progress"><span style={{ width: `${Math.round(Math.max(0, Math.min(1, updateStatus.progress)) * 100)}%` }} /></div>}
      {actionError && <span className="update-error">{actionError}</span>}
      {updateStatus.error && <span className="update-error">{updateStatus.error}</span>}
    </div>
    <div className="update-card-actions">
      {updateStatus.state === "available" && <>
        <button className="secondary-button update-install-button" onClick={() => void run(installUpdate)}><Download size={13} /> Install and restart</button>
        <button className="quiet-button" onClick={() => void run(deferUpdate)}>Defer</button>
      </>}
      <button className="secondary-button" disabled={busy} onClick={() => void run(checkUpdates)}><RefreshCw size={13} className={updateStatus.state === "checking" ? "spin-icon" : ""} />{updateStatus.state === "checking" ? "Checking…" : updateStatus.state === "error" ? "Try again" : "Check for updates"}</button>
    </div>
  </div>;
}

function GeneralSection({ settings, updateSettings, updateStatus, checkUpdates, installUpdate, deferUpdate, modelRuntime, displays, placementStatus }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void>; updateStatus: UpdateStatus; checkUpdates: () => Promise<UpdateStatus | null>; installUpdate: () => Promise<UpdateStatus | null>; deferUpdate: () => Promise<UpdateStatus | null>; modelRuntime: ModelRuntimeStatus; displays: DisplayInfo[]; placementStatus: BarPlacementStatus }) {
  const warmLabels: Record<ModelWarmPolicy, string> = {
    always_warm: "Always warm",
    unload_after_5_minutes: "Unload after 5 minutes",
    unload_after_15_minutes: "Unload after 15 minutes",
    unload_after_30_minutes: "Unload after 30 minutes",
    never_keep_warm: "Never keep warm",
  };
  return <>
    <SectionHeader eyebrow="Workspace" title="General" description="Keep Lyra close, quiet, and ready when you need it." />
    <div className="settings-card">
      <SettingRow icon={Power} label="Launch at login" description="Start Lyra in the system tray when Windows starts.">
        <button className={`toggle ${settings.launchAtLogin ? "toggle--on" : ""}`} aria-pressed={settings.launchAtLogin} onClick={() => void updateSettings({ launchAtLogin: !settings.launchAtLogin })}><span /></button>
      </SettingRow>
      <SettingRow icon={Monitor} label="Show dictation bar" description="Display the small non-activating bar while Lyra is running.">
        <button className={`toggle ${settings.showBar ? "toggle--on" : ""}`} aria-pressed={settings.showBar} onClick={() => void updateSettings({ showBar: !settings.showBar })}><span /></button>
      </SettingRow>
      <SettingRow icon={Sparkles} label="Model resource policy" description="Choose instant first-word latency or return VRAM to other workloads.">
        <select value={settings.modelWarmPolicy} onChange={(event) => void updateSettings({ modelWarmPolicy: event.target.value as ModelWarmPolicy })}>
          {Object.entries(warmLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}
        </select>
      </SettingRow>
      <SettingRow icon={Activity} label="Model runtime" description={modelRuntime.detail ?? "Recognizer lifecycle is independent from installed artifact status."}>
        <StatusPill good={modelRuntime.state === "ready"} label={modelRuntime.state === "loading" ? "Loading" : modelRuntime.state === "ready" ? "Ready" : modelRuntime.state === "error" ? "Error" : "Unloaded"} />
      </SettingRow>
      <SettingRow icon={Monitor} label="Bar placement" description="Follow the active window at startup and when a dictation begins.">
        <select value={settings.barPlacement} onChange={(event) => void updateSettings({ barPlacement: event.target.value as BarPlacement })}>
          <option value="follow_active_window">Follow active window</option>
          <option value="primary_display">Primary display</option>
          <option value="fixed_monitor">Fixed monitor</option>
        </select>
      </SettingRow>
      {settings.barPlacement === "fixed_monitor" && <SettingRow icon={Monitor} label="Fixed monitor" description="The saved monitor identity is preferred, with ordinal fallback when it changes.">
        <select value={settings.barMonitorId ?? displays.find((display) => display.index === settings.barMonitorIndex)?.id ?? ""} onChange={(event) => {
          const display = displays.find((candidate) => candidate.id === event.target.value);
          void updateSettings({ barMonitorId: display?.id ?? null, barMonitorIndex: display?.index ?? null });
        }}>
          <option value="">Select a monitor</option>
          {displays.map((display) => <option value={display.id} key={display.id}>Monitor {display.index}{display.primary ? " · primary" : ""}{display.name ? ` · ${display.name}` : ""}</option>)}
        </select>
      </SettingRow>}
      {placementStatus.fallback && <div className="info-callout"><CircleHelp size={16} /><span>{placementStatus.message ?? "Saved monitor unavailable; using the primary display."}</span></div>}
    </div>
    <UpdateSection updateStatus={updateStatus} checkUpdates={checkUpdates} installUpdate={installUpdate} deferUpdate={deferUpdate} />
  </>;
}

function DictationSection({ settings, updateSettings, hotkeyStatus }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void>; hotkeyStatus: HotkeyStatus }) {
  return <>
    <SectionHeader eyebrow="Activation" title="Dictation" description="Choose how a short gesture becomes text in the active app." />
    <div className="settings-card">
      <SettingRow icon={Keyboard} label="Activation mode" description="Push-to-talk records while the shortcut is held; toggle records between presses.">
        <Segmented value={settings.activationMode} options={[{ value: "push_to_talk", label: "Push-to-talk" }, { value: "toggle", label: "Toggle" }]} onChange={(value) => void updateSettings({ activationMode: value as ActivationMode })} />
      </SettingRow>
      <SettingRow icon={Keyboard} label="Global shortcut" description="Record a modifier plus one main key; Lyra validates and registers it immediately.">
        <RecordShortcut value={settings.hotkey} onChange={(hotkey) => void updateSettings({ hotkey })} />
      </SettingRow>
      <SettingRow icon={Timer} label="Maximum duration" description="Long recordings stop safely and are processed in memory.">
        <select value={settings.maxDurationSeconds} onChange={(event) => void updateSettings({ maxDurationSeconds: Number(event.target.value) })}>
          {[30, 60, 120, 180, 300, 600].map((seconds) => <option key={seconds} value={seconds}>{seconds} sec</option>)}
        </select>
      </SettingRow>
      <SettingRow icon={Timer} label="Auto-stop after silence" description="Toggle sessions stop after continuous silence, once speech has been detected. PTT never auto-stops.">
        <select value={settings.autoStopAfterSilenceMs ?? "off"} onChange={(event) => void updateSettings({ autoStopAfterSilenceMs: event.target.value === "off" ? null : Number(event.target.value) })}>
          <option value="off">Off</option>
          {[1000, 1500, 2000, 3000].map((milliseconds) => <option key={milliseconds} value={milliseconds}>{(milliseconds / 1000).toFixed(1)} sec</option>)}
        </select>
      </SettingRow>
      <SettingRow icon={Keyboard} label="Spoken formatting commands" description="Recognize new line, new paragraph, comma, period, and quote commands deterministically.">
        <button className={`toggle ${settings.formattingCommandsEnabled ? "toggle--on" : ""}`} aria-pressed={settings.formattingCommandsEnabled} onClick={() => void updateSettings({ formattingCommandsEnabled: !settings.formattingCommandsEnabled })}><span /></button>
      </SettingRow>
      <SettingRow icon={Clipboard} label="Paste automatically" description="Restore the prior text clipboard only when no other app changed it.">
        <button className={`toggle ${settings.autoPaste ? "toggle--on" : ""}`} aria-pressed={settings.autoPaste} onClick={() => void updateSettings({ autoPaste: !settings.autoPaste })}><span /></button>
      </SettingRow>
    </div>
    {hotkeyStatus.error && <div className="info-callout"><CircleHelp size={16} /><span>Shortcut status: {hotkeyStatus.error}</span></div>}
    <div className="info-callout"><ShieldCheck size={16} /><span>Lyra never presses Enter. If focus or injection fails, the transcript remains available to paste manually.</span></div>
  </>;
}

function RecordShortcut({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [captured, setCaptured] = useState<string | null>(null);
  const record = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    event.preventDefault();
    if (event.key === "Escape") {
      setRecording(false);
      setCaptured(null);
      return;
    }
    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push("Ctrl");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");
    if (event.metaKey) modifiers.push("Super");
    if (!modifiers.length || ["Control", "Alt", "Shift", "Meta"].includes(event.key)) {
      setCaptured(modifiers.length ? "Press a main key with the modifier" : "Add Ctrl, Alt, Shift, or Super");
      return;
    }
    const mainKey = event.code === "Space" ? "Space" : event.code || event.key;
    const next = [...modifiers, mainKey].join("+");
    onChange(next);
    setCaptured(next);
    setRecording(false);
  };
  return <div className="shortcut-recorder"><button className={`keycap shortcut-record-button ${recording ? "shortcut-record-button--active" : ""}`} onClick={() => { setRecording(true); setCaptured(null); }} onKeyDown={recording ? record : undefined}>{recording ? "Press shortcut…" : value}</button>{recording && <span className="shortcut-hint">Esc to cancel</span>}{captured && !recording && <span className="shortcut-hint">{captured}</span>}</div>;
}

function AudioSection({ settings, devices, updateSettings }: { settings: Settings; devices: AudioDeviceInfo[]; updateSettings: (patch: Partial<Settings>) => Promise<void> }) {
  return <>
    <SectionHeader eyebrow="Input" title="Audio" description="Use the Windows microphone selected for the current dictation session." />
    <div className="settings-card">
      <SettingRow icon={Mic} label="Input device" description="Device enumeration is provided by the native WASAPI capture layer.">
        <select value={settings.audioDeviceId ?? ""} onChange={(event) => void updateSettings({ audioDeviceId: event.target.value || null })}>
          <option value="">Default Windows microphone</option>
          {devices.map((device) => <option value={device.id} key={device.id}>{device.name}{device.isDefault ? " · default" : ""}</option>)}
        </select>
      </SettingRow>
      <SettingRow icon={Volume2} label="Input level" description="A small live signal appears only while recording.">
        <div className="level-meter"><span style={{ width: "38%" }} /></div>
      </SettingRow>
    </div>
  </>;
}

function VocabularySection({ settings, updateSettings }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void> }) {
  const [source, setSource] = useState("");
  const [replacement, setReplacement] = useState("");
  const add = () => {
    if (!source.trim() || !replacement.trim()) return;
    const entry: CorrectionEntry = { source: source.trim(), replacement: replacement.trim(), enabled: true, caseSensitive: false };
    void updateSettings({ corrections: [...settings.corrections, entry] });
    setSource("");
    setReplacement("");
  };
  const updateEntry = (index: number, patch: Partial<CorrectionEntry>) => {
    void updateSettings({ corrections: settings.corrections.map((entry, entryIndex) => entryIndex === index ? { ...entry, ...patch } : entry) });
  };
  const removeEntry = (index: number) => {
    void updateSettings({ corrections: settings.corrections.filter((_entry, entryIndex) => entryIndex !== index) });
  };
  return <>
    <SectionHeader eyebrow="Personalization" title="Vocabulary" description="Apply small, deterministic replacements after every local, cloud, or fallback transcription." />
    <div className="settings-card vocabulary-list">
      {settings.corrections.map((entry, index) => <div className="vocabulary-row" key={`${entry.source}-${index}`}>
        <input aria-label={`Correction source ${index + 1}`} value={entry.source} onChange={(event) => updateEntry(index, { source: event.target.value })} />
        <span className="vocabulary-arrow">→</span>
        <input aria-label={`Correction replacement ${index + 1}`} value={entry.replacement} onChange={(event) => updateEntry(index, { replacement: event.target.value })} />
        <label className="compact-check"><input type="checkbox" checked={entry.caseSensitive} onChange={(event) => updateEntry(index, { caseSensitive: event.target.checked })} />Case</label>
        <button className={`toggle compact-toggle ${entry.enabled ? "toggle--on" : ""}`} aria-label={`${entry.enabled ? "Disable" : "Enable"} ${entry.source}`} aria-pressed={entry.enabled} onClick={() => updateEntry(index, { enabled: !entry.enabled })}><span /></button>
        <button className="icon-button" title={`Remove ${entry.source}`} onClick={() => removeEntry(index)}><Trash2 size={14} /></button>
      </div>)}
      <div className="vocabulary-add"><input aria-label="New correction source" value={source} onChange={(event) => setSource(event.target.value)} placeholder="spoken phrase" /><span className="vocabulary-arrow">→</span><input aria-label="New correction replacement" value={replacement} onChange={(event) => setReplacement(event.target.value)} placeholder="replacement" /><button className="secondary-button" disabled={!source.trim() || !replacement.trim()} onClick={add}><Plus size={13} /> Add</button></div>
    </div>
    <div className="info-callout"><ShieldCheck size={16} /><span>Matches use whole words or phrases, longer sources first, and are case-insensitive by default. Lyra does not automatically capitalize other words.</span></div>
  </>;
}

function UsageSection({ usage, settings, updateSettings, resetUsage }: { usage: UsageStats; settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void>; resetUsage: () => Promise<UsageStats | null> }) {
  const reset = () => {
    if (window.confirm("Reset Lyra's local usage statistics? This cannot be undone.")) void resetUsage();
  };
  const usd = (value: number | null) => value === null ? "Hidden" : `$${value.toFixed(3)}`;
  return <>
    <SectionHeader eyebrow="Privacy-first telemetry" title="Usage" description="Aggregate local statistics only. Lyra never stores transcript text or raw audio history." />
    <div className="usage-grid">
      <Diagnostic label="Today" value={String(usage.dictationsToday)} icon={BarChart3} />
      <Diagnostic label="All time" value={String(usage.dictationsAllTime)} icon={Activity} />
      <Diagnostic label="Minutes dictated" value={usage.minutesDictated.toFixed(1)} icon={Timer} />
      <Diagnostic label="Avg latency" value={`${Math.round(usage.averageTranscriptionLatencyMs)} ms`} icon={Gauge} />
      <Diagnostic label="Real-time factor" value={usage.averageRealTimeFactor.toFixed(2)} icon={Activity} />
      <Diagnostic label="Local / cloud" value={`${usage.localPercent.toFixed(0)}% / ${usage.cloudPercent.toFixed(0)}%`} icon={Cpu} />
      <Diagnostic label="Failed sessions" value={String(usage.failedTranscriptions)} icon={X} />
      <Diagnostic label="Paste success" value={`${usage.pasteSuccessRate.toFixed(0)}%`} icon={Clipboard} />
      <Diagnostic label="Cloud fallbacks" value={String(usage.cloudFallbacks)} icon={Cloud} />
      <Diagnostic label="Cloud estimate" value={usd(usage.estimatedCloudCostUsd)} icon={Cloud} />
      <Diagnostic label="Local cost avoided" value={usd(usage.estimatedLocalCostAvoidedUsd)} icon={ShieldCheck} />
    </div>
    <div className="settings-card">
      <SettingRow icon={CircleHelp} label="Show cost estimates" description="Opt in to local estimates based on the editable cloud rate; no provider billing data is fetched.">
        <button className={`toggle ${settings.usageCostEstimatesEnabled ? "toggle--on" : ""}`} aria-pressed={settings.usageCostEstimatesEnabled} onClick={() => void updateSettings({ usageCostEstimatesEnabled: !settings.usageCostEstimatesEnabled })}><span /></button>
      </SettingRow>
      <SettingRow icon={Cloud} label="Estimated cloud rate" description="Used only for the estimates above and stored locally with settings.">
        <label className="number-control"><span>$</span><input type="number" min="0" max="1000" step="0.001" value={settings.cloudCostPerMinuteUsd} onChange={(event) => void updateSettings({ cloudCostPerMinuteUsd: Math.max(0, Number(event.target.value)) })} /><span>/ min</span></label>
      </SettingRow>
    </div>
    <div className="usage-footer"><span>Paste success counts only automatic paste attempts. A failed recognition or injection is an end-to-end failure.</span><button className="quiet-danger" onClick={reset}><Trash2 size={13} /> Reset statistics</button></div>
  </>;
}

function LanguageSection({ settings, updateSettings }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void> }) {
  return <>
    <SectionHeader eyebrow="Recognition" title="Language" description="Set a preference for compatible backends; local Parakeet remains auto-detecting." />
    <div className="settings-card">
      <SettingRow icon={Globe2} label="Preferred language" description="English and Greek are available as backend preferences.">
        <Segmented value={settings.language} options={[{ value: "auto", label: "Auto" }, { value: "en", label: "English" }, { value: "el", label: "Ελληνικά" }]} onChange={(value) => void updateSettings({ language: value as LanguagePreference })} />
      </SettingRow>
    </div>
    <div className="info-callout"><CircleHelp size={16} /><span>Parakeet v3 can recognize both English and Greek, but the selected native runtime does not expose forced-language or detected-language metadata.</span></div>
  </>;
}

function TranscriptionSection({ settings, updateSettings, runtime, lastResult }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void>; runtime: RuntimeStatus; lastResult: TranscriptionResult | null }) {
  return <>
    <SectionHeader eyebrow="Routing" title="Transcription" description="Prefer local recognition and keep cloud use an explicit fallback." />
    <div className="settings-card">
      <SettingRow icon={RouteIcon} label="Transcription route" description="Cloud fallback is used only after a local backend failure.">
        <select value={settings.route} onChange={(event) => void updateSettings({ route: event.target.value as TranscriptionRoute })}>
          <option value="local_first_cloud_fallback">Local first · cloud fallback</option>
          <option value="local_only">Local only</option>
          <option value="cloud_only">Cloud only</option>
        </select>
      </SettingRow>
      <SettingRow icon={Cpu} label="Local runtime" description={runtime.runtimeInstalled ? `${runtime.runtimeVariant?.toUpperCase()} · verified` : "Not installed yet; setup will ask before downloading."}>
        <StatusPill good={runtime.runtimeInstalled && runtime.runtimeVerified} label={runtime.runtimeInstalled ? "Verified" : "Pending"} />
      </SettingRow>
      <SettingRow icon={Sparkles} label="Model" description={runtime.modelName ?? "Parakeet TDT 0.6B v3 · Q8 GGUF"}>
        <StatusPill good={runtime.modelInstalled && runtime.modelVerified} label={runtime.modelInstalled ? "Ready" : "Pending"} />
      </SettingRow>
    </div>
    {lastResult && <div className="benchmark-card"><div><span className="eyebrow">Last recognition</span><strong>{lastResult.backend} · {lastResult.model}</strong></div><div className="benchmark-stats"><span><b>{lastResult.latencyMs} ms</b> latency</span><span><b>{lastResult.audioDurationMs} ms</b> audio</span><span><b>{lastResult.language ?? "auto"}</b> language</span></div></div>}
  </>;
}

function CloudSection({ credentialConfigured, setCredentialConfigured }: { credentialConfigured: boolean; setCredentialConfigured: (configured: boolean) => void }) {
  const [secret, setSecret] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  const save = async () => {
    try {
      await saveOpenAiCredential(secret);
      setSecret("");
      setCredentialConfigured(true);
      setMessage("Saved in Windows Credential Manager");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not save credential");
    }
  };

  const clear = async () => {
    try {
      await clearOpenAiCredential();
      setCredentialConfigured(false);
      setMessage("Credential removed");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not remove credential");
    }
  };

  return <>
    <SectionHeader eyebrow="Optional" title="Cloud" description="Keep credentials in Windows Credential Manager when cloud fallback is enabled." />
    <div className="settings-card">
      <SettingRow icon={Cloud} label="OpenAI credential" description="Stored through OS-backed secret storage; never serialized or logged.">
        <StatusPill good={credentialConfigured} label={credentialConfigured ? "Configured" : "Not configured"} />
      </SettingRow>
      <SettingRow icon={Globe2} label="Cloud model" description="Used only for an approved cloud route or local failure fallback.">
        <span className="control-placeholder">gpt-4o-mini-transcribe</span>
      </SettingRow>
    </div>
    <div className="credential-editor"><div><span className="eyebrow">Credential manager</span><p>{message ?? "Paste an API key only if you want cloud transcription or fallback."}</p></div><div className="credential-actions"><input type="password" value={secret} onChange={(event) => setSecret(event.target.value)} placeholder="sk-…" autoComplete="off" spellCheck={false} aria-label="OpenAI API key" /><button className="secondary-button" disabled={!secret.trim()} onClick={() => void save()}>Save key</button>{credentialConfigured && <button className="quiet-danger" onClick={() => void clear()}>Remove</button>}</div></div>
    <div className="info-callout"><Cloud size={16} /><span>Cloud-generated text is labeled in the result and diagnostics. Missing credentials behave as local-only.</span></div>
  </>;
}

function AppearanceSection({ settings, updateSettings }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void> }) {
  return <>
    <SectionHeader eyebrow="Surface" title="Appearance" description="A quiet charcoal surface with one cool signal accent." />
    <div className="settings-card">
      <SettingRow icon={Monitor} label="Theme" description="The first release is optimized for a dark system-tray presence.">
        <Segmented value={settings.theme} options={[{ value: "dark", label: "Dark" }, { value: "system", label: "System" }]} onChange={(value) => void updateSettings({ theme: value as Settings["theme"] })} />
      </SettingRow>
    </div>
  </>;
}

function DiagnosticsSection({ runtime, session, refresh }: { runtime: RuntimeStatus; session: SessionSnapshot; refresh: () => Promise<void> }) {
  return <>
    <SectionHeader eyebrow="Observability" title="Diagnostics" description="High-signal health information without raw audio, transcript history, or secrets." />
    <div className="diagnostics-grid">
      <Diagnostic label="Runtime" value={runtime.runtimeInstalled ? `${runtime.runtimeVersion ?? "unknown"} · ${runtime.runtimeVariant}` : "Not installed"} icon={Cpu} />
      <Diagnostic label="Model" value={runtime.modelInstalled ? `${runtime.modelName ?? "Parakeet"} · ${formatBytes(runtime.modelSizeBytes)}` : "Not installed"} icon={Sparkles} />
      <Diagnostic label="GPU" value={runtime.gpuName ?? "Unavailable"} icon={Gauge} />
      <Diagnostic label="Session" value={`${session.state} · ${formatDuration(session.durationMs)}`} icon={Activity} />
    </div>
    <div className="settings-card diagnostic-details"><div><span>Runtime path</span><code>{runtime.runtimePath}</code></div><div><span>Model path</span><code>{runtime.modelPath}</code></div><div><span>Last error</span><code>{runtime.lastError ?? "None"}</code></div></div>
    <button className="secondary-button" onClick={() => void refresh()}><RotateCcw size={14} /> Refresh diagnostics</button>
  </>;
}

function SectionHeader({ eyebrow, title, description }: { eyebrow: string; title: string; description: string }) {
  return <div className="section-header"><span className="eyebrow">{eyebrow}</span><h1>{title}</h1><p>{description}</p></div>;
}

function Segmented({ value, options, onChange }: { value: string; options: { value: string; label: string }[]; onChange: (value: string) => void }) {
  return <div className="segmented">{options.map((option) => <button key={option.value} className={value === option.value ? "segmented-active" : ""} onClick={() => onChange(option.value)}>{option.label}</button>)}</div>;
}

function StatusPill({ good = false, label }: { good?: boolean; label: string }) {
  return <span className="status-pill"><StatusDot tone={good ? "good" : "warn"} />{label}</span>;
}

function Diagnostic({ label, value, icon: Icon }: { label: string; value: string; icon: typeof Cpu }) {
  return <div className="diagnostic"><span className="diagnostic-icon"><Icon size={15} /></span><span className="eyebrow">{label}</span><strong title={value}>{value}</strong></div>;
}

function RouteIcon(_props: { size?: number }) {
  return <span className="route-icon" aria-hidden="true"><span /><span /><span /></span>;
}

function SettingsApp({ settings, runtime, modelRuntime, placementStatus, displays, usage, session, lastResult, devices, credentialConfigured, artifacts, hotkeyStatus, setupProgress, updateStatus, updateSettings, retryHotkey, setCredentialConfigured, refresh, startSetup, cancelSetup, checkUpdates, installUpdate, deferUpdate, resetUsage }: ReturnType<typeof useLyraState>) {
  const [section, setSection] = useState<SettingsSection>("general");
  const [copied, setCopied] = useState(false);
  const [onboardingDismissed, setOnboardingDismissed] = useState(settings.onboardingComplete);
  const [updatePromptVisible, setUpdatePromptVisible] = useState(false);
  const promptedVersion = useRef<string | null>(null);

  useEffect(() => {
    setOnboardingDismissed(settings.onboardingComplete);
  }, [settings.onboardingComplete]);

  useEffect(() => {
    const version = updateStatus.availableVersion;
    if (updateStatus.state !== "available" || !version || promptedVersion.current === version) return;
    promptedVersion.current = version;
    setUpdatePromptVisible(true);
    void showSettings();
  }, [updateStatus.availableVersion, updateStatus.state]);

  const copy = async () => {
    const text = await copyLastTranscript();
    if (text !== null) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    }
  };

  const content = useMemo(() => {
    switch (section) {
      case "general": return <><SetupChecklist runtime={runtime} settings={settings} devices={devices} artifacts={artifacts} hotkeyStatus={hotkeyStatus} retryHotkey={retryHotkey} setupProgress={setupProgress} startSetup={startSetup} cancelSetup={cancelSetup} /><GeneralSection settings={settings} updateSettings={updateSettings} updateStatus={updateStatus} checkUpdates={checkUpdates} installUpdate={installUpdate} deferUpdate={deferUpdate} modelRuntime={modelRuntime} displays={displays} placementStatus={placementStatus} /></>;
      case "dictation": return <DictationSection settings={settings} updateSettings={updateSettings} hotkeyStatus={hotkeyStatus} />;
      case "audio": return <AudioSection settings={settings} devices={devices} updateSettings={updateSettings} />;
      case "language": return <LanguageSection settings={settings} updateSettings={updateSettings} />;
      case "transcription": return <TranscriptionSection settings={settings} updateSettings={updateSettings} runtime={runtime} lastResult={lastResult} />;
      case "cloud": return <CloudSection credentialConfigured={credentialConfigured} setCredentialConfigured={setCredentialConfigured} />;
      case "appearance": return <AppearanceSection settings={settings} updateSettings={updateSettings} />;
      case "vocabulary": return <VocabularySection settings={settings} updateSettings={updateSettings} />;
      case "usage": return <UsageSection usage={usage} settings={settings} updateSettings={updateSettings} resetUsage={resetUsage} />;
      case "diagnostics": return <DiagnosticsSection runtime={runtime} session={session} refresh={refresh} />;
    }
  }, [artifacts, cancelSetup, checkUpdates, deferUpdate, devices, hotkeyStatus, installUpdate, lastResult, modelRuntime, placementStatus, displays, refresh, retryHotkey, runtime, section, session, settings, setupProgress, startSetup, updateSettings, updateStatus, usage]);

  return <main className="settings-shell">
    <aside className="sidebar">
      <div className="brand-lockup"><img className="brand-icon" src={lyraIcon} alt="" /><div><strong>Lyra</strong><span>Orion voice layer</span></div></div>
      <div className="sidebar-status"><StatusDot tone={runtime.runtimeInstalled && runtime.modelInstalled ? "good" : "warn"} /><span>{runtime.runtimeInstalled && runtime.modelInstalled ? "Ready locally" : "Setup in progress"}</span></div>
      <nav className="settings-nav" aria-label="Settings">
        {(Object.keys(sectionLabels) as SettingsSection[]).map((item) => <button key={item} className={section === item ? "nav-active" : ""} onClick={() => setSection(item)}><NavIcon section={item} /><span>{sectionLabels[item]}</span>{section === item && <ChevronRight size={14} />}</button>)}
      </nav>
      <div className="sidebar-footer"><button className="sidebar-footer-button" onClick={() => void copy()}><Clipboard size={14} /> {copied ? "Copied" : "Copy last transcription"}</button><span>v{updateStatus.currentVersion} · Windows x64</span></div>
    </aside>
    <section className="settings-content">
      <header className="topbar"><div className="window-title"><span className="window-title-dot" />Settings</div><div className="topbar-actions"><button className="icon-button" title="Open dictation bar" onClick={() => void showSettings()}><MoreHorizontal size={17} /></button><button className="window-close" title="Hide settings" onClick={() => void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().hide())}><X size={16} /></button></div></header>
      <div className="content-scroll">
        {!onboardingDismissed && <div className="onboarding-banner"><div className="onboarding-icon"><Sparkles size={17} /></div><div><strong>Welcome to Lyra</strong><span>Finish setup when you are ready. Downloads require your explicit approval.</span></div><button onClick={() => { setOnboardingDismissed(true); void updateSettings({ onboardingComplete: true }); }}>Got it <ArrowLeft size={14} className="rotate-180" /></button></div>}
        {updatePromptVisible && updateStatus.state === "available" && <div className="update-prompt"><div className="update-prompt-icon"><Download size={15} /></div><div><strong>Lyra {updateStatus.availableVersion} is ready</strong><span>{updateStatus.notes ?? "A signed update is available."}</span></div><button className="secondary-button" onClick={() => { setUpdatePromptVisible(false); void installUpdate().catch(() => undefined); }}><Download size={13} /> Install and restart</button><button className="quiet-button" onClick={() => { setUpdatePromptVisible(false); void deferUpdate().catch(() => undefined); }}>Defer</button></div>}
        {content}
        <footer className="settings-footer"><span><StatusDot tone="good" />Lyra is running in the tray</span><span>Audio stays in memory</span></footer>
      </div>
    </section>
  </main>;
}

function NavIcon({ section }: { section: SettingsSection }) {
  const icons: Record<SettingsSection, typeof Settings2> = { general: Settings2, dictation: Keyboard, audio: Mic, language: Globe2, transcription: Cpu, cloud: Cloud, appearance: SlidersHorizontal, vocabulary: Sparkles, usage: BarChart3, diagnostics: Activity };
  const Icon = icons[section];
  return <Icon size={15} />;
}

export default function App() {
  const state = useLyraState();
  const label = useWindowLabel();

  useEffect(() => {
    const isBar = label === "bar";
    document.documentElement.classList.toggle("bar-window", isBar);
    document.body.classList.toggle("bar-window", isBar);
    return () => {
      document.documentElement.classList.remove("bar-window");
      document.body.classList.remove("bar-window");
    };
  }, [label]);

  if (label === "bar") return <DictationBar session={state.session} settings={state.settings} runtime={state.runtime} />;
  return <SettingsApp {...state} />;
}
