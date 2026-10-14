// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Appearance from "./Appearance";
import { bundledOctober, type ThemeContent, type ThemeSource, type ThemeStatus } from "../themes";
import { defaultSettings } from "../types";
import type { LyraState } from "../state/useLyra";

const bridgeState = vi.hoisted(() => ({
  isTauri: true,
  getThemeContent: vi.fn(),
}));

vi.mock("../bridge", () => ({
  get isTauri() { return bridgeState.isTauri; },
  getThemeContent: bridgeState.getThemeContent,
}));

const novemberTheme: ThemeContent = {
  ...bundledOctober,
  id: "november",
  name: "November",
  description: "A November test theme.",
  revision: 1,
  startMonthDay: "11-01",
  endMonthDay: "12-01",
};

function availableTheme(theme: ThemeContent, source: ThemeSource) {
  return {
    id: theme.id,
    name: theme.name,
    revision: theme.revision,
    startMonthDay: theme.startMonthDay,
    endMonthDay: theme.endMonthDay,
    source,
  };
}

function themeStatus(content: ThemeContent | null, source: ThemeSource | null, catalogRevision: number): ThemeStatus {
  return {
    enabled: true,
    motionEnabled: true,
    state: content ? "active" : "upcoming",
    themeId: content?.id ?? null,
    catalogRevision,
    nextChangeAt: null,
    refreshedAt: "2026-10-15T12:00:00Z",
    error: null,
    source,
    availableThemes: [availableTheme(bundledOctober, "bundled"), availableTheme(novemberTheme, source ?? "remote")],
    content,
  };
}

function appState(status: ThemeStatus): LyraState {
  return {
    settings: { ...defaultSettings, theme: "dark", seasonalThemesEnabled: true },
    seasonal: {
      status,
      content: status.content,
      motionEnabled: true,
      reducedMotion: false,
      refreshing: false,
      refreshThemes: vi.fn(),
    },
  } as unknown as LyraState;
}

function localWindowDates(startMonthDay: string, endMonthDay: string) {
  const first = new Date(`2000-${startMonthDay}T12:00:00Z`);
  const last = new Date(`2000-${endMonthDay}T12:00:00Z`);
  last.setUTCDate(last.getUTCDate() - 1);
  const format = (date: Date) => date.toLocaleDateString(undefined, { month: "long", day: "numeric", timeZone: "UTC" });
  return `${format(first)} – ${format(last)}`;
}

async function flushMicrotasks() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

let root: Root | undefined;
let container: HTMLDivElement;

async function render(state: LyraState) {
  await act(async () => {
    root = createRoot(container);
    root.render(<Appearance state={state} />);
    await flushMicrotasks();
  });
}

function seasonalHeading() {
  return container.querySelector(".seasonal-settings__heading p")?.textContent ?? "";
}

function deliverySource() {
  return container.querySelector(".seasonal-settings__delivery strong")?.textContent ?? "";
}

function previewHeading() {
  return container.querySelector(".seasonal-preview__copy h3")?.textContent ?? "";
}

async function chooseThemePreview(themeName: string) {
  const trigger = container.querySelector<HTMLButtonElement>('button[aria-label="Theme preview"]');
  if (!trigger) throw new Error("Theme preview control was not rendered");
  await act(async () => { trigger.click(); });
  const option = [...container.querySelectorAll<HTMLButtonElement>('[role="option"]')]
    .find((candidate) => candidate.textContent?.includes(themeName));
  if (!option) throw new Error(`Theme preview option ${themeName} was not rendered`);
  await act(async () => { option.click(); await flushMicrotasks(); });
}

beforeEach(() => {
  bridgeState.isTauri = true;
  bridgeState.getThemeContent.mockReset().mockImplementation(async (id: string) => {
    if (id === "november") return novemberTheme;
    if (id === "october") return bundledOctober;
    return null;
  });
  container = document.createElement("div");
  document.body.append(container);
  (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
});

afterEach(async () => {
  if (root) await act(async () => { root?.unmount(); });
  root = undefined;
  container.remove();
});

describe("Appearance seasonal preview", () => {
  it("updates bar visibility and reflects settings changes from other surfaces", async () => {
    const state = appState(themeStatus(null, null, 1));
    state.updateSettings = vi.fn();
    await render(state);
    const toggle = container.querySelector<HTMLButtonElement>('button[aria-label="Show dictation bar"]')!;
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    await act(async () => { toggle.click(); });
    expect(state.updateSettings).toHaveBeenCalledWith({ showBar: false });
    await act(async () => {
      root?.render(<Appearance state={{ ...state, settings: { ...state.settings, showBar: false } }} />);
    });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    await act(async () => { toggle.click(); });
    expect(state.updateSettings).toHaveBeenLastCalledWith({ showBar: true });
  });

  it("switches to newly active November content and shows its live dates and remote source", async () => {
    await render(appState(themeStatus(null, null, 1)));
    expect(previewHeading()).toBe(bundledOctober.name);

    await act(async () => {
      root?.render(<Appearance state={appState(themeStatus(novemberTheme, "remote", 2))} />);
      await flushMicrotasks();
    });

    expect(previewHeading()).toBe("November");
    expect(seasonalHeading()).toContain("November");
    expect(seasonalHeading()).toContain(localWindowDates("11-01", "12-01"));
    expect(deliverySource()).toBe("Verified remote theme");
  });

  it.each([
    ["remote", "Verified remote theme"],
    ["cache", "Saved for offline use"],
  ] as const)("keeps the active November heading and %s delivery source while previewing October", async (source, deliveryLabel) => {
    await render(appState(themeStatus(novemberTheme, source, 2)));
    expect(previewHeading()).toBe("November");
    await chooseThemePreview(bundledOctober.name);

    expect(previewHeading()).toBe(bundledOctober.name);
    expect(seasonalHeading()).toContain("November");
    expect(seasonalHeading()).toContain(localWindowDates("11-01", "12-01"));
    expect(deliverySource()).toBe(deliveryLabel);

    const newerNovember = { ...novemberTheme, revision: 2, description: "A refreshed November test theme." };
    await act(async () => {
      root?.render(<Appearance state={appState(themeStatus(newerNovember, source, 3))} />);
      await flushMicrotasks();
    });

    expect(previewHeading()).toBe(bundledOctober.name);
    expect(seasonalHeading()).toContain("November");
    expect(seasonalHeading()).toContain(localWindowDates("11-01", "12-01"));
    expect(deliverySource()).toBe(deliveryLabel);
  });
});
