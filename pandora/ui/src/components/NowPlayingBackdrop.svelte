<script lang="ts">
  // Now Playing backdrop. Always: the cover, blurred and enlarged, under a tint that keeps text readable. With
  // `animated` (the sparkle button): soft orbs in the artwork's colours drift on slow circular paths over it, in the
  // spirit of the Ice notch's lava-lamp wash ("Calm"), the full lava lamp (`LavaLamp`, "Lively"), or both: the cover
  // under a fainter lava lamp, breathing with the bass together ("Blend", the default; setting `backdropMotion`). Motion is transform-only, pauses with the song and is off for reduced motion (app.css stops
  // every animation then).
  import type { BackdropMotion } from "../lib/types";
  import LavaLamp from "./LavaLamp.svelte";

  interface Props {
    artworkUrl: string | null;
    animated: boolean;
    motion: BackdropMotion;
    playing: boolean;
  }

  let { artworkUrl, animated, motion, playing }: Props = $props();

  /**
   * One orb: its centre point (% of the page), how far it circles from it (vmin), size (vmax), which artwork colour,
   * how long a lap takes and which way. Different periods keep the pattern from visibly repeating.
   */
  const ORBS = [
    { x: 18, y: 22, r: 9, size: 62, color: "--art-primary", lap: 46, reverse: false },
    { x: 82, y: 30, r: 12, size: 54, color: "--art-secondary", lap: 58, reverse: true },
    { x: 60, y: 82, r: 10, size: 66, color: "--art-tertiary", lap: 52, reverse: false },
    { x: 30, y: 70, r: 14, size: 44, color: "--art-secondary", lap: 64, reverse: true },
    { x: 70, y: 52, r: 8, size: 40, color: "--art-primary", lap: 40, reverse: false },
  ] as const;
</script>

<div class="backdrop" aria-hidden="true">
  {#if artworkUrl && !(animated && motion === "blend")}
    {#key artworkUrl}
      <img class="cover" src={artworkUrl} alt="" />
    {/key}
  {/if}

  {#if animated && motion === "blend"}
    <!-- The lamp draws the cover itself, inside the field that breathes with the bass (no cover: Lively orbs). -->
    <LavaLamp {playing} cover={artworkUrl} />
  {:else if animated && motion === "lively"}
    <LavaLamp {playing} />
  {:else if animated}
    <div class="orbs" class:paused={!playing}>
      {#each ORBS as orb, i (i)}
        <div
          class="orbit"
          class:reverse={orb.reverse}
          style:left={`${orb.x}%`}
          style:top={`${orb.y}%`}
          style:animation-duration={`${orb.lap}s`}
          style:animation-delay={`${-orb.lap * 0.37 * i}s`}
        >
          <div
            class="orb"
            style:--r={`${orb.r}vmin`}
            style:width={`${orb.size}vmax`}
            style:background={`radial-gradient(closest-side, var(${orb.color}, var(--p-tone)) 0%, color-mix(in srgb, var(${orb.color}, var(--p-tone)) 55%, transparent) 45%, transparent 100%)`}
          ></div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="tint"></div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: -1;
    overflow: hidden;
    pointer-events: none;
    background: var(--p-bg);
  }

  .cover {
    position: absolute;
    inset: -10%;
    width: 120%;
    height: 120%;
    object-fit: cover;
    filter: blur(70px) saturate(1.5);
    opacity: 0.6;
    animation: fade-in 0.6s ease both;
  }

  .orbs {
    position: absolute;
    inset: 0;
    isolation: isolate;
  }

  /* A zero-size point that turns; its child sits `--r` away, so the orb circles the point. */
  .orbit {
    position: absolute;
    width: 0;
    height: 0;
    animation-name: lap;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
  }

  .orbit.reverse {
    animation-direction: reverse;
  }

  .orbs.paused .orbit,
  .orbs.paused .orb {
    animation-play-state: paused;
  }

  .orb {
    position: absolute;
    aspect-ratio: 1;
    border-radius: 50%;
    opacity: 0.55;
    mix-blend-mode: screen;
    translate: calc(-50% + var(--r)) -50%;
    animation: swell 17s ease-in-out infinite alternate;
  }

  :global([data-theme="daylight"]) .orb,
  :global([data-theme="iceLight"]) .orb {
    mix-blend-mode: multiply;
    opacity: 0.35;
  }

  /* Evens the colour out and keeps every line of lyrics readable over the brightest part of the cover. */
  .tint {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(ellipse at 30% 40%, transparent, color-mix(in srgb, var(--p-bg) 45%, transparent) 75%),
      color-mix(in srgb, var(--p-bg) 48%, transparent);
  }

  @keyframes lap {
    to {
      rotate: 360deg;
    }
  }

  @keyframes swell {
    from {
      scale: 0.9;
    }
    to {
      scale: 1.12;
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }
</style>
