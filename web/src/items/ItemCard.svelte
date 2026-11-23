<script lang="ts">
  import { item, nameOf, type ItemKind } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import CardTrigger from "./CardTrigger.svelte"

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

<CardTrigger subject={{ kind, id }} text={text ?? name} />
