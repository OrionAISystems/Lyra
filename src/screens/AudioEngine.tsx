import { useEffect, useState } from "react";
import { Cpu, Download, Mic, ShieldCheck } from "lucide-react";
import {
  getBenchmarkResult,
  listenToBenchmarkEvents,
  runBenchmark,
  startSetupArtifact,
} from "../bridge";
import {
  Card,
  Notice,
  Page,
  Row,
  Select,
  Toggle,
} from "../components/Controls";
import type { LyraState } from "../state/useLyra";
export default function AudioEngine({ state }: { state: LyraState }) {
  const s = state.settings,
    u = state.updateSettings;
  const busy = ["downloading", "verifying", "installing"].includes(
    state.setup.state,
  );
  const [benchmarkState, setBenchmarkState] = useState("idle");
  const [benchmarkError, setBenchmarkError] = useState<string | null>(null);
  const [benchmarkResult, setBenchmarkResult] =
    useState<Awaited<ReturnType<typeof getBenchmarkResult>>>(null);
  useEffect(() => {
    let dead = false;
    void getBenchmarkResult().then((result) => {
      if (!dead && result) setBenchmarkResult(result);
    });
    let un: (() => void)[] = [];
    void listenToBenchmarkEvents({
      onState: (value) => setBenchmarkState(value),
      onResult: (value) => {
        setBenchmarkResult(value);
        setBenchmarkState("complete");
      },
    }).then((items) => {
      if (dead) items.forEach((item) => item());
      else un = items;
    });
    return () => {
      dead = true;
      un.forEach((item) => item());
    };
  }, []);
  const startBenchmark = async () => {
    setBenchmarkError(null);
    setBenchmarkState("starting");
    try {
      await runBenchmark();
      setBenchmarkState("recording");
    } catch (error) {
      setBenchmarkState("idle");
      setBenchmarkError(error instanceof Error ? error.message : String(error));
    }
  };
  return (
    <Page
      title="Audio & Engine"
      subtitle="Configure your microphone and local speech engine."
    >
      <Card>
        <Row
          icon={<Mic />}
          title="Microphone"
          description="Live input is held in memory only"
        >
          <Select
            label="Microphone"
            value={s.audioDeviceId ?? ""}
            onChange={(v) => void u({ audioDeviceId: v || null })}
          >
            <option value="">System default</option>
            {state.devices.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </Select>
        </Row>
        <div className="meter" aria-label="Microphone input level">
          <i
            style={{ width: `${Math.max(5, state.session.inputLevel * 100)}%` }}
          />
        </div>
        <Row
          icon={<ShieldCheck />}
          title="Adaptive audio"
          description="Detect noise floor, trim silence, and report clipping"
        >
          <Toggle
            label="Adaptive audio"
            checked={s.adaptiveAudioEnabled}
            onChange={(v) => void u({ adaptiveAudioEnabled: v })}
          />
        </Row>
      </Card>
      <Card>
        <Row
          icon={<Cpu />}
          title="Engine"
          description={
            s.localEngine === "parakeet_tdt"
              ? "Stable local default"
              : "Experimental English streaming with live partials"
          }
        >
          <Select
            label="Engine"
            value={s.localEngine}
            onChange={(v) =>
              void u({
                localEngine: v as typeof s.localEngine,
                language: v === "nemotron_streaming_en" ? "en" : s.language,
              })
            }
          >
            <option value="parakeet_tdt">Parakeet TDT</option>
            <option
              value="nemotron_streaming_en"
              disabled={!state.runtime.nemotronModelVerified}
            >
              Nemotron Streaming (experimental)
            </option>
          </Select>
        </Row>
        <Row
          icon={<Cpu />}
          title="Local runtime"
          description={`${state.runtime.runtimeVariant?.toUpperCase() ?? "Not installed"} · ${state.modelRuntime.state}`}
        >
          <span
            className={`badge ${state.runtime.runtimeVerified ? "ok" : ""}`}
          >
            {state.runtime.runtimeVerified ? "Ready" : "Setup required"}
          </span>
        </Row>
        <Row
          icon={<Download />}
          title="Nemotron Streaming 0.6B"
          description="Optional English q8 model · separately installed and verified"
        >
          {state.runtime.nemotronModelVerified ? (
            <span className="badge ok">Installed</span>
          ) : (
            <button
              className="secondary"
              disabled={busy}
              onClick={() => void startSetupArtifact("model-nemotron")}
            >
              {busy ? "Installing…" : "Install"}
            </button>
          )}
        </Row>
        <Row
          icon={<Cpu />}
          title="Benchmark engines"
          description="Read “Lyra turns a clear thought into useful text” once"
        >
          <button
            className="primary"
            type="button"
            disabled={
              benchmarkState === "recording" ||
              benchmarkState === "comparing" ||
              benchmarkState === "starting"
            }
            onClick={() => void startBenchmark()}
          >
            {benchmarkState === "recording" || benchmarkState === "comparing"
              ? "Benchmarking…"
              : "Run benchmark"}
          </button>
        </Row>
        {(benchmarkError || benchmarkResult) && (
          <div className="benchmark-status">
            {benchmarkError && (
              <span className="inline-error">{benchmarkError}</span>
            )}
            {benchmarkResult && !benchmarkError && (
              <span>
                Last run:{" "}
                {benchmarkResult.recommendation
                  ? `recommendation ${benchmarkResult.recommendation.replaceAll("_", " ")}`
                  : "no passing recommendation"}
                .
              </span>
            )}
          </div>
        )}
      </Card>
      <Notice>
        Audio and recognized text are never saved. Benchmark results contain
        scores and environment metadata only.
      </Notice>
    </Page>
  );
}
