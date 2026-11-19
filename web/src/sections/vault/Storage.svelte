<script lang="ts">
  import CopyId from "../../components/CopyId.svelte"
  import ArtNotice from "../../art/ArtNotice.svelte"
  import Icon from "../../components/Icon.svelte"
  import Check from "../../components/Check.svelte"
  import Pick from "../../components/Pick.svelte"
  import NumPick from "../../fields/NumPick.svelte"
  import ItemPicker from "../../items/ItemPicker.svelte"
  import ItemCard from "../../items/ItemCard.svelte"
  import { consts, item, ITEM_KINDS, nameOf, rarityName, type ItemKind } from "../../lib/core"
  import { app } from "../../lib/state.svelte"
  import { ui } from "../../lib/ui.svelte"
  import { arr, ITEMS } from "../../lib/vault"
  import type { IconName } from "../../lib/icons"

  const icons: Record<ItemKind, IconName> = { Weapon: "sword", Outfit: "shirt", Junk: "wrench", Pet: "paw" }
  let addKind = $state<ItemKind | "all">("all")
  let addCount = $state(1)

  const groups = $derived.by(() => {
    const m = new Map<string, { kind: ItemKind; id: string; n: number }>()
    let other = 0
    for (const it of arr(app.root, ITEMS)) {
      const kind = it?.type as ItemKind
      if (!ITEM_KINDS.includes(kind) || typeof it.id !== "string") {
        other++
        continue
      }
      const k = `${kind}|${it.id}`
      const g = m.get(k)
      if (g) g.n++
      else m.set(k, { kind, id: it.id, n: 1 })
    }
    return { list: [...m.values()], other }
  })
  const total = $derived(groups.list.reduce((a, g) => a + g.n, 0) + groups.other)

  const rows = $derived.by(() => {
    const q = ui.st.search.trim().toLowerCase()
    return groups.list
      .map((g) => {
        const info = item(g.kind, g.id)
        return { ...g, key: `${g.kind}|${g.id}`, info, name: info ? nameOf(info.name, app.itemRu, g.id) : g.id, rarity: info?.rarity ?? "" }
      })
      .filter((r) => !ui.st.kind || r.kind === ui.st.kind)
      .filter((r) => !ui.st.rarity || r.rarity === ui.st.rarity || (ui.st.rarity === "Normal" && r.rarity === "Common"))
      .filter(
        (r) =>
          !q ||
          r.id.toLowerCase().includes(q) ||
          r.name.toLowerCase().includes(q) ||
          (r.info && (r.info.name.en.toLowerCase().includes(q) || r.info.name.ru.toLowerCase().includes(q))),
      )
      .sort((a, b) => a.kind.localeCompare(b.kind) || a.id.localeCompare(b.id))
  })

  const kindItems = $derived([{ value: "all", label: app.t("c.all") }, ...ITEM_KINDS.map((k) => ({ value: k, label: app.t(`it.kind.${k}`) }))])
  const rarityItems = $derived([
    { value: "all", label: app.t("c.all") },
    ...["Normal", "Rare", "Legendary"].map((r) => ({ value: r, label: rarityName(r, app.ru) })),
  ])

  function toggle(key: string, on: boolean) {
    const s = new Set(ui.st.selected)
    if (on) s.add(key)
    else s.delete(key)
    ui.st.selected = s
  }

  function removeSelected() {
    for (const key of ui.st.selected) {
      const g = groups.list.find((x) => `${x.kind}|${x.id}` === key)
      if (g) app.act({ action: "removeItems", kind: g.kind, id: g.id, count: g.n })
    }
    ui.st.selected = new Set()
  }
</script>

<p class="muted">{app.t("vault.storageHelp")}</p>
<ArtNotice />

<section class="panel">
  <h3>{app.t("vault.addTitle")}</h3>
  <p class="muted small">{app.t("vault.addHelp")}</p>
  <div class="row-wrap">
    <span>{app.t("it.type")}</span>
    <Pick label={app.t("it.type")} value={addKind} items={kindItems} onchange={(v) => (addKind = v as ItemKind | "all")} />
    <span>{app.t("vault.quantity")}</span>
    <NumPick value={addCount} label={app.t("vault.quantity")} lo={1} hi={consts().itemAddMax} oncommit={(v) => (addCount = v)} />
    <ItemPicker
      kinds={addKind === "all" ? ITEM_KINDS : [addKind]}
      label={app.t("it.pick")}
      onpick={(kind, id) => app.act({ action: "addItems", kind, id, count: addCount })}
    />
  </div>
</section>

<div class="row-wrap filters">
  <input type="search" aria-label={app.t("c.search")} placeholder={app.t("c.search")} bind:value={ui.st.search} />
  <span class="muted">{app.t("it.type")}</span>
  <Pick label={app.t("it.type")} value={ui.st.kind ?? "all"} items={kindItems} onchange={(v) => (ui.st.kind = v === "all" ? null : v)} />
  <span class="muted">{app.t("it.rarity")}</span>
  <Pick label={app.t("it.rarity")} value={ui.st.rarity ?? "all"} items={rarityItems} onchange={(v) => (ui.st.rarity = v === "all" ? null : v)} />
  <span class="muted">{app.t("it.total", { n: total })}</span>
  <button type="button" disabled={ui.st.selected.size === 0} onclick={removeSelected}><Icon name="trash" /> {app.t("vault.removeSelected")}</button>
</div>

{#if rows.length === 0}
  <p class="muted empty">{ui.st.search.trim() ? app.t("c.noResults") : app.t("c.empty")}</p>
{:else}
  <ul class="items">
    {#each rows as r (r.key)}
      <li>
        <Check checked={ui.st.selected.has(r.key)} label="" aria={r.name} onchange={(v) => toggle(r.key, v)} />
        <span class="kind" title={app.t(`it.kind.${r.kind}`)}><Icon name={icons[r.kind]} /></span>
        <span class="name">
          <ItemCard kind={r.kind} id={r.id} />
          {#if !r.info}<span class="warn small">{app.t("c.unknownItem")}</span>{/if}
          {#if r.rarity}<span class="muted small">{rarityName(r.rarity, app.ru)}</span>{/if}
          {#if app.expert}<CopyId id={r.id} />{/if}
        </span>
        <span class="mono count">× {r.n}</span>
        <span class="actions">
          <button type="button" class="icon" aria-label={app.t("c.add")} title={app.t("c.add")} onclick={() => app.act({ action: "addItems", kind: r.kind, id: r.id, count: 1 })}><Icon name="plus" /></button>
          <button type="button" class="icon" aria-label={app.t("vault.removeOne")} title={app.t("vault.removeOne")} onclick={() => app.act({ action: "removeItems", kind: r.kind, id: r.id, count: 1 })}><Icon name="minus" /></button>
          <button type="button" class="icon" aria-label={app.t("c.removeAll")} title={app.t("c.removeAll")} onclick={() => app.act({ action: "removeItems", kind: r.kind, id: r.id, count: r.n })}><Icon name="trash" /></button>
        </span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .filters {
    margin: var(--s-md) 0;
  }
  .items {
    list-style: none;
    padding: 0;
    margin: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
  }
  .items li {
    display: flex;
    align-items: center;
    gap: var(--s-sm);
    padding: 4px var(--s-sm);
    border-bottom: 1px solid var(--border);
    min-height: 40px;
  }
  .items li:last-child {
    border-bottom: 0;
  }
  .kind {
    color: var(--muted);
  }
  .name {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 8px;
    align-items: baseline;
    min-width: 0;
  }
  .warn {
    color: var(--warning);
  }
  .actions {
    display: flex;
    gap: 2px;
  }
  .empty {
    padding: var(--s-lg) 0;
  }
</style>
