<script lang="ts">
  import { catalog, nameOf } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import CardTrigger from "../items/CardTrigger.svelte"

  interface Props {
    id: string
    text?: string
  }
  let { id, text }: Props = $props()
  const t = $derived((catalog().themes as any[]).find((x) => x.id === id))
  const name = $derived(t ? nameOf(t.name, app.ru, id) : id)
</script>

<CardTrigger subject={{ theme: id }} text={text ?? name} />
