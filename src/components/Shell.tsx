import {
  Activity,
  AudioLines,
  BarChart3,
  Home,
  Mic2,
  Minus,
  Palette,
  Square,
  UserRoundCog,
  X,
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ReactNode } from "react";
import { isTauri } from "../bridge";
import type { Screen } from "../types";
import lyraIcon from "../assets/lyra-icon.png";
const nav: [Screen, string, ReactNode][] = [
  ["home", "Home", <Home />],
  ["dictation", "Dictation", <Mic2 />],
  ["personalization", "Personalization", <UserRoundCog />],
  ["audio", "Audio & Engine", <AudioLines />],
  ["insights", "Insights", <BarChart3 />],
  ["appearance", "Appearance", <Palette />],
  ["diagnostics", "Diagnostics", <Activity />],
];
export function Shell({
  screen,
  setScreen,
  ready,
  children,
}: {
  screen: Screen;
  setScreen: (s: Screen) => void;
  ready: boolean;
  children: ReactNode;
}) {
  const win = isTauri ? getCurrentWindow() : null;
  return (
    <div className="window-shell">
      <div className="titlebar" data-tauri-drag-region>
        <div className="traffic">
          <i />
          <i />
          <i />
        </div>
        <div className="window-actions">
          <button aria-label="Minimize" onClick={() => void win?.minimize()}>
            <Minus />
          </button>
          <button
            aria-label="Maximize or restore"
            onClick={() => void win?.toggleMaximize()}
          >
            <Square />
          </button>
          <button aria-label="Close to tray" onClick={() => void win?.hide()}>
            <X />
          </button>
        </div>
      </div>
      <aside>
        <div className="brand">
          <img src={lyraIcon} />
          <div>
            <strong>Lyra</strong>
            <span>Orion voice layer</span>
          </div>
        </div>
        <nav>
          {nav.map(([id, label, icon]) => (
            <button
              key={id}
              className={screen === id ? "active" : ""}
              onClick={() => setScreen(id)}
            >
              {icon}
              <span>{label}</span>
            </button>
          ))}
        </nav>
        <footer>
          <div>
            <i className={ready ? "ready" : ""} />
            {ready ? "Ready locally" : "Setup required"}
          </div>
          <span>v0.4.0</span>
        </footer>
      </aside>
      <div className="content">{children}</div>
    </div>
  );
}
