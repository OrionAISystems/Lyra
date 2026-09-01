import type { ActivationMode } from "./types";

export type PrimaryBarGesture = "pointer_press_release" | "click_toggle";

export function primaryBarGesture(mode: ActivationMode): PrimaryBarGesture {
  return mode === "push_to_talk" ? "pointer_press_release" : "click_toggle";
}

export function isPrimaryPointerButton(button: number): boolean {
  return button === 0;
}
