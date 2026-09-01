import { describe, expect, it } from "vitest";
import { defaultSettings } from "./types";

describe("Lyra defaults", () => {
  it("starts with the privacy-conscious local-first product defaults", () => {
    expect(defaultSettings.activationMode).toBe("push_to_talk");
    expect(defaultSettings.hotkey).toBe("Ctrl+Alt+Space");
    expect(defaultSettings.route).toBe("local_first_cloud_fallback");
    expect(defaultSettings.autoPaste).toBe(true);
    expect(defaultSettings.maxDurationSeconds).toBe(120);
    expect(defaultSettings.schemaVersion).toBe(2);
    expect(defaultSettings.autoStopAfterSilenceMs).toBeNull();
    expect(defaultSettings.formattingCommandsEnabled).toBe(true);
    expect(defaultSettings.corrections).toEqual([
      { source: "orion ai systems", replacement: "Orion AI Systems", enabled: true, caseSensitive: false },
      { source: "geo core", replacement: "GeoCore", enabled: true, caseSensitive: false },
      { source: "war map", replacement: "WarMap", enabled: true, caseSensitive: false },
    ]);
    expect(defaultSettings.barPlacement).toBe("follow_active_window");
    expect(defaultSettings.modelWarmPolicy).toBe("always_warm");
    expect(defaultSettings.usageCostEstimatesEnabled).toBe(false);
    expect(defaultSettings.cloudCostPerMinuteUsd).toBe(0.003);
  });
});
