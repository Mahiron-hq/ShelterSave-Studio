<script lang="ts">
  import { icons, type IconName } from "../lib/icons"

  interface Props {
    name: IconName
    big?: boolean
  }
  let { name, big = false }: Props = $props()
</script>

<span class="icon" class:big>{@html icons[name]}</span>

<style>
  .icon {
    display: inline-flex;
    line-height: 0;
    flex: none;
    font-size: 16px;
  }
  .big {
    font-size: 40px;
  }
</style>
