import { useState } from "react";
import { ArrowRight, Plus, Trash2 } from "lucide-react";
import { Card, Notice, Page, Toggle } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
import type { CorrectionEntry, SnippetEntry, WordEntry } from "../types";
type Tab = "words" | "corrections" | "snippets" | "formatting";
export default function Personalization({ state }: { state: LyraState }) {
  const [tab, setTab] = useState<Tab>("words");
  const [value, setValue] = useState("");
  const [replacement, setReplacement] = useState("");
  const p = state.personalization;
  const save = state.updatePersonalization;
  const add = () => {
    if (!value.trim()) return;
    if (tab === "words")
      void save({
        ...p,
        words: [...p.words, { phrase: value.trim(), enabled: true, boost: 5 }],
      });
    if (tab === "corrections" && replacement.trim())
      void save({
        ...p,
        corrections: [
          ...p.corrections,
          {
            source: value.trim(),
            replacement: replacement.trim(),
            enabled: true,
            caseSensitive: false,
          },
        ],
      });
    if (tab === "snippets" && replacement.trim())
      void save({
        ...p,
        snippets: [
          ...p.snippets,
          {
            trigger: value.trim(),
            expansion: replacement,
            enabled: true,
            caseSensitive: false,
          },
        ],
      });
    setValue("");
    setReplacement("");
  };
  return (
    <Page
      title="Personalization"
      subtitle="Teach Lyra your words, phrases, and shortcuts."
      action={
        tab !== "formatting" && (
          <button className="primary" onClick={add}>
            <Plus size={15} /> Add {tab.slice(0, -1)}
          </button>
        )
      }
    >
      <div className="tabs">
        {(["words", "corrections", "snippets", "formatting"] as Tab[]).map(
          (t) => (
            <button
              className={tab === t ? "active" : ""}
              onClick={() => setTab(t)}
              key={t}
            >
              {t[0].toUpperCase() + t.slice(1)}
            </button>
          ),
        )}
      </div>
      {tab !== "formatting" && (
        <Card className="entry-composer">
          <input
            aria-label={tab === "words" ? "Word or phrase" : "Spoken trigger"}
            placeholder={tab === "words" ? "Word or phrase" : "Spoken trigger"}
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
          {tab !== "words" && (
            <textarea
              aria-label="Replacement"
              placeholder={
                tab === "snippets" ? "Exact multiline expansion" : "Replacement"
              }
              value={replacement}
              onChange={(e) => setReplacement(e.target.value)}
            />
          )}
        </Card>
      )}
      {tab === "words" && (
        <EntryList
          items={p.words}
          empty="No custom words yet."
          render={(item: WordEntry) => (
            <>
              <span>{item.phrase}</span>
              <small>Boost {item.boost.toFixed(0)}</small>
            </>
          )}
          onToggle={(i, v) =>
            void save({
              ...p,
              words: p.words.map((x, n) =>
                n === i ? { ...x, enabled: v } : x,
              ),
            })
          }
          onDelete={(i) =>
            void save({ ...p, words: p.words.filter((_, n) => n !== i) })
          }
        />
      )}{" "}
      {tab === "corrections" && (
        <EntryList
          items={p.corrections}
          empty="No corrections yet."
          render={(item: CorrectionEntry) => (
            <>
              <span>{item.source}</span>
              <ArrowRight size={14} />
              <strong>{item.replacement}</strong>
            </>
          )}
          onToggle={(i, v) =>
            void save({
              ...p,
              corrections: p.corrections.map((x, n) =>
                n === i ? { ...x, enabled: v } : x,
              ),
            })
          }
          onDelete={(i) =>
            void save({
              ...p,
              corrections: p.corrections.filter((_, n) => n !== i),
            })
          }
        />
      )}{" "}
      {tab === "snippets" && (
        <EntryList
          items={p.snippets}
          empty="No snippets yet."
          render={(item: SnippetEntry) => (
            <>
              <span>{item.trigger}</span>
              <ArrowRight size={14} />
              <strong className="snippet-preview">{item.expansion}</strong>
            </>
          )}
          onToggle={(i, v) =>
            void save({
              ...p,
              snippets: p.snippets.map((x, n) =>
                n === i ? { ...x, enabled: v } : x,
              ),
            })
          }
          onDelete={(i) =>
            void save({ ...p, snippets: p.snippets.filter((_, n) => n !== i) })
          }
        />
      )}{" "}
      {tab === "formatting" && (
        <Card>
          <div className="setting-row">
            <div>
              <strong>Conservative self-correction</strong>
              <small>
                “Tuesday—sorry, Thursday” replaces only the current clause.
              </small>
            </div>
            <Toggle
              label="Self correction"
              checked={p.selfCorrectionEnabled}
              onChange={(v) => void save({ ...p, selfCorrectionEnabled: v })}
            />
          </div>
          <div className="setting-row">
            <div>
              <strong>App-aware profiles</strong>
              <small>
                {p.profiles.length || 4} built-in terminal, developer, chat, and
                editor profiles.
              </small>
            </div>
            <button className="secondary">Manage</button>
          </div>
        </Card>
      )}
      <Notice>
        {tab === "words"
          ? "Enabled words are sent only to the experimental Nemotron engine as recognition context. Parakeet is unchanged."
          : "Matches are local, deterministic, and applied only after transcription."}
      </Notice>
    </Page>
  );
}
function EntryList<T extends { enabled: boolean }>({
  items,
  empty,
  render,
  onToggle,
  onDelete,
}: {
  items: T[];
  empty: string;
  render: (i: T) => React.ReactNode;
  onToggle: (i: number, v: boolean) => void;
  onDelete: (i: number) => void;
}) {
  return (
    <Card className="entry-list">
      {items.length === 0 ? (
        <div className="empty">{empty}</div>
      ) : (
        items.map((item, i) => (
          <div className="entry" key={i}>
            <Toggle
              label="Enabled"
              checked={item.enabled}
              onChange={(v) => onToggle(i, v)}
            />
            <div className="entry-main">{render(item)}</div>
            <button
              className="icon-button"
              aria-label="Delete"
              onClick={() => onDelete(i)}
            >
              <Trash2 size={15} />
            </button>
          </div>
        ))
      )}
    </Card>
  );
}
