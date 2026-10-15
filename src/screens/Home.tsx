import { BookOpen, FileText, Mic, Settings2 } from "lucide-react";
import { Fragment } from "react";
import { startDictation, stopDictation } from "../bridge";
import { Card, Page, Shortcut } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
import type { Screen } from "../types";
import { SeasonalAmbience, SeasonalArtwork } from "../components/SeasonalArtwork";
export default function Home({
  state,
  navigate,
}: {
  state: LyraState;
  navigate: (s: Screen) => void;
}) {
  const active = state.session.state === "listening";
  const theme = state.seasonal.content;
  const labels = {
    idle: ["Ready to dictate", "Lyra is ready when you are."],
    listening: [
      "Listening…",
      "Speak naturally. Your audio stays on this device.",
    ],
    processing: [
      "Turning speech into text…",
      "Only the final result will be pasted.",
    ],
    success: ["Done", "Your text is ready."],
    error: [
      "Something needs attention",
      state.session.error ?? "Open Diagnostics for details.",
    ],
  } as const;
  const copy = labels[state.session.state];
  return (
    <Page title="" subtitle="">
      <div className="hero">
        {theme && <SeasonalAmbience theme={theme} />}
        <h1>{copy[0]}</h1>
        <p>
          {active ? (
            "Release the shortcut you held, or press your toggle shortcut again to finish."
          ) : (
            <>
              {[state.settings.hotkey, ...state.settings.additionalHotkeys].map((shortcut, index) => (
                <Fragment key={shortcut}>{index > 0 && " or "}{(state.settings.dictationHotkeyModes[shortcut] ?? state.settings.activationMode) === "push_to_talk" ? "Hold " : "Press "}<Shortcut value={shortcut} /></Fragment>
              ))} and speak
            </>
          )}
        </p>
        <button
          className={`mic-orb ${active ? "active" : ""}${theme ? " mic-orb--seasonal" : ""}`}
          aria-label={active ? "Stop dictation" : "Start dictation"}
          onClick={() => void (active ? stopDictation() : startDictation())}
        >
          {theme ? <SeasonalArtwork theme={theme} state={state.session.state} inputLevel={state.session.inputLevel} /> : <Mic size={34} />}
          <i
            style={{
              transform: `scaleX(${Math.max(0.18, state.session.inputLevel)})`,
            }}
          />
        </button>
        <span className="hero-status">{state.partial?.text || copy[1]}</span>
      </div>
      <div className="quick-grid">
        <button onClick={() => navigate("personalization")}>
          <FileText />
          <strong>Snippets</strong>
          <span>Insert saved text</span>
        </button>
        <button onClick={() => navigate("personalization")}>
          <BookOpen />
          <strong>Vocabulary</strong>
          <span>Manage words</span>
        </button>
        <button onClick={() => navigate("appearance")}>
          <Settings2 />
          <strong>Settings</strong>
          <span>Customize Lyra</span>
        </button>
      </div>
      <Card className="home-context">
        <span className="ready-dot" /> Ready locally{" "}
        <b>
          {state.settings.localEngine === "parakeet_tdt"
            ? "Parakeet TDT"
            : "Nemotron Streaming"}
        </b>
        <span>{state.usage.dictationsToday} sessions today</span>
      </Card>
      <blockquote>“Turn your voice into progress.”</blockquote>
    </Page>
  );
}
