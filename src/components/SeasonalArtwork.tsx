import type { CSSProperties } from "react";
import type { ArtworkSlot, ThemeContent } from "../themes";
import type { DictationState } from "../types";

export function SeasonalArtwork({ theme, state = "idle", slot, inputLevel = 0, className = "" }: {
  theme: ThemeContent;
  state?: DictationState;
  slot?: ArtworkSlot;
  inputLevel?: number;
  className?: string;
}) {
  return <span className={`seasonal-art seasonal-art--${state} ${className}`} aria-hidden="true"
    style={{ "--voice-level": Math.max(0, Math.min(1, inputLevel)) } as CSSProperties}>
    <span className="seasonal-art__light" />
    <img src={theme.artwork[slot ?? state]} alt="" draggable={false} />
    <span className="seasonal-art__sparkle seasonal-art__sparkle--one" />
    <span className="seasonal-art__sparkle seasonal-art__sparkle--two" />
  </span>;
}

export function SeasonalAmbience({ theme }: { theme: ThemeContent }) {
  return <div className="seasonal-ambience" aria-hidden="true">
    {[0, 1, 2, 3].map((index) => <img key={index} src={theme.artwork.ambience} alt="" draggable={false}
      style={{ "--leaf-index": index } as CSSProperties} />)}
    <span className="seasonal-ambience__halo" />
  </div>;
}
