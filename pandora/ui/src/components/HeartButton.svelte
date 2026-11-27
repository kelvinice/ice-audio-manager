<script lang="ts">
  /**
   * The favourite toggle used everywhere a song can be liked. Liking pops the heart (springy overshoot as it fills)
   * and bursts a ring and a few sparks around it; unliking gives a small press-in. CSS animations only, replayed by
   * re-keying the burst, and none with reduced motion (app.css stops animations).
   */
  interface Props {
    liked: boolean;
    size?: number;
    /** Extra classes from app.css for the button (`btn-icon sm`, ...). */
    buttonClass?: string;
    /** Keep the outline visible even when not hovered (rows hide unliked hearts until hover). */
    keep?: boolean;
    onToggle: () => void;
  }

  let { liked, size = 16, buttonClass = "btn-icon sm", keep = false, onToggle }: Props = $props();

  /** Bumped on each like so the burst elements remount and their animations play again. */
  let burst = $state(0);
  let unlikedAt = $state(0);
  let wasLiked: boolean | null = null;

  $effect(() => {
    const now = liked;
    if (wasLiked === false && now) burst += 1;
    if (wasLiked === true && !now) unlikedAt += 1;
    wasLiked = now;
  });

  const SPARKS = [0, 1, 2, 3, 4, 5, 6];
</script>

<button
  class="{buttonClass} heart"
  class:liked
  class:keep={keep || liked}
  title={liked ? "Remove from Favorites" : "Add to Favorites"}
  aria-label={liked ? "Remove from Favorites" : "Add to Favorites"}
  aria-pressed={liked}
  onclick={(e) => {
    e.stopPropagation();
    onToggle();
  }}
>
  {#key `${burst}-${unlikedAt}`}
    <span class="glyph" class:pop={burst > 0 && liked} class:press={unlikedAt > 0 && !liked} style:--s={`${size}px`}>
      <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
        <path
          d="M12 20.6c-.28 0-.55-.1-.76-.28C6.6 16.3 2.6 12.9 2.6 8.7 2.6 5.8 4.85 3.6 7.6 3.6c1.75 0 3.3.86 4.4 2.3 1.1-1.44 2.65-2.3 4.4-2.3 2.75 0 5 2.2 5 5.1 0 4.2-4 7.6-8.64 11.62-.21.18-.48.28-.76.28Z"
        />
      </svg>
      {#if burst > 0 && liked}
        <span class="ring" aria-hidden="true"></span>
        {#each SPARKS as i (i)}
          <span class="spark" style:--a={`${(360 / SPARKS.length) * i - 90}deg`} aria-hidden="true"></span>
        {/each}
      {/if}
    </span>
  {/key}
</button>

<style>
  .heart {
    color: var(--p-text-muted);
  }

  .heart.liked {
    color: var(--p-love);
  }

  .heart.liked:hover {
    color: var(--p-love);
    filter: brightness(1.12);
  }

  .glyph {
    position: relative;
    display: inline-grid;
    place-items: center;
    width: var(--s);
    height: var(--s);
  }

  svg {
    display: block;
    overflow: visible;
  }

  path {
    fill: transparent;
    stroke: currentColor;
    stroke-width: 1.9;
    stroke-linejoin: round;
    transition: fill 0.18s ease;
  }

  .liked path {
    fill: currentColor;
  }

  /* Like: squash, overshoot, settle. */
  .pop svg {
    animation: pop 0.55s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .press svg {
    animation: press 0.25s ease;
  }

  .ring {
    position: absolute;
    inset: -40%;
    border-radius: 50%;
    border: 2px solid var(--p-love);
    pointer-events: none;
    animation: ring 0.5s ease-out both;
  }

  .spark {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 4px;
    height: 4px;
    margin: -2px 0 0 -2px;
    border-radius: 50%;
    background: var(--p-love);
    pointer-events: none;
    animation: spark 0.55s cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  .spark:nth-child(odd) {
    background: var(--p-accent-bright);
  }

  @keyframes pop {
    0% {
      scale: 0.55;
    }
    45% {
      scale: 1.28;
    }
    100% {
      scale: 1;
    }
  }

  @keyframes press {
    50% {
      scale: 0.8;
    }
  }

  @keyframes ring {
    from {
      opacity: 0.9;
      scale: 0.3;
    }
    to {
      opacity: 0;
      scale: 1;
    }
  }

  /* One transform so each spark flies out along its own angle. */
  @keyframes spark {
    0% {
      opacity: 1;
      transform: rotate(var(--a)) translateX(0) scale(1);
    }
    100% {
      opacity: 0;
      transform: rotate(var(--a)) translateX(calc(var(--s) * 0.95)) scale(0.4);
    }
  }
</style>
