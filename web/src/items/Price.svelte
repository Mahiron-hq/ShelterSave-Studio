<script lang="ts">
  import { catalog, nameOf } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { fmtNum } from "../lib/format"

  interface Props {
    craft?: { caps: number; junk: [string, number][] } | null
    sell?: number
    theme?: boolean
  }
  let { craft, sell = 0, theme = false }: Props = $props()
  const junk = $derived(
    (craft?.junk ?? []).map(([id, n]) => {
      const j = (catalog().junk as any[]).find((x) => x.id === id)
      return `${j ? nameOf(j.name, app.itemRu, id) : id} × ${n}`
    }),
  )
</script>

{#if craft}
  <p>{app.t("card.craft", { caps: fmtNum(craft.caps) })}</p>
  {#if junk.length}<p class="muted small">{junk.join(", ")}</p>{/if}
  {#if theme}<p class="muted small">{app.t("card.craftTheme")}</p>{/if}
{/if}
{#if sell > 0}
  <p>{app.t("card.sell", { caps: fmtNum(sell) })}</p>
{/if}

<style>
  p {
    margin: 2px 0;
  }
</style>
