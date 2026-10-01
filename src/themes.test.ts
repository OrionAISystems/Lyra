import { describe, expect, it } from "vitest";
import { bundledOctober, bundledThemeStatus, paletteStyle, withinSeason } from "./themes";

describe("seasonal theme calendar", () => {
  it("treats the configured October window as start-inclusive and end-exclusive", () => {
    expect(withinSeason(new Date(2026, 8, 30), "10-01", "11-01")).toBe(false);
    expect(withinSeason(new Date(2026, 9, 1), "10-01", "11-01")).toBe(true);
    expect(withinSeason(new Date(2026, 9, 31), "10-01", "11-01")).toBe(true);
    expect(withinSeason(new Date(2026, 10, 1), "10-01", "11-01")).toBe(false);
  });

  it("handles an annual window that crosses the year boundary", () => {
    expect(withinSeason(new Date(2026, 11, 15), "12-01", "02-01")).toBe(true);
    expect(withinSeason(new Date(2027, 0, 31), "12-01", "02-01")).toBe(true);
    expect(withinSeason(new Date(2027, 1, 1), "12-01", "02-01")).toBe(false);
    expect(withinSeason(new Date(2027, 5, 1), "12-01", "02-01")).toBe(false);
  });

  it("exposes the bundled theme only while enabled and in season", () => {
    const inSeason = new Date(2026, 9, 15);
    const active = bundledThemeStatus(true, false, inSeason);
    expect(active).toMatchObject({ enabled: true, motionEnabled: false, state: "fallback", themeId: "october", source: "bundled" });
    expect(active.content).toEqual(bundledOctober);

    const disabled = bundledThemeStatus(false, true, inSeason);
    expect(disabled).toMatchObject({ enabled: false, state: "disabled", themeId: null, source: null, content: null });

    const upcoming = bundledThemeStatus(true, true, new Date(2026, 8, 30));
    expect(upcoming).toMatchObject({ state: "upcoming", themeId: null, source: null, content: null });
  });

  it("maps palette tokens to CSS variables and derives the glow color", () => {
    const style = paletteStyle(bundledOctober.palettes.light);
    expect(style["--accent"]).toBe(bundledOctober.palettes.light.accent);
    expect(style["--bg"]).toBe(bundledOctober.palettes.light.bg);
    expect(style["--glow"]).toBe("rgba(223, 147, 78, 0.24)");
  });
});
