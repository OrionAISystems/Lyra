import { useEffect, useState } from "react";
import type { UpdateStatus } from "../types";

const UPDATE_APPEAR_DELAY_MS = 350;

export function UpdateProgress({ update }: { update: UpdateStatus }) {
  const active = update.state === "downloading" || update.state === "installing";
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (!active) {
      setVisible(false);
      return;
    }

    const timer = window.setTimeout(
      () => setVisible(true),
      UPDATE_APPEAR_DELAY_MS,
    );
    return () => window.clearTimeout(timer);
  }, [active]);

  if (!active || !visible) return null;

  const downloading = update.state === "downloading";
  const progress =
    downloading && update.progress !== null
      ? Math.round(Math.max(0, Math.min(1, update.progress)) * 100)
      : null;

  return (
    <section className="update-progress" role="status" aria-live="polite">
      <div className="update-progress__heading">
        <div>
          <strong>
            {downloading ? "Downloading Lyra update" : "Finishing update"}
          </strong>
          <span>
            {downloading
              ? `Version ${update.availableVersion ?? "update"}`
              : "Lyra will restart to finish installing."}
          </span>
        </div>
        {progress !== null && (
          <span className="update-progress__percent">{progress}%</span>
        )}
      </div>
      <div
        className={`update-progress__track${progress === null ? " is-indeterminate" : ""}`}
        role="progressbar"
        aria-label={downloading ? "Downloading Lyra update" : "Installing Lyra update"}
        aria-valuemin={progress === null ? undefined : 0}
        aria-valuemax={progress === null ? undefined : 100}
        aria-valuenow={progress ?? undefined}
      >
        <span style={progress === null ? undefined : { width: `${progress}%` }} />
      </div>
    </section>
  );
}
