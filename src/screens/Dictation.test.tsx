// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import Dictation from "./Dictation";
import { defaultSettings, type Settings } from "../types";
import type { LyraState } from "../state/useLyra";

vi.mock("../bridge", () => ({ setShortcutRecording: async () => {}, listenToRecordedShortcut: async () => () => {} }));
let root: Root;
let container: HTMLDivElement;
let settings: Settings;
function Harness() {
  const [value, setValue] = useState<Settings>({ ...defaultSettings, additionalHotkeys: ["F20"], dictationHotkeyModes: { "Ctrl+Alt+Space": "push_to_talk", F20: "toggle" } });
  settings = value;
  return <Dictation state={{ settings: value, updateSettings: async (patch) => setValue((old) => ({ ...old, ...patch })), hotkey: { error: null }, scratchpadHotkey: { error: null, registered: true } } as LyraState} />;
}
beforeEach(async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  await act(async () => root.render(<Harness />));
});
afterEach(async () => { await act(async () => root.unmount()); container.remove(); });
async function click(label: string) {
  await act(async () => { (container.querySelector(`button[aria-label="${label}"]`) as HTMLButtonElement).click(); });
}
async function choose(label: string, value: string) {
  await click(label);
  await act(async () => { (Array.from(container.querySelectorAll('[role="option"]')).find((option) => option.textContent === value) as HTMLButtonElement).click(); });
}
it("keeps bar, each dictation binding and Scratchpad modes independent", async () => {
  await choose("Bar activation", "Toggle");
  expect(settings.dictationHotkeyModes).toEqual({ "Ctrl+Alt+Space": "push_to_talk", F20: "toggle" });
  await choose("Dictation shortcut 1 behavior", "Toggle");
  expect(settings.activationMode).toBe("toggle");
  await choose("Scratchpad shortcut behavior", "Toggle");
  await choose("Dictation shortcut 2 behavior", "Hold");
  expect(settings.dictationHotkeyModes.F20).toBe("push_to_talk");
  expect(settings.dictationHotkeyModes["Ctrl+Alt+Space"]).toBe("toggle");
  expect(settings.scratchpadActivationMode).toBe("toggle");
});
it("preserves a binding's behavior when its keys change and when it becomes primary", async () => {
  await click("Change dictation shortcut 2");
  await act(async () => {
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyK", ctrlKey: true, bubbles: true }));
    window.dispatchEvent(new KeyboardEvent("keyup", { code: "KeyK", bubbles: true }));
  });
  expect(settings.additionalHotkeys).toEqual(["Ctrl+K"]);
  expect(settings.dictationHotkeyModes["Ctrl+K"]).toBe("toggle");
  expect(settings.dictationHotkeyModes.F20).toBeUndefined();
  await click("Remove dictation shortcut 1");
  expect(settings.hotkey).toBe("Ctrl+K");
  expect(settings.dictationHotkeyModes).toEqual({ "Ctrl+K": "toggle" });
});
