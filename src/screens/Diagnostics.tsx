import { useState } from "react";
import {
  CheckCircle2,
  Clipboard,
  Download,
  HardDrive,
  RefreshCw,
  TriangleAlert,
} from "lucide-react";
import { checkForUpdates, installUpdate, retryHotkey } from "../bridge";
import { Card, Notice, Page, Row } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
export default function Diagnostics({ state }: { state: LyraState }) {
  const [retrying, setRetrying] = useState(false);
  const [retryMessage, setRetryMessage] = useState<string | null>(null);
  const report = () => {
    const safe = {
      version: state.update.currentVersion,
      runtime: state.runtime.runtimeVariant,
      models: {
        parakeet: state.runtime.modelVerified,
        nemotron: state.runtime.nemotronModelVerified,
      },
      hotkey: state.hotkey.registered,
      failures: state.usage.failureCounts,
      sessions: state.usage.recentSessions.slice(0, 20),
    };
    void navigator.clipboard.writeText(JSON.stringify(safe, null, 2));
  };
  const checking = state.update.state === "checking";
  const available = state.update.state === "available";
  const retry = async () => {
    setRetrying(true);
    setRetryMessage(null);
    try {
      const result = await retryHotkey();
      setRetryMessage(
        result?.registered ? "Verified" : (result?.error ?? "Retry failed"),
      );
      await state.refresh();
    } catch (error) {
      setRetryMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setRetrying(false);
    }
  };
  return (
    <Page
      title="Diagnostics"
      subtitle="Health, updates, local artifacts, and privacy-safe troubleshooting."
    >
      <div className="health">
        <Card>
          <CheckCircle2 />
          <strong>Core ready</strong>
          <span>Capture, runtime, and model verified</span>
        </Card>
        <Card>
          <HardDrive />
          <strong>
            {state.runtime.runtimeVariant?.toUpperCase() ?? "No runtime"}
          </strong>
          <span>NeMo-Speech.cpp {state.runtime.runtimeVersion ?? "—"}</span>
        </Card>
        <Card>
          <TriangleAlert />
          <strong>{state.usage.failedTranscriptions}</strong>
          <span>Lifetime failures</span>
        </Card>
      </div>
      <Card>
        <Row
          icon={<Download />}
          title={`Lyra ${state.update.currentVersion}`}
          description={
            available
              ? `Version ${state.update.availableVersion} is ready to install`
              : state.update.state === "up_to_date"
                ? "You’re up to date"
                : (state.update.error ??
                  "Check the public release channel for an update")
          }
        >
          {available ? (
            <button className="primary" onClick={() => void installUpdate()}>
              <Download size={14} /> Install update
            </button>
          ) : (
            <button
              className="secondary"
              disabled={checking}
              onClick={() => void checkForUpdates()}
            >
              <RefreshCw size={14} />{" "}
              {checking ? "Checking…" : "Check for updates"}
            </button>
          )}
        </Row>
        <Row
          title="Parakeet TDT"
          description={state.runtime.modelRevision ?? "Pinned revision"}
        >
          <span className={`badge ${state.runtime.modelVerified ? "ok" : ""}`}>
            {state.runtime.modelVerified ? "Verified" : "Missing"}
          </span>
        </Row>
        <Row
          title="Nemotron Streaming"
          description="Experimental English engine"
        >
          <span
            className={`badge ${state.runtime.nemotronModelVerified ? "ok" : ""}`}
          >
            {state.runtime.nemotronModelVerified ? "Verified" : "Optional"}
          </span>
        </Row>
        <Row
          title="Global shortcuts"
          description={state.hotkey.error ?? state.hotkey.hotkey}
        >
          {state.hotkey.registered || retryMessage === "Verified" ? (
            <span className="badge ok shortcut-verified">Verified</span>
          ) : (
            <button
              className="secondary"
              type="button"
              disabled={retrying}
              onClick={() => void retry()}
            >
              <RefreshCw size={14} /> {retrying ? "Retrying…" : "Retry"}
            </button>
          )}
        </Row>
        {retryMessage && retryMessage !== "Verified" && (
          <div
            className={`shortcut-feedback ${retryMessage === "Verified" ? "ok" : "inline-error"}`}
          >
            {retryMessage}
          </div>
        )}
      </Card>
      <h2 className="section-title">Recent metadata-only sessions</h2>
      <Card className="session-table">
        {state.usage.recentSessions.length === 0 ? (
          <div className="empty">
            Detailed session records begin with v0.4.0.
          </div>
        ) : (
          state.usage.recentSessions.slice(0, 8).map((x) => (
            <div key={x.timestampMs}>
              <span>{new Date(x.timestampMs).toLocaleString()}</span>
              <span>
                {x.targetExecutable ?? x.targetCategory ?? "Unknown app"}
              </span>
              <span>{x.engine.replaceAll("_", " ")}</span>
              <span className={`badge ${x.failureCategory ? "" : "ok"}`}>
                {x.failureCategory ?? x.pasteOutcome}
              </span>
            </div>
          ))
        )}
      </Card>
      <button className="secondary report" onClick={report}>
        <Clipboard size={15} /> Copy diagnostic report
      </button>
      <Notice>
        The report excludes transcript text, audio, clipboard contents, window
        titles, full paths, and credentials.
      </Notice>
    </Page>
  );
}
