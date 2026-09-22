import { Monitor, PanelTop, Power, SunMoon } from "lucide-react";
import { Card, Page, Row, Select, Toggle } from "../components/Controls";
import type { LyraState } from "../state/useLyra";
export default function Appearance({ state }: { state: LyraState }) {
  const s = state.settings,
    u = state.updateSettings;
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
