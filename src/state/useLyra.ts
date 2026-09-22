import { useCallback, useEffect, useMemo, useState } from "react";
import * as api from "../bridge";
import {
  defaultPersonalization,
  defaultSettings,
  type AudioDeviceInfo,
  type HotkeyStatus,
  type ModelRuntimeStatus,
  type Personalization,
  type RuntimeStatus,
  type SessionSnapshot,
  type Settings,
  type SetupArtifact,
  type SetupProgress,
  type TranscriptionPartial,
  type UpdateStatus,
  type UsageStats,
} from "../types";
const runtime0: RuntimeStatus = {
  runtimeInstalled: true,
  runtimeVersion: "0.1.0",
  runtimeVariant: "cuda",
  runtimePath: "%LOCALAPPDATA%\\Lyra\\runtime",
  runtimeSha256: null,
  runtimeVerified: true,
  modelInstalled: true,
  modelName: "Parakeet TDT 0.6B v3",
  modelRevision: "main",
  modelPath: "%LOCALAPPDATA%\\Lyra\\models",
  modelSizeBytes: 650000000,
  modelVerified: true,
  nemotronModelInstalled: false,
  nemotronModelVerified: false,
  nemotronModelPath:
    "%LOCALAPPDATA%\\Lyra\\models\\nemotron-speech-streaming-en-0.6b.q8_0.gguf",
  gpuName: "NVIDIA GeForce RTX",
  gpuVramBytes: 8589934592,
  lastError: null,
};
const session0: SessionSnapshot = {
  state: "idle",
  durationMs: 0,
  inputLevel: 0,
  requestId: null,
  error: null,
  transcript: null,
  backend: null,
  fallbackReason: null,
};
const usage0: UsageStats = {
  dictationsToday: 8,
  dictationsAllTime: 76,
  minutesDictated: 28,
  completedResults: 71,
  averageTranscriptionLatencyMs: 420,
  averageRealTimeFactor: 0.18,
  localResultCount: 69,
  cloudResultCount: 2,
  localPercent: 97.2,
  cloudPercent: 2.8,
  failedTranscriptions: 5,
  pasteAttempts: 71,
  pasteSuccesses: 71,
  pasteSuccessRate: 100,
  cloudFallbacks: 2,
  costEstimatesEnabled: false,
  cloudCostPerMinuteUsd: 0.003,
  estimatedCloudCostUsd: null,
  estimatedLocalCostAvoidedUsd: null,
  totalWords: 412,
  estimatedTypingHoursSaved: 1.2,
  detailedSince: "2026-09-22",
  daily: [1, 2, 3, 7, 4, 2, 1].map((words, i) => ({
    day: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][i],
    started: words,
    completed: words,
    failures: 0,
    minutes: words * 2,
    words: words * 16,
    pasteAttempts: words,
    pasteSuccesses: words,
    cloudFallbacks: 0,
    latencyMs: 420,
    realTimeFactor: 0.18,
  })),
  recentSessions: [],
  failureCounts: {},
};
export function useLyra() {
  const [settings, setSettings] = useState(defaultSettings);
  const [personalization, setPersonalization] = useState(
    defaultPersonalization,
  );
  const [runtime, setRuntime] = useState(runtime0);
  const [modelRuntime, setModelRuntime] = useState<ModelRuntimeStatus>({
    state: "ready",
    detail: null,
  });
  const [session, setSession] = useState(session0);
  const [usage, setUsage] = useState(usage0);
  const [devices, setDevices] = useState<AudioDeviceInfo[]>([
    {
      id: "default",
      name: "Microphone (Samson G-Track Pro)",
      isDefault: true,
      sampleRate: 48000,
      channels: 2,
      format: "F32",
    },
  ]);
  const [artifacts, setArtifacts] = useState<SetupArtifact[]>([]);
  const [setup, setSetup] = useState<SetupProgress>({
    kind: null,
    variant: null,
    state: "idle",
    downloadedBytes: 0,
    totalBytes: 0,
    error: null,
  });
  const [hotkey, setHotkey] = useState<HotkeyStatus>({
    hotkey: defaultSettings.hotkey,
    registered: true,
    error: null,
  });
  const [partial, setPartial] = useState<TranscriptionPartial | null>(null);
  const [update, setUpdate] = useState<UpdateStatus>({
    state: "idle",
    currentVersion: "0.4.0",
    availableVersion: null,
    notes: null,
    publishedAt: null,
    progress: null,
    lastCheckedAt: null,
    error: null,
  });
  const refresh = useCallback(async () => {
    const [s, p, r, m, u, d, a, sp, h, sn, up] = await Promise.all([
      api.getSettings(),
      api.getPersonalization(),
      api.getRuntimeStatus(),
      api.getModelRuntimeStatus(),
      api.getUsageStats(),
      api.listAudioDevices(),
      api.getSetupArtifacts(),
      api.getSetupProgress(),
      api.getHotkeyStatus(),
      api.getSessionSnapshot(),
      api.getUpdateStatus(),
    ]);
    if (s) setSettings(s);
    if (p) setPersonalization(p);
    if (r) setRuntime(r);
    if (m) setModelRuntime(m);
    if (u) setUsage(u);
    if (d.length) setDevices(d);
    if (a.length) setArtifacts(a);
    if (sp) setSetup(sp);
    if (h) setHotkey(h);
    if (sn) setSession(sn);
    if (up) setUpdate(up);
  }, []);
  useEffect(() => {
    void refresh();
    let dead = false;
    let un: (() => void)[] = [];
    api
      .listenToLyraEvents({
        onState: (v) => {
          setSession(v);
          if (v.state === "idle") setPartial(null);
        },
        onInputLevel: (v) => setSession((s) => ({ ...s, inputLevel: v })),
        onRuntimeStatus: setRuntime,
        onModelRuntimeStatus: setModelRuntime,
        onSettings: setSettings,
        onPersonalization: setPersonalization,
        onPlacementStatus: () => {},
        onUsage: setUsage,
        onHotkeyStatus: setHotkey,
        onSetupProgress: setSetup,
        onUpdateStatus: setUpdate,
        onResult: () => setPartial(null),
        onPartial: (v) =>
          setPartial((current) =>
            !session.requestId || v.requestId === session.requestId
              ? v
              : current,
          ),
      })
      .then((v) => {
        if (dead) v.forEach((x) => x());
        else un = v;
      });
    return () => {
      dead = true;
      un.forEach((x) => x());
    };
  }, [refresh, session.requestId]);
  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.dataset.accent = settings.accent;
    document.documentElement.dataset.density = settings.compactMode
      ? "compact"
      : "comfortable";
  }, [settings]);
  const updateSettings = useCallback(
    async (patch: Partial<Settings>) => {
      const next = { ...settings, ...patch };
      setSettings(next);
      const saved = await api.saveSettings(next);
      if (saved) setSettings(saved);
    },
    [settings],
  );
  const updatePersonalization = useCallback(async (next: Personalization) => {
    setPersonalization(next);
    const saved = await api.savePersonalization(next);
    if (saved) setPersonalization(saved);
  }, []);
  return useMemo(
    () => ({
      settings,
      updateSettings,
      personalization,
      updatePersonalization,
      runtime,
      modelRuntime,
      session,
      usage,
      devices,
      artifacts,
      setup,
      hotkey,
      partial,
      update,
      refresh,
    }),
    [
      settings,
      updateSettings,
      personalization,
      updatePersonalization,
      runtime,
      modelRuntime,
      session,
      usage,
      devices,
      artifacts,
      setup,
      hotkey,
      partial,
      update,
      refresh,
    ],
  );
}
export type LyraState = ReturnType<typeof useLyra>;
