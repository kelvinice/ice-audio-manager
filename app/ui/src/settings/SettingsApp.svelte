<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import type { SettingsClient } from '../lib/settings/client.svelte';
  import { SpringGroup } from '../lib/motion/spring.svelte';
  import { deviceLabel } from '../lib/settings/model';
  import DeviceIcon from '../notch/activities/DeviceIcon.svelte';
  import BrandMark from './controls/BrandMark.svelte';
  import Icon from './controls/Icon.svelte';
  import InfoBar from './controls/InfoBar.svelte';
  import type { IconName } from './icons';
  import AboutPage from './pages/AboutPage.svelte';
  import AppearancePage from './pages/AppearancePage.svelte';
  import DevicesPage from './pages/DevicesPage.svelte';
  import GeneralPage from './pages/GeneralPage.svelte';
  import NotchPage from './pages/NotchPage.svelte';
  import ShortcutsPage from './pages/ShortcutsPage.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  type PageId = 'general' | 'devices' | 'shortcuts' | 'notch' | 'appearance' | 'about';
  const PAGES: { id: PageId; title: string; icon: IconName | 'notch' }[] = [
    { id: 'general', title: 'General', icon: 'settings' },
    { id: 'devices', title: 'Devices', icon: 'speakers' },
    { id: 'shortcuts', title: 'Shortcuts', icon: 'keyboard' },
    { id: 'notch', title: 'Notch', icon: 'notch' },
    { id: 'appearance', title: 'Appearance', icon: 'color' },
    { id: 'about', title: 'About', icon: 'info' },
  ];
  const STORAGE_KEY = 'ice-settings-page';

  // A `#devices` link opens that page; otherwise the last page the user had open.
  function initialPage(): PageId {
    const wanted = location.hash.slice(1) || readStored();
    return PAGES.some((p) => p.id === wanted) ? (wanted as PageId) : 'general';
  }

  function readStored(): string {
    try {
      return localStorage.getItem(STORAGE_KEY) ?? '';
    } catch {
      return '';
    }
  }

  let page = $state<PageId>(initialPage());
  const current = $derived(PAGES.find((p) => p.id === page) ?? PAGES[0]);
  const reduceMotion = $derived(client.notch?.systemReduceMotion ?? false);
  const engine = $derived(client.engine);
  const starting = $derived(engine !== null && engine.revision === 0);
  const playing = $derived(engine?.playback.find((d) => d.isDefault) ?? null);

  // Navigation indicator: the notch's own spring module (code-practices §5), snapping when motion is off.
  const navItems: HTMLButtonElement[] = [];
  let indicatorY = $state(0);
  const springs = new SpringGroup({ y: 0 }, 'snappy', (v) => (indicatorY = v.y));

  function indicatorTarget(): number | null {
    const item = navItems[PAGES.findIndex((p) => p.id === page)];
    return item ? item.offsetTop + (item.offsetHeight - 16) / 2 : null;
  }

  async function select(id: PageId): Promise<void> {
    page = id;
    try {
      localStorage.setItem(STORAGE_KEY, id);
    } catch {
      // Storage can be unavailable; remembering the page is only a convenience.
    }
    history.replaceState(null, '', `#${id}`);
    await tick();
    const y = indicatorTarget();
    if (y === null) return;
    if (reduceMotion) springs.snapAll({ y });
    else springs.setTargets({ y });
  }

  $effect(() => {
    document.documentElement.dataset.reduceMotion = String(reduceMotion);
  });

  let stop: (() => void) | null = null;
  onMount(() => {
    const y = indicatorTarget();
    if (y !== null) springs.snapAll({ y });
    void client.start().then((cleanup) => {
      stop = cleanup;
      requestAnimationFrame(() => requestAnimationFrame(() => client.reportReady()));
    });
  });

  onDestroy(() => {
    stop?.();
    springs.setPaused(true);
  });
</script>

<div class="app">
  <nav aria-label="Settings pages">
    <div class="brand">
      <span class="logo"><BrandMark size={18} /></span>
      <span class="brand-text">
        <span class="name">Ice Audio Manager</span>
        <span class="sub">Settings</span>
      </span>
    </div>

    <div class="items">
      <span class="indicator" style:transform={`translateY(${indicatorY}px)`} aria-hidden="true"></span>
      {#each PAGES as item, index (item.id)}
        <button
          bind:this={navItems[index]}
          type="button"
          class:active={item.id === page}
          aria-current={item.id === page ? 'page' : undefined}
          onclick={() => select(item.id)}
        >
          {#if item.icon === 'notch'}
            <svg class="pill-icon" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
              <rect x="1.5" y="5" width="13" height="6" rx="3" />
            </svg>
          {:else}
            <Icon name={item.icon} size={16} />
          {/if}
          <span>{item.title}</span>
        </button>
      {/each}
    </div>

    {#if playing && engine}
      <div class="now" title="Current playback device">
        <DeviceIcon kind="auto" formFactor={playing.formFactor} size={16} />
        <span class="now-text">
          <span class="now-name">{deviceLabel(playing, engine.settings)}</span>
          <span class="now-sub">
            {engine.volume ? (engine.volume.muted ? 'Muted' : `Volume ${engine.volume.percent}%`) : 'Playing here'}
          </span>
        </span>
      </div>
    {/if}
  </nav>

  <main>
    <header>
      <h1>{current?.title}</h1>
      <span class="saved" class:show={client.saved} role="status">
        {#if client.saved}<Icon name="check" size={12} /> Saved{/if}
      </span>
    </header>

    <!-- Outside the scroll area, so an error is seen wherever the page is scrolled. -->
    {#if client.banner}
      <div class="banner pinned">
        <InfoBar
          tone={client.banner.tone}
          title={client.banner.title}
          text={client.banner.text}
          ondismiss={() => client.dismissBanner()}
        />
      </div>
    {/if}

    <div class="scroll">
      <div class="column">
        {#if starting && page !== 'about'}
          <div class="banner">
            <InfoBar tone="info" title="Windows audio is starting." text="Your devices and settings appear here as soon as it’s ready." />
          </div>
        {/if}
        {#key page}
          <div class="page">
            {#if page === 'general'}
              <GeneralPage {client} />
            {:else if page === 'devices'}
              <DevicesPage {client} />
            {:else if page === 'shortcuts'}
              <ShortcutsPage {client} />
            {:else if page === 'notch'}
              <NotchPage {client} />
            {:else if page === 'appearance'}
              <AppearancePage {client} />
            {:else}
              <AboutPage version={client.version} />
            {/if}
          </div>
        {/key}
      </div>
    </div>
  </main>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    font-family: var(--ice-font);
    user-select: none;
  }

  nav {
    flex: none;
    width: 256px;
    display: flex;
    flex-direction: column;
    padding: 12px 8px 12px 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px 20px;
  }

  .logo {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--ice-set-accent-fg);
    background: var(--ice-set-accent);
  }

  .brand-text {
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 600;
  }

  .sub {
    font-size: 12px;
    line-height: 16px;
    color: var(--ice-set-fg-muted);
  }

  .items {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .items button {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 36px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--ice-set-radius-sm);
    background: none;
    text-align: left;
    cursor: pointer;
    transition: background-color 100ms var(--ice-set-ease);
  }

  .items button:hover {
    background: var(--ice-set-nav-hover);
  }

  .items button.active {
    background: var(--ice-set-nav-active);
  }

  .indicator {
    position: absolute;
    top: 0;
    left: 0;
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: var(--ice-set-accent);
    z-index: 1;
  }

  .pill-icon rect {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
  }

  .now {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: var(--ice-set-radius);
    color: var(--ice-set-fg-muted);
  }

  .now-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .now-name {
    color: var(--ice-set-fg);
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .now-sub {
    font-size: 12px;
    line-height: 16px;
  }

  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    margin-top: 8px;
    border-top-left-radius: var(--ice-set-radius);
    border-top: 1px solid var(--ice-set-layer-border);
    border-left: 1px solid var(--ice-set-layer-border);
    background: var(--ice-set-layer);
  }

  header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 28px 40px 16px;
  }

  h1 {
    margin: 0;
    font-size: 28px;
    line-height: 36px;
    font-weight: 600;
  }

  .saved {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--ice-set-ok);
    opacity: 0;
    transition: opacity 200ms var(--ice-set-ease);
  }

  .saved.show {
    opacity: 1;
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .scroll::-webkit-scrollbar {
    width: 6px;
  }

  .scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .scroll::-webkit-scrollbar-thumb {
    border-radius: 3px;
    background: var(--ice-set-control-border);
  }

  .column {
    max-width: 1000px;
    padding: 0 40px 40px;
  }

  .banner {
    margin-bottom: 16px;
  }

  .banner.pinned {
    max-width: 1000px;
    padding: 0 40px;
  }

  .page {
    animation: enter 260ms var(--ice-set-ease);
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
</style>
