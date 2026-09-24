import type { SessionRecord } from "./types";

interface DiagnosticSession {
  timestampMs: number;
  durationMs: number;
  wordCount: number;
  targetCategory: string | null;
  engine: string | null;
  runtime: string | null;
  backend: string | null;
  latencyMs: number | null;
  realTimeFactor: number | null;
  pasteOutcome: string;
  failureCategory: string | null;
}

const runtimeVariants = ["cpu", "cuda", "vulkan"] as const;
const engines = ["parakeettdt", "nemotronstreamingen"] as const;
const targetCategories = ["terminal", "developer", "chat", "editor", "other"] as const;
const backends = [
  "local_nemo",
  "openai_cloud",
  "local_nemotron_streaming",
  "local_nemotron_streaming_cpu",
  "local_nemotron_streaming_cuda",
  "local_nemotron_streaming_vulkan",
] as const;
const pasteOutcomes = ["success", "failed_or_clipboard", "copied", "not_attempted"] as const;
const failureCategories = [
  "capture",
  "no_speech",
  "clipping",
  "model_runtime",
  "recognition",
  "unsupported_language",
  "queue_overflow",
  "fallback",
  "target_recovery",
  "clipboard",
  "paste_injection",
  "unknown",
] as const;

function enumValue(value: string | null, allowed: readonly string[]): string | null {
  return value !== null && allowed.includes(value) ? value : null;
}

function safeFailureCounts(failures: Record<string, number>) {
  return Object.fromEntries(
    Object.entries(failures).filter(([category]) =>
      failureCategories.includes(category as (typeof failureCategories)[number]),
    ),
  );
}

export interface DiagnosticReportInput {
  version: string;
  runtime: string | null;
  models: { parakeet: boolean; nemotron: boolean };
  hotkey: boolean;
  failures: Record<string, number>;
  sessions: SessionRecord[];
}

export function buildDiagnosticReport(input: DiagnosticReportInput) {
  const sessions: DiagnosticSession[] = input.sessions
    .slice(0, 20)
    .map((session) => ({
      timestampMs: session.timestampMs,
      durationMs: session.durationMs,
      wordCount: session.wordCount,
      targetCategory: enumValue(session.targetCategory, targetCategories),
      engine: enumValue(session.engine, engines),
      runtime: enumValue(session.runtime, runtimeVariants),
      backend: enumValue(session.backend, backends),
      latencyMs: session.latencyMs,
      realTimeFactor: session.realTimeFactor,
      pasteOutcome: enumValue(session.pasteOutcome, pasteOutcomes) ?? "unknown",
      failureCategory: enumValue(session.failureCategory, failureCategories),
    }));

  return {
    version: /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(input.version)
      ? input.version
      : "unknown",
    runtime: enumValue(input.runtime, runtimeVariants),
    models: input.models,
    hotkey: input.hotkey,
    failures: safeFailureCounts(input.failures),
    sessions,
  };
}
