<script lang="ts">
  import type { IceConfigPatch, IceSettingsView } from "../lib/types";
  import Icon from "./Icon.svelte";

  /** Ice Audio → Notch: how Ice's notch looks and what it shows. Changes are saved to Ice's own settings. */
  interface Props {
    settings: IceSettingsView;
    /** Whether a running Ice can show or hide its notch right now. */
    canToggle: boolean;
    notchExpanded: boolean | null;
    busy: boolean;
    onPatch: (patch: IceConfigPatch) => void;
    onToggleNotch: () => void;
  }

  let { settings, canToggle, notchExpanded, busy, onPatch, onToggleNotch }: Props = $props();

  type Choice = { value: string; label: string };
  const PICKERS: { key: "theme" | "notchStyle" | "performanceProfile"; title: string; sub: string; options: Choice[] }[] = [
    {
      key: "theme",
      title: "Theme",
      sub: "The notch's colours",
      options: [
        { value: "ice", label: "Ice" },
        { value: "ice_light", label: "Ice Light" },
        { value: "midnight", label: "Midnight" },
        { value: "fluent", label: "Fluent" },
        { value: "minimal", label: "Minimal" },
      ],
    },
    {
      key: "notchStyle",
      title: "Style",
      sub: "Attached to the screen edge, or floating below it",
      options: [
        { value: "nook", label: "Edge Nook" },
        { value: "floating", label: "Floating Pill" },
      ],
    },
    {
      key: "performanceProfile",
      title: "Performance",
      sub: "Instant feels fastest; Lite uses the least memory",
      options: [
        { value: "instant", label: "Instant" },
        { value: "balanced", label: "Balanced" },
        { value: "lite", label: "Lite" },
      ],
    },
  ];

  const current = (key: (typeof PICKERS)[number]["key"]): string =>
    key === "theme" ? settings.appearance.theme : key === "notchStyle" ? settings.appearance.notchStyle : settings.performanceProfile;

  let toggles = $derived([
    { title: "Lyrics", sub: "Synced lyrics under the song", on: settings.lyrics.enabled, patch: "lyricsEnabled" },
    { title: "Spectrum", sub: "Live bars in the music pill", on: settings.visualizerSpectrum, patch: "visualizerSpectrum" },
    { title: "Moving artwork background", sub: "Cover colours drift behind the notch", on: settings.appearance.movingBackdrop, patch: "movingBackdrop" },
    { title: "Hibernate", sub: "Hide the notch until something happens", on: settings.appearance.hibernate, patch: "hibernate" },
  ] as const);
</script>

<section class="card card-glass" aria-labelledby="notch-title">
  <div class="head">
    <h2 id="notch-title">Notch</h2>
    <button
      class="btn-ghost"
      disabled={busy || !canToggle}
      title={canToggle ? "Show or hide Ice's notch" : "Start Ice to show its notch"}
      onclick={onToggleNotch}
    >
      <Icon name="panel-top" size={14} />
      <span>{notchExpanded ? "Hide notch" : "Show notch"}</span>
    </button>
  </div>

  {#each PICKERS as picker (picker.key)}
    <label class="row">
      <div class="text">
        <div class="title">{picker.title}</div>
        <div class="sub">{picker.sub}</div>
      </div>
      <select value={current(picker.key)} onchange={(e) => onPatch({ [picker.key]: (e.target as HTMLSelectElement).value } as IceConfigPatch)}>
        {#each picker.options as opt (opt.value)}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </label>
  {/each}

  {#each toggles as toggle (toggle.patch)}
    <label class="row">
      <div class="text">
        <div class="title">{toggle.title}</div>
        <div class="sub">{toggle.sub}</div>
      </div>
      <input
        type="checkbox"
        checked={toggle.on}
        onchange={(e) => onPatch({ [toggle.patch]: (e.target as HTMLInputElement).checked } as IceConfigPatch)}
      />
    </label>
  {/each}
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    padding: 20px 22px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
  }

  h2 {
    margin: 0;
    font-size: 16px;
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
    font-size: 13.5px;
    font-weight: 600;
  }

  .sub {
    margin-top: 2px;
    font-size: 12px;
    color: var(--p-text-dim);
  }

  select {
    flex-shrink: 0;
    width: 150px;
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--p-border);
    border-radius: 8px;
    background: var(--p-surface-solid);
    font-size: 12.5px;
  }
</style>
