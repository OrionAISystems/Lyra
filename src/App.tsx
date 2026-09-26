import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Mic, Power, Settings2, X } from "lucide-react";
import {
  copyLastTranscript,
  hideBar,
  isTauri,
  quitLyra,
  showSettings,
  startDictation,
  stopDictation,
  stopScratchpad,
} from "./bridge";
import { Shell } from "./components/Shell";
import Home from "./screens/Home";
import Dictation from "./screens/Dictation";
import Personalization from "./screens/Personalization";
import AudioEngine from "./screens/AudioEngine";
import Insights from "./screens/Insights";
import History from "./screens/History";
import Scratchpad from "./screens/Scratchpad";
import Appearance from "./screens/Appearance";
import Diagnostics from "./screens/Diagnostics";
import { useLyra } from "./state/useLyra";
import { isPrimaryPointerButton, primaryBarGesture } from "./barActivation";
import type { Screen } from "./types";
function DictationBar() {
  const state = useLyra();
  const barRef = useRef<HTMLElement>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [displayDurationMs, setDisplayDurationMs] = useState(0);
  const pttActive = useRef(false);
  const pttReleasePending = useRef(false);
  const pttStart = useRef<Promise<unknown> | null>(null);
  const displayStartedAt = useRef<{
    requestId: string | null;
    startedAt: number;
  } | null>(null);
  const active = state.session.state === "listening";
  const processing = state.session.state === "processing";
  const error = state.session.state === "error";
  const sessionDurationMs = Math.max(0, state.session.durationMs);
  const maxDurationSeconds = Math.max(
    1,
    Math.floor(state.settings.maxDurationSeconds),
  );
  const pushToTalk =
    primaryBarGesture(state.settings.activationMode) ===
    "pointer_press_release";
  const stopActiveSession = () =>
    state.session.destination === "scratchpad"
      ? stopScratchpad()
      : stopDictation("bar");

  useEffect(() => {
    if (!active) {
      setDisplayDurationMs(sessionDurationMs);
      displayStartedAt.current = null;
      return;
    }
    if (displayStartedAt.current?.requestId !== state.session.requestId) {
      displayStartedAt.current = {
        requestId: state.session.requestId,
        startedAt: performance.now() - sessionDurationMs,
      };
    }
    const startedAt = displayStartedAt.current.startedAt;
    const updateDuration = () => {
      setDisplayDurationMs(
        Math.min(performance.now() - startedAt, maxDurationSeconds * 1_000),
      );
    };
    updateDuration();
    const interval = window.setInterval(updateDuration, 250);
    return () => window.clearInterval(interval);
  }, [active, maxDurationSeconds, sessionDurationMs, state.session.requestId]);

  useEffect(() => {
    if (!menuOpen) return;
    const close = (event: PointerEvent) => {
      if (!barRef.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setMenuOpen(false);
    };
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", escape);
    };
  }, [menuOpen]);

  const begin = () => {
    if (processing || pttActive.current) return;
    pttActive.current = true;
    const request = startDictation("bar");
    pttStart.current = request;
    void request.then(
      () => {
        pttStart.current = null;
      },
      () => {
        pttStart.current = null;
      },
    );
  };
  const end = async () => {
    if (!pttActive.current || pttReleasePending.current) return;
    pttActive.current = false;
    pttReleasePending.current = true;
    try {
      await pttStart.current;
      await stopActiveSession();
    } finally {
      pttReleasePending.current = false;
    }
  };
  const waveform = Array.from({ length: 18 }, (_, index) => (
    <span
      key={index}
      style={{
        height: `${active ? Math.max(3, 4 + ((index * 7) % 10) * (0.35 + state.session.inputLevel)) : 3}px`,
      }}
    />
  ));

  return (
    <main
      ref={barRef}
      className={`bar-shell bar-shell--${state.session.state}`}
      onContextMenu={(event) => {
        event.preventDefault();
        setMenuOpen(true);
      }}
    >
      <button
        className="bar-surface"
        aria-label={active
          ? state.session.destination === "scratchpad"
            ? "Stop Scratchpad recording"
            : "Stop dictation"
          : "Start dictation"}
        aria-pressed={active}
        onClick={
          pushToTalk
            ? undefined
            : () => void (active ? stopActiveSession() : startDictation("bar"))
        }
        onPointerDown={(event) => {
          if (!pushToTalk || !isPrimaryPointerButton(event.button)) return;
          event.currentTarget.setPointerCapture(event.pointerId);
          begin();
        }}
        onPointerUp={pushToTalk ? () => void end() : undefined}
        onPointerCancel={pushToTalk ? () => void end() : undefined}
        onLostPointerCapture={pushToTalk ? () => void end() : undefined}
      >
        <span className="bar-leading" aria-hidden="true">
          {active ? (
            <span className="recording-dot" />
          ) : processing ? (
            <span className="mini-spinner" />
          ) : error ? (
            <X size={11} />
          ) : (
            <Mic size={11} />
          )}
        </span>
        {active ? (
          <>
            <span className="bar-waveform">{waveform}</span>
            {state.session.destination === "scratchpad" && (
              <span className="bar-destination">Scratchpad</span>
            )}
            <span className="bar-timer">
              {Math.floor(displayDurationMs / 1_000)}s/{maxDurationSeconds}s
            </span>
          </>
        ) : processing ? (
          <span className="bar-label">
            {state.session.destination === "scratchpad" ? "Saving thought…" : "Transcribing…"}
          </span>
        ) : error ? (
          <span className="bar-label">Try again</span>
        ) : (
          <span className="bar-idle-mark" aria-hidden="true" />
        )}
      </button>
      {menuOpen && (
        <div className="bar-menu" role="menu">
          <button
            type="button"
            onClick={() => {
              setMenuOpen(false);
              void showSettings();
            }}
          >
            <Settings2 size={13} /> Settings
          </button>
          <button
            type="button"
            onClick={() => {
              setMenuOpen(false);
              void hideBar();
            }}
          >
            <X size={13} /> Hide bar
          </button>
          <button
            type="button"
            className="bar-menu-danger"
            onClick={() => void quitLyra()}
          >
            <Power size={13} /> Quit Lyra
          </button>
        </div>
      )}
      {error && (
        <button
          type="button"
          className="bar-recovery"
          onClick={() => void copyLastTranscript()}
          title="Copy the last transcript"
        >
          {state.session.error ?? "Clipboard recovery available"}
        </button>
      )}
      {state.runtime.runtimeInstalled === false && (
        <span className="bar-runtime-hint">Local runtime not installed</span>
      )}
    </main>
  );
}
export default function App() {
  const state = useLyra();
  const [screen, setScreen] = useState<Screen>(
    () =>
      (new URLSearchParams(location.search).get("screen") as Screen) || "home",
  );
  const [bar, setBar] = useState(false);
  useEffect(() => {
    if (state.scratchpadOpenId) setScreen("scratchpad");
  }, [state.scratchpadOpenId]);
  useEffect(() => {
    const isBar = isTauri && getCurrentWindow().label === "bar";
    setBar(isBar);
    document.documentElement.classList.toggle("bar-window", isBar);
  }, []);
  if (bar) return <DictationBar />;
  const pages = {
    home: <Home state={state} navigate={setScreen} />,
    dictation: <Dictation state={state} />,
    personalization: <Personalization state={state} />,
    audio: <AudioEngine state={state} />,
    insights: <Insights state={state} />,
    history: <History state={state} />,
    scratchpad: <Scratchpad state={state} />,
    appearance: <Appearance state={state} />,
    diagnostics: <Diagnostics state={state} />,
  };
  return (
    <Shell
      screen={screen}
      setScreen={setScreen}
      ready={state.runtime.runtimeVerified && state.runtime.modelVerified}
    >
      {pages[screen]}
    </Shell>
  );
}
