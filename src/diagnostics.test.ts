import { describe, expect, it } from "vitest";
import { buildDiagnosticReport } from "./diagnostics";
import type { SessionRecord } from "./types";

describe("diagnostic report privacy", () => {
  it("exports only allowlisted session metadata", () => {
    const session: SessionRecord = {
      timestampMs: 1,
      appVersion: "0.4.4",
      durationMs: 100,
      wordCount: 10,
      targetExecutable: "C:\\Users\\Met\\private.exe",
      targetCategory: "editor",
      endpoint: "https://user:password@example.test",
      engine: "nemotronstreamingen",
      runtime: "cuda",
      backend: "local_nemotron_streaming_cuda",
      fallbackChain: ["C:\\Users\\Met\\AppData\\Local\\Lyra\\runtime\\nemo.dll"],
      latencyMs: 50,
      realTimeFactor: 0.5,
      pasteOutcome: "copied",
      failureCategory: null,
    };
    const report = buildDiagnosticReport({
      version: "0.4.4",
      runtime: "cuda",
      models: { parakeet: true, nemotron: false },
      hotkey: true,
      failures: { clipboard: 0, "C:\\Users\\Met\\private": 1 },
      sessions: [
        session,
        {
          ...session,
          appVersion: "C:\\Users\\Met\\private",
          targetCategory: "C:\\Users\\Met\\private",
          engine: "C:\\Users\\Met\\private",
          runtime: "C:\\Users\\Met\\private",
          backend: "C:\\Users\\Met\\private",
          pasteOutcome: "password@example.test",
          failureCategory: "password@example.test",
        },
      ],
    });
    const serialized = JSON.stringify(report);

    expect(serialized).not.toContain("Users");
    expect(serialized).not.toContain("password");
    expect(report.sessions[0]).not.toHaveProperty("fallbackChain");
    expect(report.sessions[0]).not.toHaveProperty("targetExecutable");
    expect(report.sessions[0]).not.toHaveProperty("endpoint");
    expect(report.sessions[0]).toMatchObject({
      targetCategory: "editor",
      engine: "nemotronstreamingen",
      backend: "local_nemotron_streaming_cuda",
    });
    expect(report.failures).toEqual({ clipboard: 0 });
    expect(report.sessions[1]).toMatchObject({
      targetCategory: null,
      engine: null,
      runtime: null,
      backend: null,
      pasteOutcome: "unknown",
      failureCategory: null,
    });
  });
});
