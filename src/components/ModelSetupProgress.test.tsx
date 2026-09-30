import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ModelSetupProgress } from "./ModelSetupProgress";

describe("ModelSetupProgress", () => {
  it("renders the canonical Parakeet setup kind emitted by Rust", () => {
    const markup = renderToStaticMarkup(
      <ModelSetupProgress
        setup={{
          kind: "model-parakeet",
          variant: null,
          state: "downloading",
          downloadedBytes: 416 * 1024 ** 2,
          totalBytes: 1024 ** 3,
          error: null,
        }}
      />,
    );

    expect(markup).toContain("Downloading Parakeet TDT");
    expect(markup).toContain("416 MiB of 1.0 GiB · 41%");
  });
});
