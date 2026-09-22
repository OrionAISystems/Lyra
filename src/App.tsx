import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Mic, Settings2, X } from "lucide-react";
import {
  cancelDictation,
  isTauri,
  showSettings,
  startDictation,
  stopDictation,
} from "./bridge";
import { Shell } from "./components/Shell";
import Home from "./screens/Home";
import Dictation from "./screens/Dictation";
import Personalization from "./screens/Personalization";
import AudioEngine from "./screens/AudioEngine";
import Insights from "./screens/Insights";
import Appearance from "./screens/Appearance";
import Diagnostics from "./screens/Diagnostics";
import { useLyra } from "./state/useLyra";
import type { Screen } from "./types";
function DictationBar() {
  const state = useLyra();
  const active = state.session.state === "listening";
  return (
    <div className={`dictation-bar ${state.session.state}`}>
      <button
        className="bar-mic"
        aria-label={active ? "Stop dictation" : "Start dictation"}
        onClick={() => void (active ? stopDictation() : startDictation())}
      >
        <Mic />
      </button>
      <div>
        <strong>
          {active
            ? "Listening…"
            : state.session.state === "processing"
              ? "Processing…"
              : state.session.state === "error"
                ? "Needs attention"
                : "Ready"}
        </strong>
        <span>
          {state.partial?.text || state.session.error || state.settings.hotkey}
        </span>
      </div>
      <button aria-label="Open settings" onClick={() => void showSettings()}>
        <Settings2 />
      </button>
      {active && (
        <button aria-label="Cancel" onClick={() => void cancelDictation()}>
          <X />
        </button>
      )}
    </div>
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
