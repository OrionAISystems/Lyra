import { describe, expect, it } from "vitest";
import { isPrimaryPointerButton, primaryBarGesture } from "./barActivation";

describe("dictation bar activation", () => {
  it("uses press/release semantics only for push-to-talk", () => {
    expect(primaryBarGesture("push_to_talk")).toBe("pointer_press_release");
    expect(primaryBarGesture("toggle")).toBe("click_toggle");
  });

  it("ignores non-primary pointer buttons", () => {
    expect(isPrimaryPointerButton(0)).toBe(true);
    expect(isPrimaryPointerButton(1)).toBe(false);
    expect(isPrimaryPointerButton(2)).toBe(false);
  });
});
