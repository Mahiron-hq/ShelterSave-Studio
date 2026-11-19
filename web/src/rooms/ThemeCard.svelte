<script lang="ts">
  import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "@ui-sdk/svelte/popover"
  import { catalog, nameOf } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import ThemeInfo from "./ThemeInfo.svelte"

  interface Props {
    id: string
    text?: string
  }
  let { id, text }: Props = $props()
  const t = $derived((catalog().themes as any[]).find((x) => x.id === id))
  const name = $derived(t ? nameOf(t.name, app.ru, id) : id)
</script>

<Popover openOnHover delay={300} closeDelay={150}>
  <PopoverTrigger class="item-trigger">{text ?? name}</PopoverTrigger>
  <PopoverContent class="card">
    <ThemeInfo {id}>
      {#snippet title(n)}<PopoverTitle class="title">{n}</PopoverTitle>{/snippet}
    </ThemeInfo>
  </PopoverContent>
</Popover>
