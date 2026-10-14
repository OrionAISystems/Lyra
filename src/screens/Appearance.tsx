import { Monitor, PanelTop, Power, SunMoon, Sparkles, RefreshCw } from "lucide-react";
import { useEffect, useState, type CSSProperties } from "react";
import { getThemeContent, isTauri } from "../bridge";
import { bundledOctober, paletteStyle, themeRefreshMessage, type ThemeContent } from "../themes";
import { SeasonalArtwork } from "../components/SeasonalArtwork";
import { Card, Page, Row, Select, Toggle } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
function themeDates(theme: Pick<ThemeContent, "startMonthDay" | "endMonthDay">) {
  const first = new Date(`2000-${theme.startMonthDay}T12:00:00Z`);
  const last = new Date(`2000-${theme.endMonthDay}T12:00:00Z`);
  last.setUTCDate(last.getUTCDate() - 1);
  const format = (date: Date) => date.toLocaleDateString(undefined, { month: "long", day: "numeric", timeZone: "UTC" });
  return `${format(first)} – ${format(last)}`;
}
export default function Appearance({ state }: { state: LyraState }) {
  const s = state.settings,
    u = state.updateSettings;
  const seasonal = state.seasonal;
  const [preview, setPreview] = useState<ThemeContent>(seasonal.content ?? bundledOctober);
  const [previewId, setPreviewId] = useState(seasonal.content?.id ?? "october");
  const [previewChosen, setPreviewChosen] = useState(false);
  const [previewState, setPreviewState] = useState<"idle" | "listening" | "processing" | "success">("idle");
  useEffect(() => {
    if (!previewChosen && seasonal.content) {
      setPreview(seasonal.content);
      setPreviewId(seasonal.content.id);
    }
  }, [previewChosen, seasonal.content]);
  useEffect(() => {
    let current = true;
    if (isTauri) void getThemeContent(previewId).then((content) => {
      if (current && content) setPreview(content);
    }).catch(() => {});
    return () => { current = false; };
  }, [seasonal.status.catalogRevision, previewId]);
  const scheme = s.theme === "system" ? window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light" : s.theme;
  const source = seasonal.content ? seasonal.status.source : seasonal.status.availableThemes.find((theme) => theme.id === preview.id)?.source ?? "bundled";
  return (
    <Page title="Appearance" subtitle="Make Lyra feel at home.">
      <div className="theme-grid">
        {(["dark", "light", "system"] as const).map((theme) => (
          <button
            key={theme}
            className={s.theme === theme ? "selected" : ""}
            onClick={() => void u({ theme })}
          >
            <span className={`theme-preview ${theme}`} />
            <strong>{theme[0].toUpperCase() + theme.slice(1)}</strong>
          </button>
        ))}
      </div>
      <section className="seasonal-settings card" aria-label="Seasonal themes">
        <div className="seasonal-settings__heading">
          <span className="row-icon"><Sparkles /></span>
          <div><h2>Seasonal themes</h2><p>{seasonal.content ? `${seasonal.content.name} · ${themeDates(seasonal.content)}` : "Artwork and colors that follow the calendar."}</p></div>
          <span className="seasonal-settings__badge">{!s.seasonalThemesEnabled ? "Off" : seasonal.content ? "Active now" : "Automatic"}</span>
        </div>
        <Row title="Follow the seasons" description="Switch themes automatically using your computer’s local date.">
          <Toggle label="Seasonal themes" checked={s.seasonalThemesEnabled} onChange={(value) => void u({ seasonalThemesEnabled: value })} />
        </Row>
        <Row title="Seasonal motion" description={seasonal.reducedMotion ? "Your system’s reduced-motion preference is active." : "Gentle ambience and animations. Always silent."}>
          <Toggle label="Seasonal motion" checked={s.seasonalMotionEnabled} onChange={(value) => void u({ seasonalMotionEnabled: value })} />
        </Row>
        {seasonal.status.availableThemes.length > 1 && <Row title="Preview a theme" description="Previewing doesn’t change the active seasonal dates.">
          <Select label="Theme preview" value={previewId} onChange={(id) => { setPreviewChosen(true); setPreviewId(id); }}>{seasonal.status.availableThemes.map((theme) => <option key={theme.id} value={theme.id}>{theme.name}</option>)}</Select>
        </Row>}
        <div className="seasonal-preview" style={paletteStyle(preview.palettes[scheme]) as CSSProperties} data-preview-motion={seasonal.motionEnabled ? "on" : "off"}>
          <div className="seasonal-preview__art"><SeasonalArtwork theme={preview} state={previewState} inputLevel={0.65} /></div>
          <div className="seasonal-preview__copy"><span className="seasonal-preview__eyebrow">SEASONAL COLLECTION</span><h3>{preview.name}</h3><p>{preview.description}</p><small>{themeDates(preview)} · Every year · Local time</small></div>
          <div className="seasonal-preview__states" role="group" aria-label="Preview dictation state">
            {(["idle", "listening", "processing", "success"] as const).map((value) => <button key={value} type="button" aria-pressed={previewState === value} onClick={() => setPreviewState(value)}>{value === "idle" ? "Ready" : value === "success" ? "Done" : value[0].toUpperCase() + value.slice(1)}</button>)}
          </div>
        </div>
        <div className="seasonal-settings__delivery">
          <div><strong>{source === "remote" ? "Verified remote theme" : source === "cache" ? "Saved for offline use" : "Included with Lyra"}</strong><small>{seasonal.status.refreshedAt ? `Last checked ${new Date(seasonal.status.refreshedAt).toLocaleString()}` : "October works offline. New theme packs arrive automatically."}</small></div>
          <button className="secondary" type="button" disabled={seasonal.refreshing || !s.seasonalThemesEnabled} onClick={() => void seasonal.refreshThemes()}><RefreshCw size={13} className={seasonal.refreshing ? "seasonal-refreshing" : ""} />{seasonal.refreshing ? "Checking…" : "Check for themes"}</button>
        </div>
        {seasonal.status.error && <p className="seasonal-settings__notice" role="status">{themeRefreshMessage(seasonal.status.error)}</p>}
        {state.settingsError && <p className="seasonal-settings__notice" role="alert">{state.settingsError}</p>}
      </section>
      <Card>
        <Row icon={<SunMoon />} title="Accent color">
          <Select
            label="Accent color"
            value={s.accent}
            onChange={(v) => void u({ accent: v as typeof s.accent })}
          >
            <option value="purple">Purple</option>
            <option value="blue">Blue</option>
            <option value="teal">Teal</option>
            <option value="rose">Rose</option>
          </Select>
        </Row>
        <Row
          icon={<PanelTop />}
          title="Compact mode"
          description="Use a more compact layout"
        >
          <Toggle
            label="Compact mode"
            checked={s.compactMode}
            onChange={(v) => void u({ compactMode: v })}
          />
        </Row>
        <Row
          icon={<Monitor />}
          title="Show tray icon"
          description="Keep Lyra in the system tray"
        >
          <Toggle
            label="Show tray icon"
            checked={s.showTrayIcon}
            onChange={(v) => void u({ showTrayIcon: v })}
          />
        </Row>
        <Row
          icon={<Power />}
          title="Start with Windows"
          description="Launch Lyra automatically"
        >
          <Toggle
            label="Start with Windows"
            checked={s.launchAtLogin}
            onChange={(v) => void u({ launchAtLogin: v })}
          />
        </Row>
        <Row icon={<PanelTop />} title="Show dictation bar" description="Keep the floating dictation control visible">
          <Toggle
            label="Show dictation bar"
            checked={s.showBar}
            onChange={(v) => void u({ showBar: v })}
          />
        </Row>
        <Row icon={<Monitor />} title="Dictation bar placement">
          <Select
            label="Bar placement"
            value={s.barPlacement}
            onChange={(v) =>
              void u({ barPlacement: v as typeof s.barPlacement })
            }
          >
            <option value="follow_active_window">Follow active window</option>
            <option value="primary_display">Primary display</option>
            <option value="fixed_monitor">Fixed monitor</option>
          </Select>
        </Row>
      </Card>
    </Page>
  );
}
