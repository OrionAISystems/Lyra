// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import AudioEngine from "./AudioEngine";
import { defaultSettings } from "../types";
import type { LyraState } from "../state/useLyra";

const start = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../bridge", () => ({
  isTauri: true, startSetupArtifact: start, cancelSetupArtifact: vi.fn(),
  getBenchmarkResult: async () => null,
  listenToBenchmarkEvents: async () => [], listenToMicrophoneTestEvents: async () => [],
  stopMicrophoneTest: async () => {}, startMicrophoneTest: vi.fn(), runBenchmark: vi.fn(),
}));

it("offers CPU on a fresh non-NVIDIA device and sends the explicitly chosen runtime", async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  const state = {
    settings: defaultSettings, updateSettings: vi.fn(), devices: [],
    session: { state: "idle", inputLevel: 0 }, setup: { state: "idle" },
    runtime: { runtimeVariant: null, runtimeVerified: false, gpuName: null },
    modelRuntime: { state: "unloaded" },
    artifacts: ["cpu", "vulkan", "cuda"].map((variant) => ({ kind: "runtime", variant, sizeBytes: 1000 })),
  } as unknown as LyraState;
  try {
    await act(async () => { root.render(<AudioEngine state={state} />); });
    const runtime = container.querySelector<HTMLButtonElement>('[aria-label="Runtime"]')!;
    expect(runtime.textContent).toContain("CPU");
    await act(async () => { runtime.click(); });
    const options = [...container.querySelectorAll<HTMLButtonElement>('[role="option"]')];
    expect(options.map((option) => option.textContent)).toEqual(expect.arrayContaining([
      "CPU · no GPU required", "Vulkan · compatible GPU", "CUDA · NVIDIA GPU",
    ]));
    await act(async () => { options.find((option) => option.textContent?.startsWith("CPU"))!.click(); });
    const install = [...container.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent === "Install & use")!;
    await act(async () => { install.click(); });
    expect(start).toHaveBeenLastCalledWith("runtime", "cpu");
    await act(async () => { runtime.click(); });
    await act(async () => { [...container.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((option) => option.textContent?.startsWith("Vulkan"))!.click(); });
    await act(async () => { install.click(); });
    expect(start).toHaveBeenLastCalledWith("runtime", "vulkan");
    await act(async () => { root.render(<AudioEngine state={{ ...state, setup: { ...state.setup, state: "downloading", kind: "runtime", variant: "vulkan" } }} />); });
    expect(runtime.disabled).toBe(true);
    expect(install.disabled).toBe(true);
    expect(container.textContent).toContain("Cancel runtime setup");
  } finally {
    await act(async () => { root.unmount(); });
    container.remove();
  }
});
