<script lang="ts">
  import { audioEngine, playerStore } from "../lib/audio";
  import { switchIceDevice } from "../lib/ipc";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { DeviceSummary, Track } from "../lib/types";

  interface Props {
    proxyPort: number;
    playbackDevices: DeviceSummary[];
    onOpenNowPlaying: () => void;
    onOpenEq: () => void;
    onToggleLike: (track: Track) => void;
  }

  let {
    proxyPort,
    playbackDevices,
    onOpenNowPlaying,
    onOpenEq,
    onToggleLike,
  }: Props = $props();

  let showDevices = $state(false);

  function formatMs(ms: number): string {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }

  let activeDev = $derived(playbackDevices.find((d) => d.isDefault));
</script>

<footer class="player-bar">
  <div class="left">
    {#if $playerStore.currentTrack}
      {@const track = $playerStore.currentTrack}
      <button class="track-btn" onclick={onOpenNowPlaying}>
        <div class="art">
          {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
            <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
          {:else}
            <span>♫</span>
          {/if}
        </div>
        <div class="info">
          <div class="title">{track.title}</div>
          <div class="artist">{track.artist}</div>
        </div>
      </button>
      <button class="icon-btn" title="Like" onclick={() => onToggleLike(track)}>
        {track.liked ? "♥" : "♡"}
      </button>
    {:else}
      <div class="idle-msg">Pick a demo track, local folder, or YouTube Music song</div>
    {/if}
  </div>

  <div class="center">
    <div class="transport">
      <button class="icon-btn" class:lit={$playerStore.shuffle} onclick={() => audioEngine.toggleShuffle()}>
        ⇄
      </button>
      <button class="icon-btn" onclick={() => void audioEngine.previous()}>⏮</button>
      <button class="play-btn" onclick={() => void audioEngine.togglePlayPause()}>
        {$playerStore.loading ? "…" : $playerStore.playing ? "❚❚" : "▶"}
      </button>
      <button class="icon-btn" onclick={() => void audioEngine.next()}>⏭</button>
      <button class="icon-btn" class:lit={$playerStore.repeat !== "off"} onclick={() => audioEngine.cycleRepeat()}>
        {$playerStore.repeat === "one" ? "↻1" : "↻"}
      </button>
    </div>

    <div class="seek-row">
      <span>{formatMs($playerStore.currentTimeMs)}</span>
      <input
        type="range"
        min="0"
        max={Math.max(1000, $playerStore.durationMs)}
        value={$playerStore.currentTimeMs}
        oninput={(e) => audioEngine.seekMs(Number((e.target as HTMLInputElement).value))}
      />
      <span>{formatMs($playerStore.durationMs)}</span>
    </div>
  </div>

  <div class="right">
    {#if playbackDevices.length > 0}
      <div class="dev-picker-wrap">
        <button class="btn-chip" onclick={() => (showDevices = !showDevices)}>
          🎧 {activeDev ? activeDev.name : "Audio Output"}
        </button>
        {#if showDevices}
          <div class="dev-popover card-glass">
            <div class="pop-head">Switch Output (Ice Engine)</div>
            {#each playbackDevices as dev (dev.id)}
              <button
                class:active={dev.isDefault}
                onclick={() => {
                  void switchIceDevice(dev.id);
                  showDevices = false;
                }}
              >
                <span>{dev.name}</span>
                {#if dev.isDefault}<span>✓</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <button class="btn-chip" onclick={onOpenEq}>
      EQ{$playerStore.eq.enabled ? "" : " (Off)"}
    </button>
    <button class="btn-chip" onclick={onOpenNowPlaying}>Lyrics / Canvas</button>

    <div class="vol-wrap">
      <button class="icon-btn" onclick={() => audioEngine.toggleMute()}>
        {$playerStore.muted || $playerStore.volume === 0 ? "🔇" : "🔊"}
      </button>
      <input
        type="range"
        min="0"
        max="1"
        step="0.02"
        value={$playerStore.muted ? 0 : $playerStore.volume}
        oninput={(e) => audioEngine.setVolume(Number((e.target as HTMLInputElement).value))}
      />
    </div>
  </div>
</footer>

<style>
  .player-bar {
    height: 82px;
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr 1.2fr 1fr;
    align-items: center;
    gap: 16px;
    padding: 0 18px;
    background: rgba(11, 14, 24, 0.88);
    border-top: 1px solid var(--p-border);
    backdrop-filter: blur(26px);
    z-index: 10;
  }

  .left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .track-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    text-align: left;
  }

  .art {
    width: 50px;
    height: 50px;
    border-radius: 10px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.07);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .info {
    min-width: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .artist {
    font-size: 12px;
    color: var(--p-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .idle-msg {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .transport {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .icon-btn {
    font-size: 15px;
    color: var(--p-text-muted);
    padding: 4px 6px;
  }

  .icon-btn:hover,
  .icon-btn.lit {
    color: var(--p-accent-bright);
  }

  .play-btn {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--p-accent);
    color: #080b12;
    font-size: 13px;
    font-weight: 700;
  }

  .seek-row {
    width: 100%;
    max-width: 440px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--p-text-dim);
  }

  .seek-row input {
    flex: 1;
    accent-color: var(--p-accent);
    height: 4px;
  }

  .right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-chip {
    padding: 6px 10px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid var(--p-border);
    max-width: 155px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .btn-chip:hover {
    background: rgba(255, 255, 255, 0.13);
  }

  .dev-picker-wrap {
    position: relative;
  }

  .dev-popover {
    position: absolute;
    bottom: 40px;
    right: 0;
    width: 260px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: rgba(16, 20, 34, 0.96);
    z-index: 30;
  }

  .pop-head {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--p-text-dim);
    padding: 4px 8px;
  }

  .dev-popover button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    border-radius: 8px;
    font-size: 12px;
    text-align: left;
  }

  .dev-popover button:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .dev-popover button.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .vol-wrap {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 96px;
  }

  .vol-wrap input {
    width: 68px;
    accent-color: var(--p-accent);
  }
</style>
