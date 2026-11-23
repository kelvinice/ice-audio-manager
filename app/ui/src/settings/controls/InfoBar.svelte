<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    tone: 'info' | 'warning' | 'error';
    title: string;
    text?: string;
    ondismiss?: () => void;
  }

  let { tone, title, text, ondismiss }: Props = $props();
  const glyph = $derived(tone === 'error' ? 'error' : tone === 'warning' ? 'warning' : 'info');
</script>

<div class="bar {tone}" role={tone === 'info' ? 'status' : 'alert'}>
  <span class="glyph"><Icon name={glyph} size={16} /></span>
  <p><strong>{title}</strong>{#if text}&ensp;{text}{/if}</p>
  {#if ondismiss}
    <button type="button" aria-label="Dismiss" onclick={ondismiss}><Icon name="close" size={12} /></button>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 12px 12px 16px;
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
  }

  .info {
    background: var(--ice-set-info-bg);
  }

  .warning {
    background: var(--ice-set-warn-bg);
  }

  .error {
    background: var(--ice-set-danger-bg);
  }

  .glyph {
    margin-top: 2px;
  }

  .info .glyph {
    color: var(--ice-set-accent);
  }

  .warning .glyph {
    color: var(--ice-set-warn);
  }

  .error .glyph {
    color: var(--ice-set-danger);
  }

  p {
    flex: 1;
    margin: 0;
  }

  strong {
    font-weight: 600;
  }

  button {
    width: 28px;
    height: 28px;
    margin: -4px 0;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--ice-set-radius-sm);
    background: none;
    cursor: pointer;
  }

  button:hover {
    background: var(--ice-set-control-hover);
  }
</style>
