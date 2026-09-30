import { useEffect, useState } from "react";
import { Cpu, Download, Mic, ShieldCheck, Square } from "lucide-react";
import {
  getBenchmarkResult,
  isTauri,
  listenToMicrophoneTestEvents,
  listenToBenchmarkEvents,
  runBenchmark,
  startMicrophoneTest,
  stopMicrophoneTest,
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
  const [setupStartError, setSetupStartError] = useState<string | null>(null);
  const [benchmarkState, setBenchmarkState] = useState("idle");
  const [benchmarkError, setBenchmarkError] = useState<string | null>(null);
  const [benchmarkResult, setBenchmarkResult] =
    useState<Awaited<ReturnType<typeof getBenchmarkResult>>>(null);
  const [microphoneTestState, setMicrophoneTestState] = useState<
    "idle" | "starting" | "listening" | "error"
  >("idle");
  const [microphoneTestLevel, setMicrophoneTestLevel] = useState(0);
  const [microphoneTestError, setMicrophoneTestError] = useState<string | null>(
    null,
  );
  const sessionListening = state.session.state === "listening";
  const sessionBusy = sessionListening || state.session.state === "processing";
  const benchmarkBusy = ["starting", "recording", "comparing"].includes(
    benchmarkState,
  );
  const inputLevel = Math.max(
    0,
    Math.min(1, sessionListening ? state.session.inputLevel : microphoneTestLevel),
  );
  const meterPercent =
    inputLevel <= 0.001
      ? 0
      : Math.max(0, Math.min(100, ((20 * Math.log10(inputLevel) + 60) / 60) * 100));

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
  useEffect(() => {
    let dead = false;
    let un: (() => void)[] = [];
    void listenToMicrophoneTestEvents({
      onLevel: (level) => setMicrophoneTestLevel(Math.max(0, Math.min(1, level))),
      onStopped: () => {
        setMicrophoneTestLevel(0);
        setMicrophoneTestState("idle");
      },
      onError: (message) => {
        setMicrophoneTestLevel(0);
        setMicrophoneTestState("error");
        setMicrophoneTestError(message);
        void stopMicrophoneTest().catch((error) => {
          setMicrophoneTestError(
            `${message}; ${error instanceof Error ? error.message : String(error)}`,
          );
        });
      },
    }).then((items) => {
      if (dead) items.forEach((item) => item());
      else un = items;
    });
    return () => {
      dead = true;
      un.forEach((item) => item());
      void stopMicrophoneTest().catch(() => {});
    };
  }, []);

  const toggleMicrophoneTest = async () => {
    if (microphoneTestState === "listening") {
      try {
        await stopMicrophoneTest();
        setMicrophoneTestState("idle");
        setMicrophoneTestLevel(0);
        setMicrophoneTestError(null);
      } catch (error) {
        setMicrophoneTestState("error");
        setMicrophoneTestError(
          error instanceof Error ? error.message : String(error),
        );
      }
      return;
    }
    setMicrophoneTestError(null);
    setMicrophoneTestLevel(0);
    setMicrophoneTestState("starting");
    try {
      await startMicrophoneTest();
      setMicrophoneTestState("listening");
    } catch (error) {
      setMicrophoneTestState("error");
      setMicrophoneTestError(
        error instanceof Error ? error.message : String(error),
      );
    }
  };

  const selectMicrophone = async (deviceId: string) => {
    try {
      await stopMicrophoneTest();
      setMicrophoneTestState("idle");
      setMicrophoneTestLevel(0);
      setMicrophoneTestError(null);
      await u({ audioDeviceId: deviceId || null });
    } catch (error) {
      setMicrophoneTestState("error");
      setMicrophoneTestError(
        error instanceof Error ? error.message : String(error),
      );
    }
  };

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
  const startModelSetup = async (kind: "model-parakeet" | "model-nemotron") => {
    setSetupStartError(null);
    try {
      await startSetupArtifact(kind);
    } catch (error) {
      setSetupStartError(
        error instanceof Error ? error.message : "Lyra couldn't start the model download.",
      );
    }
  };
  return (
    <Page
      title="Audio & Engine"
      subtitle="Configure your microphone and local speech engine."
    >
      <Card>
        <Row
          className="microphone-row"
          icon={<Mic />}
          title="Microphone"
          description="Test your selected input without starting dictation"
        >
          <div className="microphone-controls">
            <Select
              label="Microphone"
              value={s.audioDeviceId ?? ""}
              disabled={microphoneTestState === "starting"}
              onChange={(v) => void selectMicrophone(v)}
            >
              <option value="">System default</option>
              {state.devices.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </Select>
            <button
              className={microphoneTestState === "listening" ? "secondary" : "primary"}
              type="button"
              disabled={
                !isTauri ||
                sessionBusy ||
                benchmarkBusy ||
                microphoneTestState === "starting"
              }
              onClick={() => void toggleMicrophoneTest()}
              aria-pressed={microphoneTestState === "listening"}
            >
              {microphoneTestState === "starting" ? (
                "Starting…"
              ) : microphoneTestState === "listening" ? (
                <><Square size={12} fill="currentColor" /> Stop test</>
              ) : (
                "Test mic"
              )}
            </button>
          </div>
        </Row>
        <div
          className="meter"
          role="meter"
          aria-label="Microphone input level"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(meterPercent)}
          aria-valuetext={
            inputLevel > 0 ? `${Math.round(20 * Math.log10(inputLevel))} dBFS` : "No input"
          }
          data-active={microphoneTestState === "listening" || sessionListening}
        >
          <i style={{ width: `${meterPercent}%` }} />
        </div>
        <div
          className={`microphone-test-status ${microphoneTestError ? "error" : ""}`}
          role="status"
          aria-live="polite"
        >
          {microphoneTestError ? (
            <span>{microphoneTestError}</span>
          ) : !isTauri ? (
            <span>Open Lyra desktop to test a microphone.</span>
          ) : sessionListening ? (
            <span>Dictation is using the microphone.</span>
          ) : microphoneTestState === "starting" ? (
            <span>Opening microphone…</span>
          ) : microphoneTestState === "listening" && inputLevel >= 0.012 ? (
            <span className="detected">Input detected</span>
          ) : microphoneTestState === "listening" ? (
            <span>Listening — speak or make a sound.</span>
          ) : (
            <span>Microphone test is off.</span>
          )}
          {microphoneTestState === "listening" && inputLevel > 0 && (
            <span>{Math.round(20 * Math.log10(inputLevel))} dBFS</span>
          )}
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
      <Card className="audio-engine-detail-card">
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
          title="Parakeet TDT 0.6B"
          description="Required local recognition model · separately downloaded and verified"
        >
          {state.runtime.modelVerified ? (
            <span className="badge ok">Installed</span>
          ) : (
            <button
              className="secondary"
              disabled={!isTauri || busy}
              onClick={() => void startModelSetup("model-parakeet")}
            >
              {state.setup.kind === "model-parakeet" && busy
                ? `${state.setup.state[0]?.toUpperCase()}${state.setup.state.slice(1)}…`
                : "Install"}
            </button>
          )}
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
              disabled={!isTauri || busy}
              onClick={() => void startModelSetup("model-nemotron")}
            >
              {state.setup.kind === "model-nemotron" && busy
                ? `${state.setup.state[0]?.toUpperCase()}${state.setup.state.slice(1)}…`
                : "Install"}
            </button>
          )}
        </Row>
        {setupStartError && (
          <p className="inline-error" role="alert">
            {setupStartError}
          </p>
        )}
        <Row
          icon={<Cpu />}
          title="Benchmark engines"
          description="Read “Lyra turns a clear thought into useful text” once"
        >
          <button
            className="primary"
            type="button"
            disabled={
              benchmarkBusy ||
              microphoneTestState === "listening" ||
              microphoneTestState === "starting"
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
        The microphone test only measures a live input level; Lyra does not
        retain, save, or transcribe its audio. Benchmark results contain scores
        and environment metadata only.
      </Notice>
    </Page>
  );
}
