import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../bridge";
import { bundledThemeStatus, paletteKeys, paletteStyle, type ThemeStatus } from "../themes";
import type { Settings } from "../types";

export function useSeasonalTheme(settings: Settings) {
  const snapshotSequence = useRef(0);
  // Native settings may disable seasonality; wait for its snapshot before decorating.
  const [status, setStatus] = useState<ThemeStatus>(() => bundledThemeStatus(api.isTauri ? false : settings.seasonalThemesEnabled, settings.seasonalMotionEnabled));
  const [refreshing, setRefreshing] = useState(false);
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  const [reducedMotion, setReducedMotion] = useState(() => window.matchMedia("(prefers-reduced-motion: reduce)").matches);
  const sync = useCallback(async () => {
    const sequence = ++snapshotSequence.current;
    if (!api.isTauri) {
      setStatus(bundledThemeStatus(settings.seasonalThemesEnabled, settings.seasonalMotionEnabled));
      return;
    }
    const next = await api.getThemeStatus();
    if (next && sequence === snapshotSequence.current) setStatus(next);
  }, [settings.seasonalThemesEnabled, settings.seasonalMotionEnabled]);

  useEffect(() => {
    const color = window.matchMedia("(prefers-color-scheme: dark)");
    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const updateColor = () => setDark(color.matches);
    const updateMotion = () => setReducedMotion(motion.matches);
    color.addEventListener("change", updateColor);
    motion.addEventListener("change", updateMotion);
    return () => { color.removeEventListener("change", updateColor); motion.removeEventListener("change", updateMotion); };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unsubscribe = () => {};
    void api.listenToThemeChanges((next) => {
      if (!disposed) { snapshotSequence.current += 1; setStatus(next); }
    }).then((unlisten) => {
      if (disposed) unlisten(); else unsubscribe = unlisten;
      if (!disposed) void sync().catch(() => {});
    }).catch(() => { if (!disposed) void sync().catch(() => {}); });
    const onVisible = () => { if (!document.hidden) void sync().catch(() => {}); };
    window.addEventListener("focus", onVisible);
    document.addEventListener("visibilitychange", onVisible);
    // Guards clock changes/resume; the native service also schedules midnight.
    const timer = window.setInterval(onVisible, 60_000);
    return () => {
      disposed = true; snapshotSequence.current += 1; unsubscribe(); window.clearInterval(timer);
      window.removeEventListener("focus", onVisible);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [sync]);

  const content = settings.seasonalThemesEnabled ? status.content : null;
  const motionEnabled = settings.seasonalMotionEnabled && !reducedMotion;
  const scheme = settings.theme === "system" ? dark ? "dark" : "light" : settings.theme;
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.seasonalMotion = motionEnabled ? "on" : "off";
    if (content) {
      root.dataset.seasonalTheme = content.id;
      for (const [key, value] of Object.entries(paletteStyle(content.palettes[scheme]))) root.style.setProperty(key, value);
      for (const [key, value] of Object.entries(paletteStyle(content.palettes.dark))) root.style.setProperty(`--seasonal-bar-${key.slice(2)}`, value);
    } else {
      delete root.dataset.seasonalTheme;
      for (const key of paletteKeys) { root.style.removeProperty(`--${key}`); root.style.removeProperty(`--seasonal-bar-${key}`); }
    }
    return () => {
      delete root.dataset.seasonalTheme;
      for (const key of paletteKeys) { root.style.removeProperty(`--${key}`); root.style.removeProperty(`--seasonal-bar-${key}`); }
    };
  }, [content, scheme, motionEnabled]);

  const refreshThemes = useCallback(async () => {
    const sequence = ++snapshotSequence.current;
    setRefreshing(true);
    try {
      const next = await api.refreshThemes();
      if (next) { if (sequence === snapshotSequence.current) setStatus(next); } else await sync();
    } catch {
      if (sequence === snapshotSequence.current) setStatus((current) => ({ ...current, error: "Themes couldn't be refreshed. Your current appearance is still available." }));
    } finally { setRefreshing(false); }
  }, [sync]);
  return { status, content, motionEnabled, reducedMotion, refreshing, refreshThemes };
}
