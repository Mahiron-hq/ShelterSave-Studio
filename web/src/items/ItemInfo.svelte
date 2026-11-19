<script lang="ts">
  import ArtImage from "../art/ArtImage.svelte"
  import { catalog, item, nameOf, rarityName, type ItemKind } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { fmtNum } from "../lib/format"
  import { SPECIAL_LETTERS } from "../lib/vault"
  import Price from "./Price.svelte"
  import type { Snippet } from "svelte"

  interface Props {
    kind: ItemKind
    id: string
    /** Renders the title; a plain heading by default. */
    title?: Snippet<[string]>
  }
  let { kind, id, title }: Props = $props()
  const info = $derived(item(kind, id))
  const name = $derived(info ? nameOf(info.name, app.itemRu, id) : id)
  const facts = $derived.by(() => {
    if (!info) return ""
    if (kind === "Weapon") return app.t("card.damage", { min: info.damageMin, max: info.damageMax })
    if (kind === "Outfit") {
      const parts = SPECIAL_LETTERS.map((l, i) => [l, info.special?.[i] ?? 0] as const)
        .filter(([, v]) => v !== 0)
        .map(([l, v]) => `${l} ${v > 0 ? "+" : ""}${v}`)
      return parts.length ? app.t("card.special", { list: parts.join("  ") }) : ""
    }
    if (kind === "Pet") {
      const b = catalog().petBonuses?.[info.bonus]
      const bonus = b ? nameOf(b, app.itemRu, info.bonus) : info.bonus
      const range = `${fmtNum(info.bonusMin)}–${fmtNum(info.bonusMax)}`
      return bonus.includes("{0}") ? bonus.replace("{0}", range) : `${bonus}: ${range}`
    }
    return ""
  })
  const desc = $derived(info?.desc ? nameOf(info.desc, app.ru).replaceAll("\n", " ") : "")
</script>

<div class="head">
  <ArtImage id="{kind}:{id}" max={120} />
  <div>
    {#if title}{@render title(name)}{:else}<p class="title">{name}</p>{/if}
    <p class="muted small">
      {app.t(`it.kind.${kind}`)}{#if info?.rarity}&nbsp;·&nbsp;{rarityName(info.rarity, app.ru)}{/if}
    </p>
    {#if facts}<p>{facts}</p>{/if}
  </div>
</div>
{#if desc}<p class="muted">{desc}</p>{/if}
<Price craft={info?.craft} sell={info?.sell ?? 0} />
{#if app.expert}<p class="mono muted small">{id}</p>{/if}

<style>
  .head {
    display: flex;
    gap: var(--s-sm);
    align-items: flex-start;
  }
  .title,
  :global(.card .title) {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }
  p {
    margin: 2px 0;
  }
</style>
