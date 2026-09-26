import { useEffect, useMemo, useState } from "react";
import { Archive, Copy, Trash2 } from "lucide-react";
import {
  clearHistory,
  copySavedItem,
  deleteHistoryItem,
  getHistoryItem,
  listHistory,
  pasteSavedItem,
  promoteHistoryItem,
} from "../bridge";
import { Card, Page, Select } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
import type { SavedItemDetail, SavedItemSummary, TranscriptHistoryRetention } from "../types";

const PAGE_SIZE = 50;

const retentionLabels: Record<TranscriptHistoryRetention, string> = {
  off: "Off",
  seven_days: "7 days",
  thirty_days: "30 days",
  ninety_days: "90 days",
  one_year: "1 year",
  forever: "Forever",
};

export default function History({ state }: { state: LyraState }) {
  const [items, setItems] = useState<SavedItemSummary[]>([]);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<SavedItemDetail | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void listHistory(0, PAGE_SIZE).then((loaded) => {
      if (!current) return;
      const page = loaded ?? { items: [], total: 0, offset: 0, hasMore: false };
      setItems(page.items);
      setTotal(page.total);
      setHasMore(page.hasMore);
    }).catch(() => current && setNotice("History couldn't be loaded from local storage."));
    return () => {
      current = false;
    };
  }, [state.savedItemsRevision, state.settings.transcriptHistoryRetention, state.settings.historyLastRetention]);

  const loadMore = async () => {
    setBusy(true);
    try {
      const page = await listHistory(items.length, PAGE_SIZE);
      if (!page) return;
      setItems((current) => {
        const seen = new Set(current.map((item) => item.id));
        return [...current, ...page.items.filter((item) => !seen.has(item.id))];
      });
      setTotal(page.total);
      setHasMore(page.hasMore);
    } catch {
      setNotice("More History entries couldn't be loaded.");
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    let current = true;
    void getHistoryItem(selectedId).then((loaded) => {
      if (current) setDetail(loaded);
    }).catch(() => {
      if (current) {
        setDetail(null);
        setNotice("This History item is unavailable or its encrypted text couldn't be opened.");
      }
    });
    return () => {
      current = false;
    };
  }, [selectedId, state.savedItemsRevision]);

  const groups = useMemo(() => groupByLocalDay(items), [items]);
  const retention = state.settings.transcriptHistoryRetention;
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

  return (
    <Page
      title="History"
      subtitle="Your past dictations, stored locally when you choose to keep them."
      action={
        <button
          type="button"
          className="danger-quiet"
          disabled={total === 0 || busy}
          onClick={() => {
            if (window.confirm("Clear all saved transcript History? This won't remove Scratchpad entries or Insights.")) {
              void run(clearHistory, "Transcript History cleared.").then(() => {
                setItems([]);
                setTotal(0);
                setHasMore(false);
                setSelectedId(null);
              });
            }
          }}
        >
          <Trash2 size={14} /> Clear history
        </button>
      }
    >
      <Card className="history-retention">
        <div className="history-retention-copy">
          <strong>Transcript history</strong>
          <small>
            Saved text stays on this device and is protected for your Windows account. Audio is never saved.
          </small>
        </div>
        <Select
          label="Transcript history retention"
          value={retention}
          onChange={(value) => {
            void state.updateSettings({
              transcriptHistoryRetention: value as TranscriptHistoryRetention,
            });
          }}
        >
          {(Object.keys(retentionLabels) as TranscriptHistoryRetention[]).map((value) => (
            <option value={value} key={value}>{retentionLabels[value]}</option>
          ))}
        </Select>
      </Card>
      {retention === "off" && (
        <div className="privacy-note" role="status">
          <Archive size={16} />
          <span>History is off. New dictations won't be saved. Existing entries remain until their previous retention period expires.</span>
        </div>
      )}
      {(state.savedItemNotice || notice) && (
        <p className="saved-item-notice" role="status">{notice ?? state.savedItemNotice}</p>
      )}

      {detail && (
        <Card className="saved-detail">
          <div className="saved-detail-head">
            <div>
              <strong>{formatDateTime(detail.summary.timestampMs)}</strong>
              <small>{detail.summary.targetApp ?? "Unknown app"} · {engineLabel(detail.summary.engine)}</small>
            </div>
            <div className="saved-actions">
              <button type="button" className="secondary" disabled={busy} onClick={() => void run(() => copySavedItem("history", detail.summary.id), "Copied to clipboard.")}>
                <Copy size={14} /> Copy
              </button>
              <button type="button" className="secondary" disabled={busy} onClick={() => void run(() => pasteSavedItem("history", detail.summary.id), "Pasted into the current app.")}>
                Paste again
              </button>
              <button type="button" className="secondary" disabled={busy} onClick={() => void run(() => promoteHistoryItem(detail.summary.id), "Kept in Scratchpad.")}>
                Keep this → Scratchpad
              </button>
              <button type="button" className="icon-button danger" aria-label="Delete History item" title="Delete" disabled={busy} onClick={() => {
                if (window.confirm("Delete this saved transcript?")) {
                  void run(() => deleteHistoryItem(detail.summary.id), "Transcript deleted.").then(() => {
                    setSelectedId(null);
                  });
                }
              }}>
                <Trash2 size={15} />
              </button>
            </div>
          </div>
          <pre className="saved-transcript">{detail.text}</pre>
          <div className="saved-metadata">
            <span>{detail.summary.wordCount} words</span>
            <span>{formatDuration(detail.summary.durationMs)}</span>
            <span>{detail.summary.latencyMs.toLocaleString()} ms latency</span>
            <span>{pasteOutcomeLabel(detail.summary.pasteOutcome)}</span>
          </div>
        </Card>
      )}

      {total === 0 ? (
        <Card className="saved-empty">
          <Archive size={22} />
          <strong>{retention === "off" ? "Nothing is being saved" : "No saved dictations yet"}</strong>
          <p>{retention === "off" ? "Choose a retention period above to keep future dictations here." : "Completed dictations will appear here until their retention period ends."}</p>
        </Card>
      ) : (
        <>
          <div className="history-groups">
            {groups.map(([day, dayItems]) => (
              <section className="history-day" key={day}>
                <h2>{day}</h2>
                <Card className="history-list">
                  {dayItems.map((item) => (
                    <button
                      type="button"
                      className={`history-entry ${selectedId === item.id ? "selected" : ""}`}
                      key={item.id}
                      aria-pressed={selectedId === item.id}
                      onClick={() => setSelectedId(item.id)}
                    >
                      <span className="history-entry-time">{formatTime(item.timestampMs)}</span>
                      <span className="history-entry-main">
                        <strong>{item.targetApp ?? "Unknown app"}</strong>
                        <small>{item.wordCount} words · {formatDuration(item.durationMs)} · {engineLabel(item.engine)}</small>
                      </span>
                      <span className="history-entry-engine">{engineLabel(item.engine)}</span>
                    </button>
                  ))}
                </Card>
              </section>
            ))}
          </div>
          {hasMore && (
            <button type="button" className="secondary history-load-more" disabled={busy} onClick={() => void loadMore()}>
              {busy ? "Loading…" : `Load older entries · ${Math.max(0, total - items.length)} remaining`}
            </button>
          )}
        </>
      )}
    </Page>
  );
}

function groupByLocalDay(items: SavedItemSummary[]): [string, SavedItemSummary[]][] {
  const groups = new Map<string, SavedItemSummary[]>();
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  for (const item of items) {
    const date = new Date(item.timestampMs);
    const dateStart = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    const day = dateStart === today
      ? "Today"
      : dateStart === today - 86_400_000
        ? "Yesterday"
        : date.toLocaleDateString(undefined, {
            weekday: "long", month: "long", day: "numeric",
            ...(date.getFullYear() === now.getFullYear() ? {} : { year: "numeric" }),
          });
    groups.set(day, [...(groups.get(day) ?? []), item]);
  }
  return [...groups.entries()];
}

function formatDateTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleString(undefined, {
    dateStyle: "medium", timeStyle: "short",
  });
}

function formatTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleTimeString(undefined, {
    hour: "numeric", minute: "2-digit",
  });
}

function formatDuration(durationMs: number) {
  return `${Math.max(1, Math.round(durationMs / 1_000))} sec`;
}

export function engineLabel(engine: string) {
  const lower = engine.toLowerCase();
  if (lower.includes("nemotron")) return "Nemotron";
  if (lower.includes("parakeet")) return "Parakeet";
  if (lower.includes("openai") || lower.includes("cloud")) return "OpenAI";
  return engine.replaceAll("_", " ");
}

function pasteOutcomeLabel(outcome: string) {
  switch (outcome) {
    case "success": return "Paste succeeded";
    case "failed_or_clipboard": return "Paste failed · copied for recovery";
    case "copied": return "Copied for manual paste";
    case "copy_failed": return "Copy failed";
    case "not_applicable": return "Not applicable";
    default: return outcome.replaceAll("_", " ");
  }
}
