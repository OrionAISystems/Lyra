import { useMemo, useState } from "react";
import { Activity, FileText, Timer, Zap } from "lucide-react";
import { Card, Page, Select } from "../components/Controls";
import { formatUsageDay } from "../insights";
import type { LyraState } from "../state/useLyra";

export default function Insights({ state }: { state: LyraState }) {
  const [range, setRange] = useState("7");
  const [advanced, setAdvanced] = useState(false);
  const daily = state.usage.daily.slice(-Number(range));
  const max = Math.max(1, ...daily.map((d) => d.words));
  const noSpeech = state.usage.failureCounts.no_speech ?? 0;
  const considered = Math.max(1, state.usage.dictationsAllTime - noSpeech);
  const successRate = (state.usage.completedResults / considered) * 100;
  const typingHours =
    state.settings.typingWpm > 0
      ? state.usage.totalWords / state.settings.typingWpm / 60
      : 0;
  const selectedWords = useMemo(
    () => daily.reduce((sum, day) => sum + day.words, 0),
    [daily],
  );

  return (
    <Page
      title="Insights"
      subtitle="A private, metadata-only view of your usage."
      action={
        <div className="insights-actions">
          <Select label="Range" value={range} onChange={setRange}>
            <option value="7">Last 7 days</option>
            <option value="30">Last 30 days</option>
            <option value="90">Last 90 days</option>
          </Select>
          <button
            type="button"
            className="secondary"
            onClick={() => setAdvanced((value) => !value)}
          >
            {advanced ? "Hide advanced" : "Show advanced"}
          </button>
        </div>
      }
    >
      <div className="metrics">
        <Metric
          icon={<Activity />}
          value={state.usage.minutesDictated.toFixed(0)}
          label="Minutes dictated"
        />
        <Metric
          icon={<FileText />}
          value={state.usage.totalWords.toLocaleString()}
          label="Words dictated"
        />
        <Metric
          icon={<Zap />}
          value={`${successRate.toFixed(0)}%`}
          label="Success rate"
        />
        <Metric
          icon={<Timer />}
          value={`${state.usage.averageTranscriptionLatencyMs.toFixed(0)}ms`}
          label="Average latency"
        />
      </div>
      <Card className="chart">
        <div className="chart-legend" aria-label="Engine legend">
          <span>
            <i className="legend-swatch parakeet" />
            Parakeet
          </span>
          <span>
            <i className="legend-swatch nemotron" />
            Nemotron
          </span>
        </div>
        <div
          className="bars"
          role="img"
          aria-label="Daily dictated words chart, split by engine"
        >
          {daily.map((d, i) => {
            const parakeet = d.parakeetWords || (d.nemotronWords ? 0 : d.words);
            const nemotron = d.nemotronWords;
            const total = Math.max(1, parakeet + nemotron || d.words);
            return (
              <div className="bar-slot" key={`${d.day}-${i}`}>
                <div
                  className="bar-stack"
                  style={{ height: `${Math.max(8, (d.words / max) * 100)}%` }}
                  title={`${formatUsageDay(d.day)}: ${d.words} words (${parakeet} Parakeet, ${nemotron} Nemotron)`}
                >
                  <i
                    className="bar-segment parakeet"
                    style={{ height: `${(parakeet / total) * 100}%` }}
                  />
                  <i
                    className="bar-segment nemotron"
                    style={{ height: `${(nemotron / total) * 100}%` }}
                  />
                </div>
                <span>{formatUsageDay(d.day)}</span>
              </div>
            );
          })}
        </div>
      </Card>
      {advanced && (
        <Card className="advanced-insights">
          <div className="advanced-heading">
            <div>
              <strong>Advanced usage</strong>
              <small>Detailed metadata, never transcript content.</small>
            </div>
            <label className="wpm-control">
              <span>Typing speed (WPM)</span>
              <input
                type="number"
                min={10}
                max={200}
                step={1}
                value={state.settings.typingWpm}
                onChange={(event) =>
                  void state.updateSettings({
                    typingWpm: Number(event.target.value) || 40,
                  })
                }
              />
            </label>
          </div>
          <div className="advanced-grid">
            <span>
              <b>{state.usage.recentSessions.length}</b> recent sessions
              retained
            </span>
            <span>
              <b>{state.usage.averageRealTimeFactor.toFixed(2)}×</b> average RTF
            </span>
            <span>
              <b>{state.usage.pasteSuccessRate.toFixed(0)}%</b> paste success
            </span>
            <span>
              <b>{state.usage.cloudFallbacks}</b> cloud fallbacks
            </span>
            <span>
              <b>{selectedWords.toLocaleString()}</b> words in range
            </span>
            <span>
              <b>{noSpeech}</b> silence-only captures excluded
            </span>
          </div>
        </Card>
      )}
      <Card className="narrative">
        <strong>“Small input. Big output.”</strong>
        <p>
          You’ve saved an estimated {typingHours.toFixed(1)} hours of typing at{" "}
          {state.settings.typingWpm} WPM. Paste succeeded{" "}
          {state.usage.pasteSuccessRate.toFixed(0)}% of the time.
        </p>
        {state.usage.detailedSince && (
          <small>
            Detailed session metadata begins {state.usage.detailedSince};
            earlier lifetime totals are preserved separately.
          </small>
        )}
      </Card>
    </Page>
  );
}

function Metric({
  icon,
  value,
  label,
}: {
  icon: React.ReactNode;
  value: string;
  label: string;
}) {
  return (
    <Card className="metric">
      <i>{icon}</i>
      <strong>{value}</strong>
      <span>{label}</span>
    </Card>
  );
}
