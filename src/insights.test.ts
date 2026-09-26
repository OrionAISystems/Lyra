import { describe, expect, it } from "vitest";
import { formatUsageDay } from "./insights";

describe("formatUsageDay", () => {
  it("formats persisted ISO days without exposing the leading separator", () => {
    expect(formatUsageDay("2026-09-22")).toMatch(/Sep.*22|22.*Sep/i);
    expect(formatUsageDay("2026-09-22")).not.toBe("-22");
  });

  it("preserves short fixture labels", () => {
    expect(formatUsageDay("Mon")).toBe("Mon");
  });

  it("formats monthly and yearly aggregate labels", () => {
    expect(formatUsageDay("2026-09")).toMatch(/Sep.*26|26.*Sep/i);
    expect(formatUsageDay("2026")).toBe("2026");
  });
});
