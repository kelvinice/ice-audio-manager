<script lang="ts">
  import { onMount } from "svelte";
  import { getLaunchAtSignIn, setLaunchAtSignIn } from "../lib/ipc";
  import type { LaunchAtSignIn } from "../lib/types";

  /** Settings → Playback row for Pandora's own Windows startup entry (like Ice's). Debug builds show it locked. */
  interface Props {
    onNotify: (msg: string) => void;
  }

  let { onNotify }: Props = $props();

  let launch = $state<LaunchAtSignIn | null>(null);
  let busy = $state(false);

  onMount(() => {
    void getLaunchAtSignIn()
      .then((next) => (launch = next))
      .catch(() => (launch = null));
  });

  async function toggle(on: boolean) {
    busy = true;
    try {
      launch = await setLaunchAtSignIn(on);
      onNotify(on ? "Pandora will start in the tray when you sign in" : "Pandora won’t start at sign-in");
    } catch (e) {
      onNotify(`Couldn’t change launch at sign-in: ${e instanceof Error ? e.message : String(e)}`);
      launch = await getLaunchAtSignIn().catch(() => launch);
    } finally {
      busy = false;
    }
  }

  let locked = $derived(!launch?.available || busy);
</script>

<label class="toggle-row" class:muted={!launch?.available}>
  <div>
    <div class="row-title">Launch at Sign-in</div>
    <div class="row-sub">
      {launch?.available ? "Start quietly in the tray when you sign in to Windows" : (launch?.reason ?? "Checking…")}
    </div>
  </div>
  <input
    type="checkbox"
    checked={launch?.enabled ?? false}
    disabled={locked}
    onchange={(e) => void toggle((e.target as HTMLInputElement).checked)}
  />
</label>

<style>
  /* Twin: `.toggle-row` in SettingsView.svelte. */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 0;
    border-top: 1px solid var(--p-border);
    cursor: pointer;
  }

  .toggle-row.muted {
    opacity: 0.55;
    cursor: default;
  }

  .row-title {
    font-size: 13px;
    font-weight: 600;
  }

  .row-sub {
    font-size: 12px;
    color: var(--p-text-dim);
    margin-top: 3px;
  }
</style>
