<script lang="ts">
  export type IconKind =
    | 'auto'
    | 'headphones'
    | 'speakers'
    | 'hdmi'
    | 'speaker'
    | 'speaker-muted'
    | 'mic'
    | 'mic-muted'
    | 'plug-in'
    | 'plug-out';

  interface Props {
    kind?: IconKind;
    formFactor?: number | null;
    size?: number;
  }

  let { kind = 'auto', formFactor = null, size = 15 }: Props = $props();

  const resolved = $derived.by((): Exclude<IconKind, 'auto'> => {
    if (kind !== 'auto') return kind;
    switch (formFactor) {
      case 3:
      case 5:
        return 'headphones';
      case 4:
        return 'mic';
      case 9:
        return 'hdmi';
      case 1:
        return 'speakers';
      default:
        return 'speaker';
    }
  });
</script>

<svg
  class="ice-icon"
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if resolved === 'headphones'}
    <path d="M3 9.5V8a5 5 0 0 1 10 0v1.5" />
    <rect x="2.2" y="9" width="2.6" height="4.4" rx="1.2" />
    <rect x="11.2" y="9" width="2.6" height="4.4" rx="1.2" />
  {:else if resolved === 'speakers'}
    <rect x="3.5" y="2" width="9" height="12" rx="1.8" />
    <circle cx="8" cy="5.2" r="1" />
    <circle cx="8" cy="10.2" r="2.2" />
  {:else if resolved === 'hdmi'}
    <rect x="2" y="3" width="12" height="8" rx="1.5" />
    <path d="M5.5 13.5h5M8 11v2.5" />
  {:else if resolved === 'speaker'}
    <path d="M2.5 6.3H5l3.5-2.8v9L5 9.7H2.5V6.3Z" />
    <path d="M11 5.6a3.4 3.4 0 0 1 0 4.8" />
    <path d="M12.9 3.8a6 6 0 0 1 0 8.4" />
  {:else if resolved === 'speaker-muted'}
    <path d="M2.5 6.3H5l3.5-2.8v9L5 9.7H2.5V6.3Z" />
    <path d="M11 6l3.2 4M14.2 6L11 10" />
  {:else if resolved === 'mic'}
    <rect x="5.8" y="2.2" width="4.4" height="6.8" rx="2.2" />
    <path d="M3.6 7.8a4.4 4.4 0 0 0 8.8 0M8 12.2v1.8M5.8 14h4.4" />
  {:else if resolved === 'mic-muted'}
    <rect x="5.8" y="2.2" width="4.4" height="6.8" rx="2.2" />
    <path d="M3.6 7.8a4.4 4.4 0 0 0 8.8 0M8 12.2v1.8M5.8 14h4.4" />
    <path d="M2.4 13.6L13.6 2.4" />
  {:else if resolved === 'plug-in'}
    <path d="M6 2.5v3M10 2.5v3M4.2 5.5h7.6v2.2a3.8 3.8 0 0 1-7.6 0V5.5Z" />
    <path d="M8 11.5v2.5" />
  {:else}
    <path d="M6 2.5v2.5M10 2.5v2.5M4.2 5h7.6v2a3.8 3.8 0 0 1-7.6 0V5Z" />
    <path d="M3 13L13 3" />
  {/if}
</svg>

<style>
  .ice-icon {
    display: block;
    flex-shrink: 0;
  }
</style>
