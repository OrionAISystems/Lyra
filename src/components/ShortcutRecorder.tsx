import { Keyboard, Plus, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { listenToRecordedShortcut, setShortcutRecording } from "../bridge";

/** Use physical key codes, so shifted punctuation and keyboard layouts stay stable. */
export function shortcutFromEvent(event: Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">): string | null {
  const key = /^(Key[A-Z]|Digit[0-9])$/.test(event.code)
    ? event.code.replace(/^(Key|Digit)/, "")
    : event.code;
  if (!key || /^(Control|Alt|Shift|Meta|OS)(Left|Right)$/.test(key)) return null;
  const modifiers = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super"].filter(Boolean);
  if (!modifiers.length && !/^F(1[3-9]|2[0-4])$/.test(key)) return null;
  return [...modifiers, key].join("+");
}

export function ShortcutRecorder({ value, label, onChange, onRemove, disabled, onRecordingChange }: {
  value?: string; label: string; onChange: (value: string) => Promise<void>; onRemove?: () => void;
  disabled?: boolean; onRecordingChange?: (active: boolean) => void;
}) {
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const button = useRef<HTMLButtonElement>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const onRecordingChangeRef = useRef(onRecordingChange);
  onRecordingChangeRef.current = onRecordingChange;

  useEffect(() => {
    if (!recording) return;
    let disposed = false;
    let finished = false;
    let unlisten = () => {};
    let pending: { code: string; shortcut: string } | null = null;
    const finish = async (shortcut?: string) => {
      if (finished || disposed) return;
      finished = true;
      setBusy(true);
      try {
        await setShortcutRecording(false);
        if (shortcut) await onChangeRef.current(shortcut);
      } catch (cause) {
        setError(String(cause));
      } finally {
        if (!disposed) { setBusy(false); setRecording(false); button.current?.focus(); }
      }
    };
    const keydown = (event: KeyboardEvent) => {
      event.preventDefault(); event.stopPropagation();
      if (event.code === "Escape") { void finish(); return; }
      if (event.repeat || finished) return;
      const shortcut = shortcutFromEvent(event);
      if (shortcut) { pending = { code: event.code, shortcut }; setPreview(shortcut); setError(null); }
      else if (!/^(Control|Alt|Shift|Meta)(Left|Right)$/.test(event.code)) {
        pending = null; setPreview(null); setError("Use a modifier combination or a single F13–F24 key.");
      }
    };
    const keyup = (event: KeyboardEvent) => {
      event.preventDefault(); event.stopPropagation();
      if (pending?.code === event.code) void finish(pending.shortcut);
    };
    const cancel = () => { void finish(); };
    // Subscribe before enabling native interception; registered chords do not reach the webview.
    void (async () => {
      try {
        unlisten = await listenToRecordedShortcut((shortcut) => { void finish(shortcut); });
        if (disposed) { unlisten(); return; }
        await setShortcutRecording(true);
        if (disposed) { await setShortcutRecording(false); return; }
        setBusy(false);
        window.addEventListener("keydown", keydown, true);
        window.addEventListener("keyup", keyup, true);
      } catch (cause) { setError(String(cause)); setBusy(false); setRecording(false); }
    })();
    window.addEventListener("blur", cancel);
    const timeout = window.setTimeout(cancel, 25000);
    return () => {
      disposed = true; unlisten(); window.clearTimeout(timeout);
      onRecordingChangeRef.current?.(false);
      window.removeEventListener("keydown", keydown, true);
      window.removeEventListener("keyup", keyup, true);
      window.removeEventListener("blur", cancel);
      void setShortcutRecording(false).catch(() => {});
    };
  }, [recording]);

  return <div className="shortcut-editor">
    <div className="shortcut-pill-group">
      <button ref={button} type="button" className={`shortcut-pill ${recording ? "recording" : ""}`}
        aria-label={recording ? `Recording ${label}` : `Change ${label}`} aria-pressed={recording}
        disabled={busy || disabled} onClick={() => { if (!recording) { onRecordingChange?.(true); setError(null); setPreview(null); setBusy(true); setRecording(true); } }}>
        {value ? <Keyboard size={15} aria-hidden="true" /> : <Plus size={15} aria-hidden="true" />}
        {recording ? (preview ?? "Press keys…") : value ? value.split("+").map((key, index) => <kbd key={`${index}-${key}`}>{key}</kbd>) : "Add shortcut"}
      </button>
      {onRemove && !recording && <button type="button" className="shortcut-remove" disabled={disabled} aria-label={`Remove ${label}`} onClick={onRemove}><X size={14} /></button>}
    </div>
    {recording && <small className="shortcut-hint" role="status">Release to save · Esc to cancel</small>}
    {error && <small className="inline-error" role="alert">{error}</small>}
  </div>;
}
