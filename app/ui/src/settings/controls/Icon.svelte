<script lang="ts">
  import { GLYPHS, type IconName } from '../icons';

  interface Props {
    name: IconName;
    size?: number;
  }

  let { name, size = 16 }: Props = $props();
</script>

<span class="icon" aria-hidden="true" style:font-size={`${size}px`}>{GLYPHS[name]}</span>

<style>
  .icon {
    font-family: 'Segoe Fluent Icons', 'Segoe MDL2 Assets';
    font-style: normal;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
</style>
