<script lang="ts">
  import { audioEngine, playerStore, SHUFFLE_TITLES } from "../lib/audio";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { DeviceSummary, Track } from "../lib/types";
  import HeartButton from "./HeartButton.svelte";
  import Icon from "./Icon.svelte";
  import OutputPicker from "./OutputPicker.svelte";
  import VolumeControl from "./VolumeControl.svelte";
  import SeekBar from "./SeekBar.svelte";

  interface Props {
    proxyPort: number;
    playbackDevices: DeviceSummary[];
    likedIds: Set<string>;
    wavyProgress?: boolean;
    onOpenNowPlaying: () => void;
    onOpenEq: () => void;
    queueOpen: boolean;
    onToggleQueue: () => void;
    onToggleLike: (track: Track) => void;
    onAddToPlaylist: (track: Track) => void;
  }

  let {
    proxyPort,
    playbackDevices,
    likedIds,
    wavyProgress = true,
    onOpenNowPlaying,
    onOpenEq,
    queueOpen,
    onToggleQueue,
    onToggleLike,
    onAddToPlaylist,
  }: Props = $props();


</script>

<footer class="player-bar">
  <div class="left">
    {#if $playerStore.currentTrack}
      {@const track = $playerStore.currentTrack}
      {@const isLiked = likedIds.has(track.id)}
      <button class="track-btn" onclick={onOpenNowPlaying} title="Open Now Playing & Lyrics">
        <div class="art">
          {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
            <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
          {:else}
            <span class="art-empty"><Icon name="music" size={20} /></span>
          {/if}
          <span class="art-hover"><Icon name="maximize" size={15} /></span>
        </div>
        <div class="info">
          <div class="title">{track.title}</div>
          <div class="artist">{track.artist}</div>
        </div>
      </button>

      <div class="left-acts">
        <HeartButton liked={isLiked} size={17} buttonClass="btn-icon sm" keep onToggle={() => onToggleLike(track)} />
        <button
          class="dock-icon"
          title="Add to Playlist"
          aria-label="Add to Playlist"
          onclick={() => onAddToPlaylist(track)}
        >
          <Icon name="list-plus" size={16} />
        </button>
      </div>
    {:else}
      <div class="idle-state">
        <span class="idle-badge"><Icon name="music" size={15} /></span>
        <span class="idle-msg">Select a track to start listening</span>
      </div>
    {/if}
  </div>

  <div class="center">
    <div class="transport">
      <button
        class="dock-icon"
        class:lit={$playerStore.shuffle !== "off"}
        title={SHUFFLE_TITLES[$playerStore.shuffle]}
        aria-label={SHUFFLE_TITLES[$playerStore.shuffle]}
        onclick={() => audioEngine.cycleShuffle()}
      >
        <Icon name={$playerStore.shuffle === "magic" ? "shuffle-magic" : "shuffle"} size={16} />
        {#if $playerStore.shuffle !== "off"}<span class="dot"></span>{/if}
      </button>

      <button
        class="dock-icon"
        title="Previous track"
        aria-label="Previous track"
        onclick={() => void audioEngine.previous()}
      >
        <Icon name="skip-back" size={17} />
      </button>

      <button
        class="play-btn"
        title={$playerStore.playing ? "Pause" : "Play"}
        aria-label={$playerStore.playing ? "Pause" : "Play"}
        onclick={() => void audioEngine.togglePlayPause()}
      >
        {#if $playerStore.loading}
          <Icon name="loader" size={18} spin={true} />
        {:else}
          <Icon name={$playerStore.playing ? "pause" : "play"} size={18} />
        {/if}
      </button>

      <button
        class="dock-icon"
        title="Next track"
        aria-label="Next track"
        onclick={() => void audioEngine.next()}
      >
        <Icon name="skip-forward" size={17} />
      </button>

      <button
        class="dock-icon"
        class:lit={$playerStore.repeat !== "off"}
        title="Cycle repeat mode"
        aria-label="Cycle repeat mode"
        onclick={() => audioEngine.cycleRepeat()}
      >
        <Icon name={$playerStore.repeat === "one" ? "repeat-1" : "repeat"} size={16} />
        {#if $playerStore.repeat !== "off"}<span class="dot"></span>{/if}
      </button>
    </div>

    <div class="seek-row">
      <SeekBar
        positionMs={$playerStore.currentTimeMs}
        durationMs={$playerStore.durationMs}
        playing={$playerStore.playing}
        wavy={wavyProgress}
        onSeek={(ms) => audioEngine.seekMs(ms)}
      />
    </div>
  </div>

  <div class="right">
    <button
      class="dock-icon"
      class:lit={queueOpen}
      title={queueOpen ? "Hide queue" : "Show queue"}
      aria-label="Queue"
      aria-pressed={queueOpen}
      onclick={onToggleQueue}
    >
      <Icon name="queue" size={17} />
      {#if queueOpen}<span class="dot"></span>{/if}
    </button>

    <button
      class="dock-icon"
      title="Lyrics & Now Playing"
      aria-label="Lyrics and Now Playing"
      onclick={onOpenNowPlaying}
    >
      <Icon name="lyrics" size={17} />
    </button>

    <button
      class="dock-icon"
      class:lit={$playerStore.eq.enabled}
      title={$playerStore.eq.enabled ? `Equalizer (${$playerStore.eq.preset})` : "Equalizer (Off)"}
      aria-label="Equalizer and Audio Effects"
      onclick={onOpenEq}
    >
      <Icon name="sliders" size={17} />
      {#if $playerStore.eq.enabled}<span class="dot"></span>{/if}
    </button>

    {#if playbackDevices.length > 0}
      <OutputPicker devices={playbackDevices} />
    {/if}

    <VolumeControl />
  </div>
</footer>

<style>
  .player-bar {
    height: 84px;
    flex-shrink: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(300px, 1.35fr) minmax(0, 1fr);
    align-items: center;
    gap: 18px;
    padding: 0 20px;
    background: var(--p-rail);
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
    position: relative;
    width: 52px;
    height: 52px;
    border-radius: 11px;
    overflow: hidden;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .art-empty {
    color: var(--p-text-dim);
  }

  .art-hover {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(8, 11, 20, 0.56);
    color: #fff;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .track-btn:hover .art-hover {
    opacity: 1;
  }

  .info {
    min-width: 0;
  }

  .title {
    font-size: 13.5px;
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

  .left-acts {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: 2px;
  }

  .idle-state {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--p-text-dim);
  }

  .idle-badge {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--p-fill);
    display: grid;
    place-items: center;
  }

  .idle-msg {
    font-size: 12px;
  }

  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .transport {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .dock-icon {
    position: relative;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--p-text-muted);
    transition: background 0.15s ease, color 0.15s ease, transform 0.12s ease;
  }

  .dock-icon:hover {
    background: var(--p-fill);
    color: var(--p-text);
  }

  .dock-icon:active {
    transform: scale(0.91);
  }

  .dock-icon.lit {
    color: var(--p-accent-bright);
  }


  .dock-icon .dot {
    position: absolute;
    bottom: 3px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--p-accent-bright);
  }

  .play-btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--p-accent-bright), var(--p-accent));
    color: var(--p-on-accent);
    display: grid;
    place-items: center;
    box-shadow: 0 4px 16px var(--p-glow-1);
    transition: transform 0.14s ease, filter 0.14s ease;
  }

  .play-btn:hover {
    transform: scale(1.06);
    filter: brightness(1.06);
  }

  .play-btn:active {
    transform: scale(0.94);
  }

  .seek-row {
    width: 100%;
    max-width: 460px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }

</style>
