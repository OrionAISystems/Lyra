import { useState } from "react";
import { Activity, FileText, Timer, Zap } from "lucide-react";
import { Card, Page, Select } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
export default function Insights({ state }: { state: LyraState }) {
  const [range, setRange] = useState("7");
  const daily = state.usage.daily.slice(-Number(range));
  const max = Math.max(1, ...daily.map((d) => d.words));
  return (
    <Page
      title="Insights"
      subtitle="A private, metadata-only view of your usage."
      action={
        <Select label="Range" value={range} onChange={setRange}>
          <option value="7">Last 7 days</option>
          <option value="30">Last 30 days</option>
          <option value="90">Last 90 days</option>
        </Select>
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
          value={`${((state.usage.completedResults / Math.max(1, state.usage.dictationsAllTime)) * 100).toFixed(0)}%`}
          label="Success rate"
        />
        <Metric
          icon={<Timer />}
          value={`${state.usage.averageTranscriptionLatencyMs.toFixed(0)}ms`}
          label="Average latency"
        />
      </div>
      <Card className="chart">
        <div
          className="bars"
          role="img"
          aria-label="Daily dictated words chart"
        >
          {daily.map((d, i) => (
            <div className="bar-slot" key={`${d.day}-${i}`}>
              <div
                className="bar"
                style={{ height: `${Math.max(8, (d.words / max) * 100)}%` }}
                title={`${d.day}: ${d.words} words`}
              />
              <span>{d.day.slice(-3)}</span>
            </div>
          ))}
        </div>
      </Card>
      <Card className="narrative">
        <strong>“Small input. Big output.”</strong>
        <p>
          You’ve saved an estimated{" "}
          {state.usage.estimatedTypingHoursSaved.toFixed(1)} hours of typing.
          Paste succeeded {state.usage.pasteSuccessRate.toFixed(0)}% of the
          time.
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
