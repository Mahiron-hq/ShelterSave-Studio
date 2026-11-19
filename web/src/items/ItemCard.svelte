<script lang="ts">
  import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "@ui-sdk/svelte/popover"
  import { item, nameOf, type ItemKind } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import ItemInfo from "./ItemInfo.svelte"

  interface Props {
    kind: ItemKind
    id: string
    /** Text of the trigger; the item name by default. */
    text?: string
  }
  let { kind, id, text }: Props = $props()

  const info = $derived(item(kind, id))
  const name = $derived(info ? nameOf(info.name, app.itemRu, id) : id)
</script>

<Popover openOnHover delay={300} closeDelay={150}>
  <PopoverTrigger class="item-trigger">{text ?? name}</PopoverTrigger>
  <PopoverContent class="card">
    <ItemInfo {kind} {id}>
      {#snippet title(n)}<PopoverTitle class="title">{n}</PopoverTitle>{/snippet}
    </ItemInfo>
  </PopoverContent>
</Popover>

<style>
  :global(.item-trigger) {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    font-weight: 600;
    text-align: start;
    cursor: help;
    color: var(--text);
  }
  :global(.item-trigger:hover) {
    text-decoration: underline dotted;
    background: transparent !important;
  }
  :global([data-slot="popover-content"].card) {
    max-width: 340px;
  }
</style>
