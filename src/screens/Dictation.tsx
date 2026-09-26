import { Clock3, Keyboard, Pilcrow, Radio, Send, Timer } from "lucide-react";
import { useEffect, useState } from "react";
import { Card, Page, Row, Select, Toggle } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
export default function Dictation({ state }: { state: LyraState }) {
  const s = state.settings,
    u = state.updateSettings;
  const [scratchpadShortcutDraft, setScratchpadShortcutDraft] = useState(s.scratchpadHotkey);
  useEffect(() => setScratchpadShortcutDraft(s.scratchpadHotkey), [s.scratchpadHotkey]);
  return (
    <Page
      title="Dictation"
      subtitle="Control how Lyra listens and turns your speech into text."
    >
      <Card>
        <Row
          icon={<Keyboard />}
          title="Activation mode"
          description={
            s.activationMode === "push_to_talk"
              ? "Push-to-talk while held"
              : "Press once to start and once to stop"
          }
        >
          <Select
            label="Activation mode"
            value={s.activationMode}
            onChange={(v) =>
              void u({ activationMode: v as typeof s.activationMode })
            }
          >
            <option value="push_to_talk">Push-to-talk</option>
            <option value="toggle">Toggle</option>
          </Select>
        </Row>
        <Row
          icon={<Keyboard />}
          title="Global shortcut"
          description="Start or stop dictation"
        >
          <input
            value={s.hotkey}
            onChange={(e) => void u({ hotkey: e.target.value })}
          />
        </Row>
        <Row
          icon={<Keyboard />}
          title="Scratchpad shortcut"
          description="Record a thought directly into Scratchpad"
        >
          <input
            aria-label="Scratchpad shortcut"
            aria-describedby="scratchpad-shortcut-status"
            value={scratchpadShortcutDraft}
            onChange={(e) => setScratchpadShortcutDraft(e.target.value)}
            onBlur={() => {
              const value = scratchpadShortcutDraft.trim();
              if (!value) setScratchpadShortcutDraft(s.scratchpadHotkey);
              else if (value !== s.scratchpadHotkey) void u({ scratchpadHotkey: value });
            }}
          />
        </Row>
        <Row
          icon={<Timer />}
          title="Auto-stop after silence"
          description="Stop recording when silence is detected"
        >
          <Toggle
            label="Auto stop"
            checked={s.autoStopAfterSilenceMs !== null}
            onChange={(v) =>
              void u({ autoStopAfterSilenceMs: v ? 1500 : null })
            }
          />
        </Row>
        <Row
          icon={<Clock3 />}
          title="Maximum duration"
          description="Long recordings stop automatically"
        >
          <Select
            label="Maximum duration"
            value={s.maxDurationSeconds}
            onChange={(v) => void u({ maxDurationSeconds: Number(v) })}
          >
            <option value="120">2 minutes</option>
            <option value="300">5 minutes</option>
            <option value="600">10 minutes</option>
          </Select>
        </Row>
        <Row
          icon={<Send />}
          title="Automatic paste"
          description="Paste the final result into the original app"
        >
          <Toggle
            label="Automatic paste"
            checked={s.autoPaste}
            onChange={(v) => void u({ autoPaste: v })}
          />
        </Row>
        <Row
          icon={<Pilcrow />}
          title="Spoken formatting"
          description="Enable commands like “new line” or “comma”"
        >
          <Toggle
            label="Spoken formatting"
            checked={s.formattingCommandsEnabled}
            onChange={(v) => void u({ formattingCommandsEnabled: v })}
          />
        </Row>
      </Card>
      <h2 className="section-title">Recovery shortcuts</h2>
      <Card>
        <Row
          icon={<Radio />}
          title="Paste last"
          description="Recover the most recent in-memory result"
        >
          <input
            placeholder="Not assigned"
            value={s.recoveryShortcuts.pasteLast ?? ""}
            onChange={(e) =>
              void u({
                recoveryShortcuts: {
                  ...s.recoveryShortcuts,
                  pasteLast: e.target.value || null,
                },
              })
            }
          />
        </Row>
        <Row
          icon={<Keyboard />}
          title="Open settings"
          description="Required before the tray icon can be hidden"
        >
          <input
            value={s.recoveryShortcuts.openSettings ?? ""}
            onChange={(e) =>
              void u({
                recoveryShortcuts: {
                  ...s.recoveryShortcuts,
                  openSettings: e.target.value || null,
                },
              })
            }
          />
        </Row>
      </Card>
      {state.hotkey.error && (
        <p className="inline-error">{state.hotkey.error}</p>
      )}
      {state.scratchpadHotkey.error && (
        <p id="scratchpad-shortcut-status" className="inline-error">Scratchpad shortcut: {state.scratchpadHotkey.error}</p>
      )}
      {!state.scratchpadHotkey.error && (
        <p id="scratchpad-shortcut-status" className="inline-hint">
          Scratchpad shortcut {state.scratchpadHotkey.registered ? "is ready" : "is not registered"} · {state.scratchpadHotkey.hotkey}
        </p>
      )}
      {state.settingsError && <p className="inline-error">{state.settingsError}</p>}
    </Page>
  );
}
