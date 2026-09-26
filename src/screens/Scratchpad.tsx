import { useEffect, useState } from "react";
import { Copy, Mic, Pin, PinOff, Trash2 } from "lucide-react";
import {
  copySavedItem,
  deleteScratchpadItem,
  getScratchpadItem,
  listScratchpad,
  pasteSavedItem,
  setScratchpadPinned,
  startScratchpad,
  stopScratchpad,
} from "../bridge";
import { Card, Page } from "../components/Controls";
import { engineLabel } from "./History";
import type { LyraState } from "../state/useLyra";
import type { SavedItemDetail, SavedItemSummary } from "../types";

const PAGE_SIZE = 50;

export default function Scratchpad({ state }: { state: LyraState }) {
  const [items, setItems] = useState<SavedItemSummary[]>([]);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(state.scratchpadOpenId);
  const [detail, setDetail] = useState<SavedItemDetail | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const isScratchpadSession = state.session.destination === "scratchpad";
  const listening = isScratchpadSession && state.session.state === "listening";
  const processing = isScratchpadSession && state.session.state === "processing";
  const anotherSessionActive = ["listening", "processing"].includes(state.session.state) && !isScratchpadSession;
  const pushToTalk = state.settings.activationMode === "push_to_talk";

  useEffect(() => {
    if (state.scratchpadOpenId) setSelectedId(state.scratchpadOpenId);
  }, [state.scratchpadOpenId]);

  useEffect(() => {
    let current = true;
    void listScratchpad(0, PAGE_SIZE).then((loaded) => {
      if (!current) return;
      const page = loaded ?? { items: [], total: 0, offset: 0, hasMore: false };
      setItems(page.items);
      setTotal(page.total);
      setHasMore(page.hasMore);
    }).catch(() => current && setNotice("Scratchpad couldn't be loaded from local storage."));
    return () => {
      current = false;
    };
  }, [state.savedItemsRevision]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    let current = true;
    void getScratchpadItem(selectedId).then((loaded) => {
      if (current) setDetail(loaded);
    }).catch(() => {
      if (current) {
        setDetail(null);
        setNotice("This Scratchpad entry is unavailable or couldn't be decrypted.");
      }
    });
    return () => {
      current = false;
    };
  }, [selectedId, state.savedItemsRevision]);

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true);
    setNotice(null);
    try {
      await action();
      setNotice(success);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "That action couldn't be completed.");
    } finally {
      setBusy(false);
    }
  };
  const start = () => {
    setNotice(null);
    void startScratchpad().catch((error) => {
      setNotice(error instanceof Error ? error.message : "Scratchpad couldn't start recording.");
    });
  };
  const stop = () => {
    void stopScratchpad().catch((error) => {
      setNotice(error instanceof Error ? error.message : "Scratchpad couldn't stop recording.");
    });
  };
  const toggle = () => (listening ? stop() : start());
  const loadMore = async () => {
    setBusy(true);
    try {
      const page = await listScratchpad(items.length, PAGE_SIZE);
      if (!page) return;
      setItems((current) => {
        const seen = new Set(current.map((item) => item.id));
        return [...current, ...page.items.filter((item) => !seen.has(item.id))];
      });
      setTotal(page.total);
      setHasMore(page.hasMore);
    } catch {
      setNotice("More Scratchpad entries couldn't be loaded.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Page
      title="Scratchpad"
      subtitle="Speak a thought and keep it here until you need it."
      action={
        <button
          type="button"
          className={`primary scratchpad-record ${listening ? "recording" : ""}`}
          aria-pressed={listening}
          aria-label={listening ? "Stop Scratchpad recording" : "Record a Scratchpad thought"}
          disabled={anotherSessionActive || processing || (busy && !listening)}
          title={anotherSessionActive ? "Finish the active dictation first" : pushToTalk ? "Hold to record a thought" : "Start or stop a Scratchpad thought"}
          onClick={pushToTalk ? undefined : toggle}
          onPointerDown={pushToTalk ? (event) => {
            if (event.button !== 0 || anotherSessionActive || processing) return;
            event.preventDefault();
            event.currentTarget.setPointerCapture(event.pointerId);
            start();
          } : undefined}
          onPointerUp={pushToTalk ? stop : undefined}
          onPointerCancel={pushToTalk ? stop : undefined}
          onKeyDown={pushToTalk ? (event) => {
            if ((event.key === "Enter" || event.key === " ") && !event.repeat) {
              event.preventDefault();
              start();
            }
          } : undefined}
          onKeyUp={pushToTalk ? (event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              stop();
            }
          } : undefined}
        >
          <Mic size={15} /> {processing ? "Saving thought…" : listening ? "Listening… release to save" : pushToTalk ? "Hold to record" : "Record a thought"}
        </button>
      }
    >
      <div className="scratchpad-intro">
        <span className={`recording-indicator ${listening ? "active" : ""}`} />
        <span>{listening ? "Your thought is being captured locally." : processing ? "Transcribing your thought…" : "Only the thoughts you deliberately record are kept here."}</span>
        <small>Saved on this device · protected for your Windows account</small>
      </div>
      {(state.savedItemNotice || notice) && (
        <p className="saved-item-notice" role="status">{notice ?? state.savedItemNotice}</p>
      )}

      {detail && (
        <Card className="saved-detail scratchpad-detail">
          <div className="saved-detail-head">
            <div>
              <strong>{formatDateTime(detail.summary.timestampMs)}</strong>
              <small>{engineLabel(detail.summary.engine)} · {detail.summary.wordCount} words · {formatDuration(detail.summary.durationMs)}</small>
            </div>
            <div className="saved-actions">
              <button type="button" className="secondary" disabled={busy} onClick={() => void run(() => copySavedItem("scratchpad", detail.summary.id), "Copied to clipboard.")}>
                <Copy size={14} /> Copy
              </button>
              <button type="button" className="secondary" disabled={busy} onClick={() => void run(() => pasteSavedItem("scratchpad", detail.summary.id), "Pasted into the current app.")}>
                Paste into current app
              </button>
              <button type="button" className="icon-button" aria-label={detail.summary.pinned ? "Unpin Scratchpad item" : "Pin Scratchpad item"} title={detail.summary.pinned ? "Unpin" : "Pin"} disabled={busy} onClick={() => void run(() => setScratchpadPinned(detail.summary.id, !detail.summary.pinned), detail.summary.pinned ? "Unpinned." : "Pinned at the top.")}>
                {detail.summary.pinned ? <PinOff size={15} /> : <Pin size={15} />}
              </button>
              <button type="button" className="icon-button danger" aria-label="Delete Scratchpad item" title="Delete" disabled={busy} onClick={() => {
                if (window.confirm("Delete this Scratchpad entry?")) {
                  void run(() => deleteScratchpadItem(detail.summary.id), "Scratchpad entry deleted.").then(() => setSelectedId(null));
                }
              }}>
                <Trash2 size={15} />
              </button>
            </div>
          </div>
          <pre className="saved-transcript">{detail.text}</pre>
          <div className="saved-metadata">
            <span>{detail.summary.latencyMs.toLocaleString()} ms latency</span>
            <span>{detail.summary.model}</span>
          </div>
        </Card>
      )}

      {total === 0 ? (
        <Card className="saved-empty">
          <Mic size={22} />
          <strong>Your Scratchpad is empty</strong>
          <p>Hold the shortcut or record button, speak a thought, and release. Lyra won't paste it anywhere.</p>
          <small>Shortcut: {state.settings.scratchpadHotkey}</small>
        </Card>
      ) : (
        <section className="scratchpad-list-section">
          <div className="scratchpad-list-heading">
            <h2>Your thoughts</h2>
            <small>{total} saved</small>
          </div>
          <Card className="scratchpad-list">
            {items.map((item) => (
              <button
                type="button"
                className={`scratchpad-entry ${selectedId === item.id ? "selected" : ""}`}
                key={item.id}
                aria-pressed={selectedId === item.id}
                onClick={() => setSelectedId(item.id)}
              >
                <span className="scratchpad-entry-pin" aria-hidden="true">{item.pinned && <Pin size={13} />}</span>
                <span className="scratchpad-entry-main">
                  <small>{formatDateTime(item.timestampMs)}</small>
                  <strong>{item.wordCount} words · {formatDuration(item.durationMs)} · {engineLabel(item.engine)}</strong>
                </span>
              </button>
            ))}
          </Card>
          {hasMore && (
            <button type="button" className="secondary history-load-more" disabled={busy} onClick={() => void loadMore()}>
              {busy ? "Loading…" : `Load older entries · ${Math.max(0, total - items.length)} remaining`}
            </button>
          )}
        </section>
      )}
    </Page>
  );
}

function formatDateTime(timestampMs: number) {
  const date = new Date(timestampMs);
  const today = new Date();
  const sameDay = date.getFullYear() === today.getFullYear() && date.getMonth() === today.getMonth() && date.getDate() === today.getDate();
  return `${sameDay ? "Today" : date.toLocaleDateString(undefined, { month: "short", day: "numeric" })}, ${date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
}

function formatDuration(durationMs: number) {
  return `${Math.max(1, Math.round(durationMs / 1_000))} sec`;
}
