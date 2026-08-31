import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType, type ReactNode } from "react";
import {
  Activity,
  ArrowLeft,
  Check,
  ChevronRight,
  CircleHelp,
  Clipboard,
  Cloud,
  Cpu,
  Gauge,
  Globe2,
  Keyboard,
  Mic,
  Monitor,
  MoreHorizontal,
  Play,
  Power,
  Radio,
  RotateCcw,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Square,
  Timer,
  Volume2,
  X,
} from "lucide-react";
import {
  cancelDictation,
  copyLastTranscript,
  getRuntimeStatus,
  getSetupArtifacts,
  getSessionSnapshot,
  getSettings,
  hasOpenAiCredential,
  hideBar,
  listAudioDevices,
  isTauri,
  listenToLyraEvents,
  quitLyra,
  saveSettings,
  saveOpenAiCredential,
  clearOpenAiCredential,
  showSettings,
  startDictation,
  stopDictation,
} from "./bridge";
import type {
  ActivationMode,
  DictationState,
  LanguagePreference,
  AudioDeviceInfo,
  RuntimeStatus,
  SessionSnapshot,
  Settings,
  SetupArtifact,
  TranscriptionRoute,
  TranscriptionResult,
} from "./types";
import { defaultSettings } from "./types";

type SettingsSection = "general" | "dictation" | "audio" | "language" | "transcription" | "cloud" | "appearance" | "diagnostics";

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

const sectionLabels: Record<SettingsSection, string> = {
  general: "General",
  dictation: "Dictation",
  audio: "Audio",
  language: "Language",
  transcription: "Transcription",
  cloud: "Cloud",
  appearance: "Appearance",
  diagnostics: "Diagnostics",
};

function useLyraState() {
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [runtime, setRuntime] = useState<RuntimeStatus>(initialRuntime);
  const [session, setSession] = useState<SessionSnapshot>(initialSession);
  const [lastResult, setLastResult] = useState<TranscriptionResult | null>(null);
  const [devices, setDevices] = useState<AudioDeviceInfo[]>([]);
  const [credentialConfigured, setCredentialConfigured] = useState(false);
  const [artifacts, setArtifacts] = useState<SetupArtifact[]>([]);

  const refresh = useCallback(async () => {
    const [loadedSettings, loadedRuntime, loadedArtifacts, loadedSession, loadedDevices, loadedCredential] = await Promise.all([
      getSettings(),
      getRuntimeStatus(),
      getSetupArtifacts(),
      getSessionSnapshot(),
      listAudioDevices(),
      hasOpenAiCredential(),
    ]);
    if (loadedSettings) setSettings(loadedSettings);
    if (loadedRuntime) setRuntime(loadedRuntime);
    setArtifacts(loadedArtifacts);
    if (loadedSession) setSession(loadedSession);
    setDevices(loadedDevices);
    setCredentialConfigured(loadedCredential);
  }, []);

  useEffect(() => {
    void refresh();
    let unlisten: (() => void)[] = [];
    void listenToLyraEvents({
      onState: setSession,
      onInputLevel: (inputLevel) => setSession((current) => ({ ...current, inputLevel })),
      onRuntimeStatus: setRuntime,
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
    const saved = await saveSettings(next);
    if (saved) setSettings(saved);
  }, [settings]);

  return { settings, runtime, session, lastResult, devices, credentialConfigured, artifacts, refresh, updateSettings, setCredentialConfigured };
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
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  return `${String(Math.floor(totalSeconds / 60)).padStart(2, "0")} : ${String(totalSeconds % 60).padStart(2, "0")}`;
}

function formatBytes(bytes: number | null) {
  if (bytes === null) return "—";
  return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
}

function StatusDot({ tone = "muted" }: { tone?: "good" | "warn" | "muted" }) {
  return <span className={`status-dot status-dot--${tone}`} aria-hidden="true" />;
}

function DictationBar({ session, settings, runtime }: { session: SessionSnapshot; settings: Settings; runtime: RuntimeStatus }) {
  const [menuOpen, setMenuOpen] = useState(false);
  const barRef = useRef<HTMLElement>(null);
  const isListening = session.state === "listening";
  const isProcessing = session.state === "processing";
  const isError = session.state === "error";

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

  const waveform = Array.from({ length: 18 }, (_, index) => {
    const base = isListening ? 4 + ((index * 7) % 10) : 3;
    const level = isListening ? Math.max(3, Math.round(base * (0.35 + session.inputLevel))) : base;
    return <span key={index} style={{ height: `${level}px` }} />;
  });

  return (
    <main ref={barRef} className={`bar-shell bar-shell--${session.state}`} onContextMenu={(event) => { event.preventDefault(); setMenuOpen(true); }}>
      <button
        className="bar-surface"
        aria-label={isListening ? "Stop dictation" : "Start dictation"}
        onClick={() => void onPrimaryAction()}
      >
        <span className="bar-leading" aria-hidden="true">
          {isListening ? <span className="recording-dot" /> : isProcessing ? <span className="mini-spinner" /> : isError ? <X size={11} /> : <Mic size={11} />}
        </span>
        {isListening ? (
          <>
            <span className="bar-waveform">{waveform}</span>
            <span className="bar-timer">{formatDuration(session.durationMs)}</span>
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

function SetupChecklist({ runtime, settings, devices, artifacts }: { runtime: RuntimeStatus; settings: Settings; devices: AudioDeviceInfo[]; artifacts: SetupArtifact[] }) {
  const checks = [
    { label: "Microphone available", done: devices.length > 0 || !isTauri, icon: Mic },
    { label: "GPU detected", done: Boolean(runtime.gpuName && runtime.gpuName !== "Detecting hardware…"), icon: Cpu },
    { label: "Native runtime verified", done: runtime.runtimeInstalled && runtime.runtimeVerified, icon: ShieldCheck },
    { label: "Parakeet model ready", done: runtime.modelInstalled && runtime.modelVerified, icon: Sparkles },
    { label: `Hotkey · ${settings.hotkey}`, done: Boolean(settings.hotkey), icon: Keyboard },
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
      {artifacts.length > 0 && <div className="artifact-summary">{artifacts.map((artifact) => <span key={artifact.kind}><b>{artifact.kind === "runtime" ? "Runtime" : "Model"}</b> {formatBytes(artifact.sizeBytes)} · pinned {artifact.revision ?? "current"}</span>)}</div>}
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

function GeneralSection({ settings, updateSettings }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void> }) {
  return <>
    <SectionHeader eyebrow="Workspace" title="General" description="Keep Lyra close, quiet, and ready when you need it." />
    <div className="settings-card">
      <SettingRow icon={Power} label="Launch at login" description="Start Lyra in the system tray when Windows starts.">
        <button className={`toggle ${settings.launchAtLogin ? "toggle--on" : ""}`} aria-pressed={settings.launchAtLogin} onClick={() => void updateSettings({ launchAtLogin: !settings.launchAtLogin })}><span /></button>
      </SettingRow>
      <SettingRow icon={Monitor} label="Show dictation bar" description="Display the small non-activating bar while Lyra is running.">
        <button className={`toggle ${settings.showBar ? "toggle--on" : ""}`} aria-pressed={settings.showBar} onClick={() => void updateSettings({ showBar: !settings.showBar })}><span /></button>
      </SettingRow>
      <SettingRow icon={Sparkles} label="Keep local model warm" description="Use more memory for faster first-word latency.">
        <button className={`toggle ${settings.keepModelWarm ? "toggle--on" : ""}`} aria-pressed={settings.keepModelWarm} onClick={() => void updateSettings({ keepModelWarm: !settings.keepModelWarm })}><span /></button>
      </SettingRow>
    </div>
  </>;
}

function DictationSection({ settings, updateSettings }: { settings: Settings; updateSettings: (patch: Partial<Settings>) => Promise<void> }) {
  return <>
    <SectionHeader eyebrow="Activation" title="Dictation" description="Choose how a short gesture becomes text in the active app." />
    <div className="settings-card">
      <SettingRow icon={Keyboard} label="Activation mode" description="Push-to-talk records while the shortcut is held; toggle records between presses.">
        <Segmented value={settings.activationMode} options={[{ value: "push_to_talk", label: "Push-to-talk" }, { value: "toggle", label: "Toggle" }]} onChange={(value) => void updateSettings({ activationMode: value as ActivationMode })} />
      </SettingRow>
      <SettingRow icon={Keyboard} label="Global shortcut" description="The shortcut Lyra listens for outside its own windows.">
        <span className="keycap">{settings.hotkey}</span>
      </SettingRow>
      <SettingRow icon={Timer} label="Maximum duration" description="Long recordings stop safely and are processed in memory.">
        <select value={settings.maxDurationSeconds} onChange={(event) => void updateSettings({ maxDurationSeconds: Number(event.target.value) })}>
          {[30, 60, 120, 180].map((seconds) => <option key={seconds} value={seconds}>{seconds} sec</option>)}
        </select>
      </SettingRow>
      <SettingRow icon={Clipboard} label="Paste automatically" description="Restore the prior text clipboard only when no other app changed it.">
        <button className={`toggle ${settings.autoPaste ? "toggle--on" : ""}`} aria-pressed={settings.autoPaste} onClick={() => void updateSettings({ autoPaste: !settings.autoPaste })}><span /></button>
      </SettingRow>
    </div>
    <div className="info-callout"><ShieldCheck size={16} /><span>Lyra never presses Enter. If focus or injection fails, the transcript remains available to paste manually.</span></div>
  </>;
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

function SettingsApp({ settings, runtime, session, lastResult, devices, credentialConfigured, artifacts, updateSettings, setCredentialConfigured, refresh }: ReturnType<typeof useLyraState>) {
  const [section, setSection] = useState<SettingsSection>("general");
  const [copied, setCopied] = useState(false);
  const [onboardingDismissed, setOnboardingDismissed] = useState(settings.onboardingComplete);

  const copy = async () => {
    const text = await copyLastTranscript();
    if (text !== null) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    }
  };

  const content = useMemo(() => {
    switch (section) {
      case "general": return <><SetupChecklist runtime={runtime} settings={settings} devices={devices} artifacts={artifacts} /><GeneralSection settings={settings} updateSettings={updateSettings} /></>;
      case "dictation": return <DictationSection settings={settings} updateSettings={updateSettings} />;
      case "audio": return <AudioSection settings={settings} devices={devices} updateSettings={updateSettings} />;
      case "language": return <LanguageSection settings={settings} updateSettings={updateSettings} />;
      case "transcription": return <TranscriptionSection settings={settings} updateSettings={updateSettings} runtime={runtime} lastResult={lastResult} />;
      case "cloud": return <CloudSection credentialConfigured={credentialConfigured} setCredentialConfigured={setCredentialConfigured} />;
      case "appearance": return <AppearanceSection settings={settings} updateSettings={updateSettings} />;
      case "diagnostics": return <DiagnosticsSection runtime={runtime} session={session} refresh={refresh} />;
    }
  }, [lastResult, refresh, runtime, section, session, settings, updateSettings]);

  return <main className="settings-shell">
    <aside className="sidebar">
      <div className="brand-lockup"><span className="brand-orbit"><span /></span><div><strong>Lyra</strong><span>Orion voice layer</span></div></div>
      <div className="sidebar-status"><StatusDot tone={runtime.runtimeInstalled && runtime.modelInstalled ? "good" : "warn"} /><span>{runtime.runtimeInstalled && runtime.modelInstalled ? "Ready locally" : "Setup in progress"}</span></div>
      <nav className="settings-nav" aria-label="Settings">
        {(Object.keys(sectionLabels) as SettingsSection[]).map((item) => <button key={item} className={section === item ? "nav-active" : ""} onClick={() => setSection(item)}><NavIcon section={item} /><span>{sectionLabels[item]}</span>{section === item && <ChevronRight size={14} />}</button>)}
      </nav>
      <div className="sidebar-footer"><button className="sidebar-footer-button" onClick={() => void copy()}><Clipboard size={14} /> {copied ? "Copied" : "Copy last transcription"}</button><span>v0.1 · Windows x64</span></div>
    </aside>
    <section className="settings-content">
      <header className="topbar"><div className="window-title"><span className="window-title-dot" />Settings</div><div className="topbar-actions"><button className="icon-button" title="Open dictation bar" onClick={() => void showSettings()}><MoreHorizontal size={17} /></button><button className="window-close" title="Hide settings" onClick={() => void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().hide())}><X size={16} /></button></div></header>
      <div className="content-scroll">
        {!onboardingDismissed && <div className="onboarding-banner"><div className="onboarding-icon"><Sparkles size={17} /></div><div><strong>Welcome to Lyra</strong><span>Finish setup when you are ready. Downloads require your explicit approval.</span></div><button onClick={() => { setOnboardingDismissed(true); void updateSettings({ onboardingComplete: true }); }}>Got it <ArrowLeft size={14} className="rotate-180" /></button></div>}
        {content}
        <footer className="settings-footer"><span><StatusDot tone="good" />Lyra is running in the tray</span><span>Audio stays in memory</span></footer>
      </div>
    </section>
  </main>;
}

function NavIcon({ section }: { section: SettingsSection }) {
  const icons: Record<SettingsSection, typeof Settings2> = { general: Settings2, dictation: Keyboard, audio: Mic, language: Globe2, transcription: Cpu, cloud: Cloud, appearance: SlidersHorizontal, diagnostics: Activity };
  const Icon = icons[section];
  return <Icon size={15} />;
}

export default function App() {
  const state = useLyraState();
  const label = useWindowLabel();
  if (label === "bar") return <DictationBar session={state.session} settings={state.settings} runtime={state.runtime} />;
  return <SettingsApp {...state} />;
}
