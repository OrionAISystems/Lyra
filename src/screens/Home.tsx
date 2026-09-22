import { BookOpen, FileText, Mic, Settings2 } from "lucide-react";
import { startDictation, stopDictation } from "../bridge";
import { Card, Page, Shortcut } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
import type { Screen } from "../types";
export default function Home({
  state,
  navigate,
}: {
  state: LyraState;
  navigate: (s: Screen) => void;
}) {
  const active = state.session.state === "listening";
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
        <h1>{copy[0]}</h1>
        <p>
          {active ? (
            "Release the shortcut when you’re finished."
          ) : (
            <>
              Hold <Shortcut value={state.settings.hotkey} /> and speak
            </>
          )}
        </p>
        <button
          className={`mic-orb ${active ? "active" : ""}`}
          aria-label={active ? "Stop dictation" : "Start dictation"}
          onClick={() => void (active ? stopDictation() : startDictation())}
        >
          <Mic size={34} />
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
