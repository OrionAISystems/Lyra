import { useEffect, useRef, useState } from "react";
import { Check, Download, X } from "lucide-react";
import type { SetupArtifactKind, SetupProgress } from "../types";

const COMPLETION_NOTICE_MS = 2_800;

function isModelKind(kind: SetupProgress["kind"]): boolean {
  return kind === "model-parakeet" || kind === "model-nemotron";
}

function modelName(kind: SetupArtifactKind | null): string {
  return kind === "model-nemotron" ? "Nemotron Streaming" : "Parakeet TDT";
}

function formatBytes(bytes: number): string {
  const value = Math.max(0, bytes);
  if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(1)} GiB`;
  const mebibytes = value / 1024 ** 2;
  return `${mebibytes >= 100 ? Math.round(mebibytes) : mebibytes.toFixed(1)} MiB`;
}

function phaseLabel(state: SetupProgress["state"]): string {
  switch (state) {
    case "downloading":
      return "Downloading";
    case "verifying":
      return "Verifying";
    case "installing":
      return "Installing";
    case "error":
      return "Couldn't install";
    default:
      return "Preparing";
  }
}

export function ModelSetupProgress({ setup }: { setup: SetupProgress }) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const wasActive = useRef(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [showComplete, setShowComplete] = useState(false);
  const [errorDismissed, setErrorDismissed] = useState(false);

  const modelSetup = isModelKind(setup.kind);
  const active =
    modelSetup &&
    ["downloading", "verifying", "installing"].includes(setup.state);
  const complete = modelSetup && setup.state === "complete";
  const failed = modelSetup && setup.state === "error";
  const name = modelName(setup.kind);
  const progress =
    setup.state === "downloading" && setup.totalBytes > 0
      ? Math.round(
          Math.max(0, Math.min(1, setup.downloadedBytes / setup.totalBytes)) *
            100,
        )
      : null;

  useEffect(() => {
    if (active) {
      if (!wasActive.current) {
        setDialogOpen(true);
        setErrorDismissed(false);
      }
      wasActive.current = true;
      return;
    }

    wasActive.current = false;
    if (!failed) setDialogOpen(false);
  }, [active, failed]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (dialogOpen && !dialog.open) dialog.showModal();
    else if (!dialogOpen && dialog.open) dialog.close();
  }, [dialogOpen]);

  useEffect(() => {
    if (!complete) {
      setShowComplete(false);
      return;
    }
    setShowComplete(true);
    const timer = window.setTimeout(
      () => setShowComplete(false),
      COMPLETION_NOTICE_MS,
    );
    return () => window.clearTimeout(timer);
  }, [complete, setup.kind]);

  if (!modelSetup) return null;

  const downloading = setup.state === "downloading";
  const phase = phaseLabel(setup.state);
  const downloaded = Math.min(setup.downloadedBytes, setup.totalBytes);
  const progressDescription =
    progress === null
      ? `${phase} ${name} model`
      : `${formatBytes(downloaded)} of ${formatBytes(setup.totalBytes)} · ${progress}%`;

  return (
    <>
      <dialog
        ref={dialogRef}
        className="model-setup-dialog"
        aria-labelledby="model-setup-title"
        onCancel={(event) => {
          event.preventDefault();
          setDialogOpen(false);
        }}
        onClose={() => setDialogOpen(false)}
        onClick={(event) => {
          if (event.target === event.currentTarget) setDialogOpen(false);
        }}
      >
        <header className="model-setup-dialog__header">
          <span className="model-setup-dialog__icon" aria-hidden="true">
            {failed ? <X size={17} /> : <Download size={17} />}
          </span>
          <div>
            <h2 id="model-setup-title">
              {failed ? `Couldn't install ${name}` : `${phase} ${name}`}
            </h2>
            <p>
              {failed
                ? setup.error ?? "Lyra couldn't finish installing this model."
                : downloading
                  ? "The download continues if you close this window."
                  : setup.state === "verifying"
                    ? "Checking the downloaded file against its pinned checksum."
                    : "Saving the verified model to Lyra's local model store."}
            </p>
          </div>
          <button
            className="model-setup-dialog__close"
            type="button"
            aria-label="Continue setup in background"
            title="Continue in background"
            onClick={() => setDialogOpen(false)}
          >
            <X size={16} />
          </button>
        </header>
        {!failed && (
          <>
            <div className="model-setup-dialog__progress-copy">
              <span>{downloading ? "Download progress" : phase}</span>
              {progress !== null && <span>{progress}%</span>}
            </div>
            <div
              className={`model-setup-track${progress === null ? " is-indeterminate" : ""}`}
              role="progressbar"
              aria-label={`${phase} ${name} model`}
              aria-valuemin={progress === null ? undefined : 0}
              aria-valuemax={progress === null ? undefined : 100}
              aria-valuenow={progress ?? undefined}
            >
              <span
                style={progress === null ? undefined : { width: `${progress}%` }}
              />
            </div>
            {progress !== null && (
              <p className="model-setup-dialog__size">{progressDescription}</p>
            )}
          </>
        )}
        <footer>
          <button
            className="secondary"
            type="button"
            onClick={() => setDialogOpen(false)}
          >
            {failed ? "Close" : "Continue in background"}
          </button>
        </footer>
      </dialog>

      {active && !dialogOpen && (
        <section className="model-setup-card" aria-label={`${name} download progress`}>
          <button
            className="model-setup-card__open"
            type="button"
            aria-label={`Show ${name} download details, ${progressDescription}`}
            onClick={() => setDialogOpen(true)}
          >
            <span className="model-setup-card__copy">
              <span>{phase} {name}</span>
              {progress !== null && <span>{progress}%</span>}
            </span>
            <span
              className={`model-setup-track model-setup-track--compact${progress === null ? " is-indeterminate" : ""}`}
              aria-hidden="true"
            >
              <span
                style={progress === null ? undefined : { width: `${progress}%` }}
              />
            </span>
          </button>
        </section>
      )}

      {failed && !dialogOpen && !errorDismissed && (
        <section className="model-setup-card model-setup-card--error" role="alert">
          <button
            className="model-setup-card__open"
            type="button"
            onClick={() => setDialogOpen(true)}
          >
            <span className="model-setup-card__copy">
              <span>{phase} {name}</span>
              <span>View details</span>
            </span>
            <span className="model-setup-card__error">
              {setup.error ?? "Lyra couldn't finish installing this model."}
            </span>
          </button>
          <button
            className="model-setup-card__dismiss"
            type="button"
            aria-label="Dismiss model installation error"
            onClick={() => setErrorDismissed(true)}
          >
            <X size={14} />
          </button>
        </section>
      )}

      {showComplete && (
        <section className="model-setup-card model-setup-card--success" role="status">
          <Check size={15} />
          <span>{name} model installed and verified</span>
        </section>
      )}
    </>
  );
}
