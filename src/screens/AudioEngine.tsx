import { Cpu, Download, Mic, ShieldCheck } from "lucide-react";
import { runBenchmark, startSetupArtifact } from "../bridge";
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
          <button className="primary" onClick={() => void runBenchmark()}>
            Run benchmark
          </button>
        </Row>
      </Card>
      <Notice>
        Audio and recognized text are never saved. Benchmark results contain
        scores and environment metadata only.
      </Notice>
    </Page>
  );
}
