<script lang="ts">
  // D-27 hibernate: a plain notch for meetings and screen sharing. No track, art, lyrics or values; only a small dot
  // while the microphone is muted. Twin: `rasterize_plain_frame` in crates/ice-win/src/native_dot.rs.
  interface Props {
    micMuted: boolean;
    /** The small `dot` idle style centres the mic dot; the mini pill puts it at the right end. */
    centred: boolean;
  }

  let { micMuted, centred }: Props = $props();
</script>

<div class="plain" class:centred aria-label={micMuted ? 'Notch hibernating. Microphone muted' : 'Notch hibernating'}>
  {#if micMuted}
    <span class="mic-dot" aria-hidden="true"></span>
  {/if}
</div>

<style>
  .plain {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .plain.centred {
    justify-content: center;
  }

  .mic-dot {
    width: 5px;
    height: 5px;
    margin-right: -2px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-hibernate-dot);
  }

  .plain.centred .mic-dot {
    margin-right: 0;
  }
</style>
