// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { bundledThemeStatus, type ThemeStatus } from "../themes";
import { defaultSettings, type Settings } from "../types";
import { useSeasonalTheme } from "./useSeasonalTheme";

const bridgeState = vi.hoisted(() => ({
  isTauri: false,
  getThemeStatus: vi.fn(),
  listenToThemeChanges: vi.fn(),
  refreshThemes: vi.fn(),
  saveSettings: vi.fn(),
}));

vi.mock("../bridge", () => ({
  get isTauri() { return bridgeState.isTauri; },
  getThemeStatus: bridgeState.getThemeStatus,
  listenToThemeChanges: bridgeState.listenToThemeChanges,
  refreshThemes: bridgeState.refreshThemes,
  saveSettings: bridgeState.saveSettings,
}));

type ThemeHookValue = ReturnType<typeof useSeasonalTheme>;
type MediaQueryStub = MediaQueryList & { setMatches(value: boolean): void };

function ThemeHookProbe({ settings, onValue }: { settings: Settings; onValue: (value: ThemeHookValue) => void }) {
  onValue(useSeasonalTheme(settings));
  return null;
}

function mediaQueryStub(query: string, initialMatches: boolean): MediaQueryStub {
  let matches = initialMatches;
  const listeners = new Set<EventListenerOrEventListenerObject>();
  return {
    media: query,
    onchange: null,
    get matches() { return matches; },
    addEventListener: (_type: string, listener: EventListenerOrEventListenerObject | null) => { if (listener) listeners.add(listener); },
    removeEventListener: (_type: string, listener: EventListenerOrEventListenerObject | null) => { if (listener) listeners.delete(listener); },
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: (event) => {
      for (const listener of listeners) {
        if (typeof listener === "function") listener(event);
        else listener.handleEvent(event);
      }
      return true;
    },
    setMatches(value) {
      matches = value;
      this.dispatchEvent(Object.assign(new Event("change"), { matches, media: query }));
    },
  } as MediaQueryStub;
}

function activeStatus(refreshedAt = "initial"): ThemeStatus {
  return {
    ...bundledThemeStatus(true, true, new Date(2026, 9, 15, 12)),
    state: "active",
    source: "remote",
    catalogRevision: 1,
    refreshedAt,
  };
}

function settings(overrides: Partial<Settings> = {}): Settings {
  return { ...defaultSettings, ...overrides };
}

let root: Root | undefined;
let container: HTMLDivElement;
let current: ThemeHookValue | undefined;
let mediaQueries: Map<string, MediaQueryStub>;
let initialDocumentAttributes: { style: string | null; seasonalTheme: string | null; seasonalMotion: string | null };
let matchMediaDescriptor: PropertyDescriptor | undefined;

function media(query: string): MediaQueryStub {
  const result = mediaQueries.get(query);
  if (!result) throw new Error(`No media query stub registered for ${query}`);
  return result;
}

async function flushMicrotasks() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

async function mount(settingsValue: Settings) {
  await act(async () => {
    root = createRoot(container);
    root.render(<ThemeHookProbe settings={settingsValue} onValue={(value) => { current = value; }} />);
    await flushMicrotasks();
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2026, 9, 15, 12));
  bridgeState.isTauri = false;
  bridgeState.getThemeStatus.mockReset();
  bridgeState.listenToThemeChanges.mockReset().mockImplementation(async () => () => {});
  bridgeState.refreshThemes.mockReset();
  bridgeState.saveSettings.mockReset();
  mediaQueries = new Map();
  matchMediaDescriptor = Object.getOwnPropertyDescriptor(window, "matchMedia");
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn((query: string) => {
      let result = mediaQueries.get(query);
      if (!result) {
        result = mediaQueryStub(query, false);
        mediaQueries.set(query, result);
      }
      return result;
    }),
  });
  initialDocumentAttributes = {
    style: document.documentElement.getAttribute("style"),
    seasonalTheme: document.documentElement.getAttribute("data-seasonal-theme"),
    seasonalMotion: document.documentElement.getAttribute("data-seasonal-motion"),
  };
  document.documentElement.removeAttribute("style");
  document.documentElement.removeAttribute("data-seasonal-theme");
  document.documentElement.removeAttribute("data-seasonal-motion");
  container = document.createElement("div");
  document.body.append(container);
  (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
});

afterEach(async () => {
  if (root) await act(async () => { root?.unmount(); });
  root = undefined;
  container.remove();
  vi.useRealTimers();
  if (matchMediaDescriptor) Object.defineProperty(window, "matchMedia", matchMediaDescriptor);
  else delete (window as unknown as { matchMedia?: typeof window.matchMedia }).matchMedia;
  const html = document.documentElement;
  if (initialDocumentAttributes.style === null) html.removeAttribute("style");
  else html.setAttribute("style", initialDocumentAttributes.style);
  if (initialDocumentAttributes.seasonalTheme === null) html.removeAttribute("data-seasonal-theme");
  else html.setAttribute("data-seasonal-theme", initialDocumentAttributes.seasonalTheme);
  if (initialDocumentAttributes.seasonalMotion === null) html.removeAttribute("data-seasonal-motion");
  else html.setAttribute("data-seasonal-motion", initialDocumentAttributes.seasonalMotion);
});

describe("useSeasonalTheme", () => {
  it("applies the system palette without changing saved appearance preferences", async () => {
    const savedSettings = settings({ theme: "system", accent: "blue", seasonalThemesEnabled: true });
    bridgeState.isTauri = true;
    bridgeState.getThemeStatus.mockResolvedValue(activeStatus());
    await mount(savedSettings);

    expect(current?.status.state).toBe("active");
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#984410");
    expect(document.documentElement.style.getPropertyValue("--seasonal-bar-accent")).toBe("#f4a45c");

    await act(async () => {
      media("(prefers-color-scheme: dark)").setMatches(true);
      await flushMicrotasks();
    });
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#f4a45c");
    expect(document.documentElement.style.getPropertyValue("--seasonal-bar-accent")).toBe("#f4a45c");

    await act(async () => {
      media("(prefers-color-scheme: dark)").setMatches(false);
      await flushMicrotasks();
    });
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#984410");
    expect(savedSettings).toMatchObject({ theme: "system", accent: "blue" });
    expect(bridgeState.saveSettings).not.toHaveBeenCalled();
  });

  it("waits for the native status snapshot before decorating an enabled theme", async () => {
    let resolveStatus!: (status: ThemeStatus) => void;
    bridgeState.isTauri = true;
    bridgeState.getThemeStatus.mockReturnValue(new Promise<ThemeStatus>((resolve) => { resolveStatus = resolve; }));

    await mount(settings({ theme: "system", seasonalThemesEnabled: true }));

    expect(current?.content).toBeNull();
    expect(document.documentElement.dataset.seasonalTheme).toBeUndefined();
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("");

    await act(async () => {
      resolveStatus(activeStatus("native-snapshot"));
      await flushMicrotasks();
    });

    expect(current?.status.refreshedAt).toBe("native-snapshot");
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#984410");
  });

  it("removes seasonal overrides when disabled while retaining the user's theme and accent settings", async () => {
    const savedSettings = settings({ theme: "light", accent: "rose", seasonalThemesEnabled: true });
    await mount(savedSettings);
    expect(document.documentElement.style.getPropertyValue("--accent")).not.toBe("");

    const disabled = { ...savedSettings, seasonalThemesEnabled: false };
    await act(async () => {
      root?.render(<ThemeHookProbe settings={disabled} onValue={(value) => { current = value; }} />);
      await flushMicrotasks();
    });

    expect(current?.content).toBeNull();
    expect(document.documentElement.dataset.seasonalTheme).toBeUndefined();
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("");
    expect(savedSettings).toMatchObject({ theme: "light", accent: "rose" });
    expect(bridgeState.saveSettings).not.toHaveBeenCalled();
  });

  it("turns seasonal motion off for either the saved switch or the system reduced-motion preference", async () => {
    await mount(settings({ seasonalThemesEnabled: true, seasonalMotionEnabled: true }));
    expect(current?.motionEnabled).toBe(true);
    expect(document.documentElement.dataset.seasonalMotion).toBe("on");

    await act(async () => {
      media("(prefers-reduced-motion: reduce)").setMatches(true);
      await flushMicrotasks();
    });
    expect(current?.reducedMotion).toBe(true);
    expect(current?.motionEnabled).toBe(false);
    expect(document.documentElement.dataset.seasonalMotion).toBe("off");

    const motionDisabled = settings({ seasonalThemesEnabled: true, seasonalMotionEnabled: false });
    await act(async () => {
      root?.render(<ThemeHookProbe settings={motionDisabled} onValue={(value) => { current = value; }} />);
      await flushMicrotasks();
    });
    expect(current?.motionEnabled).toBe(false);
    expect(document.documentElement.dataset.seasonalMotion).toBe("off");
  });

  it("accepts native theme events and refreshes the status again when the window regains focus", async () => {
    const initial = activeStatus("initial");
    const focusRefresh = activeStatus("focus-refresh");
    const unavailable: ThemeStatus = { ...initial, state: "unavailable", themeId: null, source: null, content: null, error: "offline" };
    let onThemeChange: ((status: ThemeStatus) => void) | undefined;
    const unlisten = vi.fn();
    bridgeState.isTauri = true;
    bridgeState.listenToThemeChanges.mockImplementation(async (handler: (status: ThemeStatus) => void) => {
      onThemeChange = handler;
      return unlisten;
    });
    bridgeState.getThemeStatus.mockResolvedValueOnce(initial).mockResolvedValueOnce(focusRefresh);

    await mount(settings({ seasonalThemesEnabled: true }));
    expect(bridgeState.getThemeStatus).toHaveBeenCalledTimes(1);
    expect(current?.status.refreshedAt).toBe("initial");

    await act(async () => {
      onThemeChange?.(unavailable);
      await flushMicrotasks();
    });
    expect(current?.status.state).toBe("unavailable");
    expect(document.documentElement.dataset.seasonalTheme).toBeUndefined();

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      await flushMicrotasks();
    });
    expect(bridgeState.getThemeStatus).toHaveBeenCalledTimes(2);
    expect(current?.status.refreshedAt).toBe("focus-refresh");
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");

    await act(async () => { root?.unmount(); });
    root = undefined;
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("does not poll the full artwork status while the window stays visible", async () => {
    const hiddenDescriptor = Object.getOwnPropertyDescriptor(document, "hidden");
    Object.defineProperty(document, "hidden", { configurable: true, value: false });
    bridgeState.isTauri = true;
    bridgeState.getThemeStatus.mockResolvedValue(activeStatus());

    try {
      await mount(settings({ seasonalThemesEnabled: true }));
      expect(bridgeState.getThemeStatus).toHaveBeenCalledOnce();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000);
      });

      expect(bridgeState.getThemeStatus).toHaveBeenCalledOnce();
    } finally {
      if (hiddenDescriptor) Object.defineProperty(document, "hidden", hiddenDescriptor);
      else Reflect.deleteProperty(document, "hidden");
    }
  });

  it("does not let a pending status read overwrite a newer native theme event", async () => {
    let onThemeChange: ((status: ThemeStatus) => void) | undefined;
    let resolveInitial!: (status: ThemeStatus) => void;
    const nativeSnapshot = activeStatus("native-event");
    bridgeState.isTauri = true;
    bridgeState.listenToThemeChanges.mockImplementation(async (handler: (status: ThemeStatus) => void) => {
      onThemeChange = handler;
      return () => {};
    });
    bridgeState.getThemeStatus.mockReturnValue(new Promise<ThemeStatus>((resolve) => { resolveInitial = resolve; }));

    await mount(settings({ theme: "system", seasonalThemesEnabled: true }));
    expect(bridgeState.getThemeStatus).toHaveBeenCalledOnce();

    await act(async () => {
      onThemeChange?.(nativeSnapshot);
      await flushMicrotasks();
    });
    expect(current?.status.refreshedAt).toBe("native-event");

    await act(async () => {
      resolveInitial(activeStatus("stale-initial-read"));
      await flushMicrotasks();
    });

    expect(current?.status.refreshedAt).toBe("native-event");
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");
  });

  it("falls back to a status read when the native event subscription fails", async () => {
    const recovered = activeStatus("subscription-fallback");
    bridgeState.isTauri = true;
    bridgeState.listenToThemeChanges.mockRejectedValue(new Error("event channel unavailable"));
    bridgeState.getThemeStatus.mockResolvedValue(recovered);

    await mount(settings({ theme: "system", seasonalThemesEnabled: true }));

    expect(bridgeState.listenToThemeChanges).toHaveBeenCalledOnce();
    expect(bridgeState.getThemeStatus).toHaveBeenCalledOnce();
    expect(current?.status.refreshedAt).toBe("subscription-fallback");
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");
  });

  it("does not report a stale manual-refresh failure after a newer native snapshot arrives", async () => {
    let onThemeChange: ((status: ThemeStatus) => void) | undefined;
    let rejectRefresh!: (error: Error) => void;
    let pendingRefresh: Promise<void> | undefined;
    const initial = activeStatus("initial");
    const nativeSnapshot = activeStatus("native-event-after-refresh-started");
    bridgeState.isTauri = true;
    bridgeState.listenToThemeChanges.mockImplementation(async (handler: (status: ThemeStatus) => void) => {
      onThemeChange = handler;
      return () => {};
    });
    bridgeState.getThemeStatus.mockResolvedValue(initial);
    bridgeState.refreshThemes.mockReturnValue(new Promise<ThemeStatus>((_resolve, reject) => { rejectRefresh = reject; }));

    await mount(settings({ theme: "system", seasonalThemesEnabled: true }));
    await act(async () => {
      pendingRefresh = current?.refreshThemes();
      await flushMicrotasks();
    });

    await act(async () => {
      onThemeChange?.(nativeSnapshot);
      await flushMicrotasks();
    });
    await act(async () => {
      rejectRefresh(new Error("stale request failed"));
      await pendingRefresh;
    });

    expect(current?.status.refreshedAt).toBe("native-event-after-refresh-started");
    expect(current?.status.error).not.toBe("Themes couldn't be refreshed. Your current appearance is still available.");
    expect(current?.refreshing).toBe(false);
  });

  it("keeps the bundled appearance available and reports a manual refresh failure", async () => {
    const savedSettings = settings({ theme: "dark", accent: "teal", seasonalThemesEnabled: true });
    bridgeState.refreshThemes.mockRejectedValue(new Error("offline"));
    await mount(savedSettings);
    expect(current?.content).not.toBeNull();
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");

    await act(async () => { await current?.refreshThemes(); });

    expect(current?.refreshing).toBe(false);
    expect(current?.status.error).toBe("Themes couldn't be refreshed. Your current appearance is still available.");
    expect(current?.content).not.toBeNull();
    expect(document.documentElement.dataset.seasonalTheme).toBe("october");
    expect(savedSettings).toMatchObject({ theme: "dark", accent: "teal" });
    expect(bridgeState.saveSettings).not.toHaveBeenCalled();
  });
});
