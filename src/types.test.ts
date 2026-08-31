import { describe, expect, it } from "vitest";
import { defaultSettings } from "./types";

describe("Lyra defaults", () => {
  it("starts with the privacy-conscious local-first product defaults", () => {
    expect(defaultSettings.activationMode).toBe("push_to_talk");
    expect(defaultSettings.hotkey).toBe("Ctrl+Alt+Space");
    expect(defaultSettings.route).toBe("local_first_cloud_fallback");
    expect(defaultSettings.autoPaste).toBe(true);
    expect(defaultSettings.maxDurationSeconds).toBe(120);
  });
});
