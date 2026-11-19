<script lang="ts">
  import { Combobox, ComboboxContent, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList } from "@ui-sdk/svelte/combobox"
  import { item, nameOf, rarityName, searchItems, type ItemKind } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import ItemInfo from "./ItemInfo.svelte"

  interface Props {
    kinds: ItemKind[]
    label: string
    gender?: number
    onpick: (kind: ItemKind, id: string) => void
  }
  let { kinds, label, gender = 0, onpick }: Props = $props()
  let query = $state("")
  let lit = $state<string | null>(null)
  const litItem = $derived.by(() => {
    if (!lit) return null
    const [k, ...rest] = lit.split("|")
    const id = rest.join("|")
    return item(k as ItemKind, id) ? { kind: k as ItemKind, id } : null
  })

  const found = $derived.by(() => {
    const list = searchItems(kinds, query, gender).slice(0, 200)
    const out = list.map((i) => ({
      value: `${i.kind}|${i.id}`,
      label: `${nameOf(i.name, app.itemRu, i.id)}  ·  ${rarityName(i.rarity, app.ru)}`,
    }))
    if (app.expert && query.trim()) {
      for (const k of kinds)
        out.push({ value: `${k}|${query.trim()}`, label: `${app.t("it.manualId")}: ${query.trim()} (${app.t(`it.kind.${k}`)})` })
    }
    return out
  })
</script>

<Combobox
  filter={false}
  bind:query
  value={null}
  onHighlight={(v) => (lit = typeof v === "string" ? v : null)}
  onOpenChange={(open) => {
    if (!open) lit = null
  }}
  onValueChange={(picked) => {
    const v = picked as unknown
    if (typeof v !== "string") return
    const [k, ...rest] = v.split("|")
    query = ""
    onpick(k as ItemKind, rest.join("|"))
  }}
>
  <ComboboxInput aria-label={label} placeholder={app.t("it.pick")} class="picker-input" />
  <ComboboxContent class="with-info">
    <div class="list">
      <ComboboxEmpty>{app.t("c.noResults")}</ComboboxEmpty>
      <ComboboxList>
        {#each found as f (f.value)}
          <ComboboxItem value={f.value}>{f.label}</ComboboxItem>
        {/each}
      </ComboboxList>
    </div>
    {#if litItem}
      <aside class="info card" aria-live="polite">
        <ItemInfo kind={litItem.kind} id={litItem.id} />
      </aside>
    {/if}
  </ComboboxContent>
</Combobox>

<style>
  :global([data-slot="combobox-content"].with-info) {
    display: flex;
    gap: var(--s-sm);
    align-items: flex-start;
    overflow: hidden;
  }
  .list {
    overflow: auto;
    max-height: min(350px, var(--available-height, 350px));
    min-width: min(260px, 80vw);
  }
  .info {
    width: 300px;
    max-height: min(350px, var(--available-height, 350px));
    overflow: auto;
    padding: 4px var(--s-sm);
    border-inline-start: 1px solid var(--border);
  }
  @media (max-width: 640px) {
    :global([data-slot="combobox-content"].with-info) {
      flex-direction: column;
    }
    .list {
      max-height: 180px;
    }
    .info {
      width: auto;
      max-height: 170px;
      border-inline-start: 0;
      border-top: 1px solid var(--border);
    }
  }
  :global(.picker-input) {
    width: min(280px, 100%);
  }
</style>
