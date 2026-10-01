import october from "../themes/october/theme.json";
import pumpkin from "../themes/october/assets/pumpkin.png";
import leaf from "../themes/october/assets/leaf.png";

export const paletteKeys = ["bg", "panel", "panel2", "panel3", "line", "text", "muted", "chrome", "sidebar", "surface", "control", "accent", "accent2", "glow"] as const;
export type ThemePalette = Record<typeof paletteKeys[number], string>;
export type ArtworkSlot = "idle" | "listening" | "processing" | "success" | "error" | "bar" | "empty" | "ambience";
export type ThemeSource = "remote" | "cache" | "bundled";
export interface ThemeContent {
  id: string;
  name: string;
  description: string;
  revision: number;
  startMonthDay: string;
  endMonthDay: string;
  palettes: { dark: ThemePalette; light: ThemePalette };
  artwork: Record<ArtworkSlot, string>;
}
export interface AvailableTheme {
  id: string;
  name: string;
  revision: number;
  startMonthDay: string;
  endMonthDay: string;
  source: ThemeSource;
}
export interface ThemeStatus {
  enabled: boolean;
  motionEnabled: boolean;
  state: "disabled" | "active" | "upcoming" | "fallback" | "unavailable";
  themeId: string | null;
  catalogRevision: number | null;
  nextChangeAt: string | null;
  refreshedAt: string | null;
  error: string | null;
  source: ThemeSource | null;
  availableThemes: AvailableTheme[];
  content: ThemeContent | null;
}

export const bundledOctober: ThemeContent = {
  id: october.id,
  name: october.name,
  description: october.description,
  revision: october.revision,
  startMonthDay: october.startMonthDay,
  endMonthDay: october.endMonthDay,
  palettes: october.palettes,
  artwork: { idle: pumpkin, listening: pumpkin, processing: pumpkin, success: pumpkin, error: pumpkin, bar: pumpkin, empty: pumpkin, ambience: leaf },
};

// Browser preview follows the same annual local window; production dates are Rust-owned.
export function withinSeason(date: Date, start: string, end: string): boolean {
  const today = `${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
  return start < end ? today >= start && today < end : today >= start || today < end;
}

export function bundledThemeStatus(enabled: boolean, motionEnabled: boolean, date = new Date()): ThemeStatus {
  const active = enabled && withinSeason(date, bundledOctober.startMonthDay, bundledOctober.endMonthDay);
  return {
    enabled, motionEnabled, state: !enabled ? "disabled" : active ? "fallback" : "upcoming",
    themeId: active ? bundledOctober.id : null, catalogRevision: null, nextChangeAt: null,
    refreshedAt: null, error: null, source: active ? "bundled" : null,
    availableThemes: [{ id: bundledOctober.id, name: bundledOctober.name, revision: bundledOctober.revision, startMonthDay: bundledOctober.startMonthDay, endMonthDay: bundledOctober.endMonthDay, source: "bundled" }], content: active ? bundledOctober : null,
  };
}

export function paletteStyle(palette: ThemePalette): Record<string, string> {
  const style: Record<string, string> = {};
  for (const key of paletteKeys) style[`--${key}`] = palette[key];
  const hex = palette.glow.slice(1);
  style["--glow"] = `rgba(${parseInt(hex.slice(0, 2), 16)}, ${parseInt(hex.slice(2, 4), 16)}, ${parseInt(hex.slice(4, 6), 16)}, 0.24)`;
  return style;
}

export function themeRefreshMessage(error: string): string {
  const messages: Record<string, string> = {
    "catalog-unavailable": "Theme updates are unavailable right now. Included and saved themes still work offline.",
    "catalog-invalid": "The theme update couldn’t be verified. Your current appearance is still available.",
    "catalog-rollback-rejected": "An older theme update was skipped. Your current appearance is still available.",
    "assets-unavailable": "The theme download couldn’t finish. Your current appearance is still available.",
    "cache-unavailable": "Saved theme content couldn’t be loaded. Included themes are still available.",
  };
  return messages[error] ?? "Themes couldn’t be refreshed. Your current appearance is still available.";
}
