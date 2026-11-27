<script lang="ts">
  import type { ExperimentalSettings, PandoraSettings } from "../lib/types";
  import Icon from "./Icon.svelte";

  /** Settings → Experimental: features still being tried out, each off until switched on. */
  interface Props {
    settings: PandoraSettings;
    onUpdateSettings: (patch: Partial<PandoraSettings>) => void;
  }

  let { settings, onUpdateSettings }: Props = $props();

  const FEATURES: { key: keyof ExperimentalSettings; title: string; sub: string }[] = [
    {
      key: "smoothTransitions",
      title: "Smooth transitions",
      sub: "Songs fade in, fade out over their last 3 seconds, and skipping dips out instead of cutting.",
    },
    {
      key: "beatPulse",
      title: "Beat pulse",
      sub: "The cover in Now Playing pulses with the kick drum.",
    },
    {
      key: "gentleSleep",
      title: "Gentle sleep timer",
      sub: "The sleep timer lowers the music over its last 30 seconds instead of stopping it at once.",
    },
  ];

  function set(key: keyof ExperimentalSettings, on: boolean) {
    onUpdateSettings({ experimental: { ...settings.experimental, [key]: on } });
  }
</script>

<section class="panel card-glass">
  <div class="head">
    <span class="badge"><Icon name="flask" size={18} /></span>
    <div>
      <h2>Experimental</h2>
      <p>Early features. They may change or go away, and some may not work perfectly yet.</p>
    </div>
  </div>

  {#each FEATURES as feature (feature.key)}
    <label class="row">
      <div class="text">
        <div class="title">{feature.title}</div>
        <div class="sub">{feature.sub}</div>
      </div>
      <input
        type="checkbox"
        checked={settings.experimental[feature.key]}
        onchange={(e) => set(feature.key, (e.target as HTMLInputElement).checked)}
      />
    </label>
  {/each}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    padding: 18px 20px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    margin-bottom: 12px;
  }

  .badge {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  h2 {
    margin: 0;
    font-size: 16px;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 12.5px;
    color: var(--p-text-dim);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 0;
    border-top: 1px solid var(--p-border);
    cursor: pointer;
  }

  .text {
    min-width: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
  }

  .sub {
    margin-top: 3px;
    font-size: 12px;
    color: var(--p-text-dim);
  }
</style>
