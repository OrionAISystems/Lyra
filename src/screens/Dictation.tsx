import { Clock3, Keyboard, Pilcrow, Radio, Send, Timer } from "lucide-react";
import { useState } from "react";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { Card, Page, Row, Select, Toggle } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
export default function Dictation({ state }: { state: LyraState }) {
  const s = state.settings,
    u = state.updateSettings;
  const [activeRecorder, setActiveRecorder] = useState<string | null>(null);
  const bindings = [s.hotkey, ...s.additionalHotkeys];
  const recorder = (label: string, value: string | undefined, onChange: (value: string) => Promise<void>, onRemove?: () => void) =>
    <ShortcutRecorder key={label} label={label} value={value} onChange={onChange} onRemove={onRemove}
      disabled={activeRecorder !== null && activeRecorder !== label} onRecordingChange={(active) => setActiveRecorder(active ? label : null)} />;
  return (
    <Page
      title="Dictation"
      subtitle="Control how Lyra listens and turns your speech into text."
    >
      <Card>
        <Row
          icon={<Keyboard />}
          title="Bar activation"
          description={
            s.activationMode === "push_to_talk"
              ? "Push-to-talk while held"
              : "Press once to start and once to stop"
          }
        >
          <Select
            label="Bar activation"
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
          title="Global shortcuts"
          className="shortcut-settings-row"
          description="The shortcut that starts a recording controls when it stops"
        >
          <div className="shortcut-list">
            {bindings.map((binding, index) => (
              <div className="shortcut-binding" key={index}>
                {recorder(`dictation shortcut ${index + 1}`, binding, async (value) => {
                  const next = bindings.map((old, i) => i === index ? value : old);
                  const modes = { ...s.dictationHotkeyModes };
                  const mode = modes[binding] ?? s.activationMode;
                  delete modes[binding]; modes[value] = mode;
                  await u({ hotkey: next[0], additionalHotkeys: next.slice(1), dictationHotkeyModes: modes });
                }, bindings.length > 1 ? () => {
                  const next = bindings.filter((_, i) => i !== index);
                  const modes = { ...s.dictationHotkeyModes }; delete modes[binding];
                  void u({ hotkey: next[0], additionalHotkeys: next.slice(1), dictationHotkeyModes: modes });
                } : undefined)}
                <Select label={`Dictation shortcut ${index + 1} behavior`} disabled={activeRecorder !== null}
                  value={s.dictationHotkeyModes[binding] ?? s.activationMode}
                  onChange={(mode) => void u({ dictationHotkeyModes: { ...s.dictationHotkeyModes, [binding]: mode as typeof s.activationMode } })}>
                  <option value="push_to_talk">Hold</option>
                  <option value="toggle">Toggle</option>
                </Select>
              </div>
            ))}
            {recorder("new dictation shortcut", undefined, async (value) => {
              await u({ additionalHotkeys: [...s.additionalHotkeys, value],
                dictationHotkeyModes: { ...s.dictationHotkeyModes, [value]: "push_to_talk" } });
            })}
          </div>
        </Row>
        <Row
          icon={<Keyboard />}
          title="Scratchpad shortcut"
          className="shortcut-settings-row"
          description="Record a thought directly into Scratchpad"
        >
          <div className="shortcut-binding">
            {recorder("Scratchpad shortcut", s.scratchpadHotkey, async (value) => { await u({ scratchpadHotkey: value }); })}
            <Select label="Scratchpad shortcut behavior" disabled={activeRecorder !== null}
              value={s.scratchpadActivationMode ?? s.activationMode}
              onChange={(mode) => void u({ scratchpadActivationMode: mode as typeof s.activationMode })}>
              <option value="push_to_talk">Hold</option><option value="toggle">Toggle</option>
            </Select>
          </div>
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
          {recorder("Paste last shortcut", s.recoveryShortcuts.pasteLast ?? undefined, async (value) => { await u({ recoveryShortcuts: { ...s.recoveryShortcuts, pasteLast: value } }); },
            s.recoveryShortcuts.pasteLast ? () => { void u({ recoveryShortcuts: { ...s.recoveryShortcuts, pasteLast: null } }); } : undefined)}
        </Row>
        <Row
          icon={<Keyboard />}
          title="Open settings"
          description="Required before the tray icon can be hidden"
        >
          {recorder("Open settings shortcut", s.recoveryShortcuts.openSettings ?? undefined, async (value) => { await u({ recoveryShortcuts: { ...s.recoveryShortcuts, openSettings: value } }); },
            s.recoveryShortcuts.openSettings ? () => { void u({ recoveryShortcuts: { ...s.recoveryShortcuts, openSettings: null } }); } : undefined)}
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
