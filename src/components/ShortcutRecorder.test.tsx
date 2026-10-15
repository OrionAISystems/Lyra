// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ShortcutRecorder, shortcutFromEvent } from "./ShortcutRecorder";

const native = vi.hoisted(() => ({ set: vi.fn(), listen: vi.fn(), recorded: null as null | ((value: string) => void), unlisten: vi.fn() }));
vi.mock("../bridge", () => ({ setShortcutRecording: native.set, listenToRecordedShortcut: native.listen }));
let root: Root;
let container: HTMLDivElement;
let changed: ReturnType<typeof vi.fn<(value: string) => Promise<void>>>;
beforeEach(async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  vi.clearAllMocks();
  native.set.mockResolvedValue(null);
  native.listen.mockImplementation(async (callback) => { native.recorded = callback; return native.unlisten; });
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  changed = vi.fn().mockResolvedValue(undefined);
  await act(async () => { root.render(<ShortcutRecorder label="dictation" value="Ctrl+Alt+Space" onChange={changed} />); });
});
afterEach(async () => { await act(async () => root.unmount()); container.remove(); });
async function begin() { await act(async () => (container.querySelector("button") as HTMLButtonElement).click()); }
async function key(type: string, code: string, options: KeyboardEventInit = {}) {
  await act(async () => { window.dispatchEvent(new KeyboardEvent(type, { code, bubbles: true, cancelable: true, ...options })); });
}
it("records F20 on release and preserves the prior binding until then", async () => {
  await begin(); await key("keydown", "F20"); expect(changed).not.toHaveBeenCalled();
  expect(container.textContent).toContain("F20");
  await key("keyup", "F20"); expect(changed).toHaveBeenCalledExactlyOnceWith("F20");
  expect(native.set).toHaveBeenCalledWith(false); expect(native.unlisten).toHaveBeenCalled();
});
it("records a modifier chord and ignores modifier-only presses and repeats", async () => {
  await begin(); await key("keydown", "ControlLeft", { ctrlKey: true });
  await key("keydown", "Space", { ctrlKey: true, altKey: true });
  await key("keydown", "Space", { ctrlKey: true, altKey: true, repeat: true });
  await key("keyup", "ControlLeft"); expect(changed).not.toHaveBeenCalled();
  await key("keyup", "Space"); expect(changed).toHaveBeenCalledExactlyOnceWith("Ctrl+Alt+Space");
});
it("rejects typing keys and F1–F12, and Escape cancels without saving", async () => {
  await begin(); await key("keydown", "KeyA"); await key("keyup", "KeyA");
  expect(container.textContent).toContain("Use a modifier");
  await key("keydown", "F12"); await key("keyup", "F12");
  await key("keydown", "Escape"); expect(changed).not.toHaveBeenCalled();
  expect(container.querySelector("button")?.getAttribute("aria-pressed")).toBe("false");
});
it("captures an already-registered chord through the native event once", async () => {
  await begin();
  await act(async () => { native.recorded?.("Ctrl+Alt+Space"); native.recorded?.("Ctrl+Alt+Space"); });
  expect(changed).toHaveBeenCalledExactlyOnceWith("Ctrl+Alt+Space");
});
it("cancels on focus loss and cleans up native interception", async () => {
  await begin(); await act(async () => { window.dispatchEvent(new Event("blur")); });
  expect(changed).not.toHaveBeenCalled(); expect(native.set).toHaveBeenLastCalledWith(false);
});
it("shows a native recorder failure without saving", async () => {
  native.set.mockRejectedValueOnce(new Error("Finish recording first"));
  await begin(); expect(container.textContent).toContain("Finish recording first"); expect(changed).not.toHaveBeenCalled();
});
it("cancels a recorder that is left waiting and releases native interception", async () => {
  vi.useFakeTimers();
  try {
    await begin();
    await act(async () => { await vi.advanceTimersByTimeAsync(25000); });
    expect(changed).not.toHaveBeenCalled();
    expect(native.set).toHaveBeenLastCalledWith(false);
    expect(container.querySelector("button")?.getAttribute("aria-pressed")).toBe("false");
  } finally { vi.useRealTimers(); }
});
it("uses physical punctuation and letter codes, and accepts only extended bare function keys", () => {
  const event = { code: "Semicolon", ctrlKey: true, altKey: false, shiftKey: true, metaKey: false };
  expect(shortcutFromEvent(event)).toBe("Ctrl+Shift+Semicolon");
  expect(shortcutFromEvent({ ...event, code: "KeyK" })).toBe("Ctrl+Shift+K");
  for (let n = 13; n <= 24; n++) expect(shortcutFromEvent({ ...event, code: `F${n}`, ctrlKey: false, shiftKey: false })).toBe(`F${n}`);
});
