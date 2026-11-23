<script lang="ts">
  interface Props {
    size?: number;
  }

  let { size = 18 }: Props = $props();
</script>

<!-- The app's snowflake mark (Segoe Fluent Icons has no snowflake). Drawn here, not copied from any app (§9). -->
<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  {#each [0, 60, 120] as angle (angle)}
    <g transform={`rotate(${angle} 12 12)`}>
      <path d="M12 2.5v19M12 6l-2.6-2.2M12 6l2.6-2.2M12 18l-2.6 2.2M12 18l2.6 2.2" />
    </g>
  {/each}
</svg>

<style>
  svg {
    display: block;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
