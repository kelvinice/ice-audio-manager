<script lang="ts">
  import type { IconName } from '../icons';
  import BrandMark from '../controls/BrandMark.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';

  interface Props {
    version: string | null;
  }

  let { version }: Props = $props();

  // The promises code-practices §1 makes, in the user's words.
  const promises: { icon: IconName; title: string; description: string }[] = [
    {
      icon: 'keyboard',
      title: 'No keyboard hooks',
      description: 'Shortcuts are registered with Windows, so the app is told only about the ones you set.',
    },
    {
      icon: 'globe',
      title: 'No telemetry',
      description: 'No analytics, no crash reports and no background network traffic.',
    },
    {
      icon: 'lock',
      title: 'No administrator rights',
      description: 'It runs as you and writes only to your own app data.',
    },
    {
      icon: 'page',
      title: 'Settings you can read',
      description: 'Saved as plain text in %APPDATA%\\IceAudioManager\\config.toml. Edits there apply right away.',
    },
  ];
</script>

<div class="hero">
  <span class="mark"><BrandMark size={34} /></span>
  <div>
    <h2>Ice Audio Manager</h2>
    <p>{version ? `Version ${version}` : 'Version unknown'} · Free and open source (MIT)</p>
  </div>
</div>

<Section title="Built to be trusted">
  {#each promises as p (p.title)}
    <SettingCard icon={p.icon} title={p.title} description={p.description} />
  {/each}
</Section>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-bottom: 28px;
    padding: 20px;
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
    background: var(--ice-set-card);
  }

  .mark {
    width: 64px;
    height: 64px;
    display: grid;
    place-items: center;
    border-radius: 16px;
    color: var(--ice-set-accent-fg);
    background: var(--ice-set-accent);
  }

  h2 {
    margin: 0;
    font-size: 20px;
    line-height: 28px;
    font-weight: 600;
  }

  p {
    margin: 2px 0 0;
    color: var(--ice-set-fg-muted);
  }
</style>
