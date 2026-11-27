<script lang="ts">
  // Experimental notch widget: the user's buttons in their order (`lib/notch/widget.ts`), drawn as the Floating side
  // rail (`rail`, 24 px, vertical) or inside the Edge Nook toolbar capsule (`toolbar`, 22 px, horizontal).
  import type { WidgetButton } from '../../lib/ipc/engine';
  import { WIDGET_BUTTONS, pandoraTitle, type WidgetBinding } from '../../lib/notch/widget';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props extends WidgetBinding {
    variant: 'rail' | 'toolbar';
  }

  let {
    buttons,
    variant,
    pinned,
    micMuted,
    lyricsOn,
    lyricsAvailable,
    callModeOn,
    pandora,
    instant = false,
    onTogglePin,
    onToggleMicMute,
    onToggleLyrics,
    onHibernate,
    onToggleCallMode,
    onPandora,
    onOpenSettings,
  }: Props = $props();

  const size = $derived(variant === 'rail' ? 13 : 12);

  interface Look {
    title: string;
    on?: boolean;
    danger?: boolean;
    busy?: boolean;
    disabled?: boolean;
    icon?: 'mic-muted';
    run: () => void;
  }

  function look(b: WidgetButton): Look {
    switch (b) {
      case 'pin':
        return { title: pinned ? 'Unpin notch' : 'Pin notch open', on: pinned, run: onTogglePin };
      case 'mic':
        return {
          title: micMuted ? 'Microphone is off: turn on' : 'Microphone is on: turn off',
          danger: micMuted,
          icon: micMuted ? 'mic-muted' : undefined,
          run: onToggleMicMute,
        };
      case 'lyrics':
        return {
          title: lyricsAvailable ? (lyricsOn ? 'Hide lyrics' : 'Show lyrics') : 'No lyrics for this song',
          on: lyricsOn && lyricsAvailable,
          disabled: !lyricsAvailable,
          run: onToggleLyrics,
        };
      case 'hibernate':
        return { title: 'Hibernate: a plain notch. Wake it from the tray icon or Settings.', run: onHibernate };
      case 'call_mode':
        return { title: callModeOn ? 'Call mode is on: turn off' : 'Call mode is off: turn on', on: callModeOn, run: onToggleCallMode };
      case 'pandora_recognize':
        return {
          title: pandoraTitle(b, pandora),
          busy: pandora?.busy === 'recognize',
          disabled: pandora?.busy != null,
          run: () => onPandora('recognize'),
        };
      case 'pandora_open':
        return {
          title: pandoraTitle(b, pandora),
          busy: pandora?.busy === 'open',
          disabled: pandora?.busy != null,
          run: () => onPandora('open'),
        };
      case 'settings':
        return { title: 'Settings', run: onOpenSettings };
    }
  }
</script>

{#each buttons as b (b)}
  {@const l = look(b)}
  <button
    class="wb"
    class:rail={variant === 'rail'}
    class:on={l.on}
    class:danger={l.danger}
    class:busy={l.busy && !instant}
    type="button"
    title={l.title}
    aria-label={l.title}
    aria-pressed={l.on ?? (l.danger ? true : undefined)}
    aria-busy={l.busy || undefined}
    disabled={l.disabled}
    onclick={l.run}
  >
    <DeviceIcon kind={l.icon ?? WIDGET_BUTTONS[b].icon} {size} />
  </button>
{/each}

<style>
  .wb {
    appearance: none;
    width: 22px;
    height: 22px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg-muted);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .wb.rail {
    width: 24px;
    height: 24px;
  }

  .wb:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .wb:focus-visible {
    outline: 2px solid var(--ice-accent);
    outline-offset: 1px;
  }

  .wb.on {
    background: var(--ice-control-bg-active);
    color: var(--ice-accent-strong);
  }

  .wb.rail.on {
    border-color: var(--ice-accent);
  }

  .wb.danger {
    background: color-mix(in srgb, var(--ice-danger) 16%, transparent);
    color: var(--ice-danger);
  }

  .wb:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .wb:disabled:hover {
    background: transparent;
    color: var(--ice-fg-muted);
  }

  /* Only while a Pandora job runs; `instant` (preset off / reduced motion) drops the class. */
  .wb.busy,
  .wb.busy:disabled {
    opacity: 1;
    color: var(--ice-accent-strong);
    animation: widget-busy 1.1s ease-in-out infinite;
  }

  @keyframes widget-busy {
    50% {
      opacity: 0.45;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wb.busy {
      animation: none;
    }
  }
</style>
