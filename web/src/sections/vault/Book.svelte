<script lang="ts">
  import Icon from "../../components/Icon.svelte"
  import Check from "../../components/Check.svelte"
  import Seg from "../../components/Seg.svelte"
  import ItemCard from "../../items/ItemCard.svelte"
  import ThemeCard from "../../rooms/ThemeCard.svelte"
  import { catalog, core, nameOf, query, rarityName } from "../../lib/core"
  import { app } from "../../lib/state.svelte"

  type Filter = "all" | "weapons" | "outfits" | "themes"
  let filter = $state<Filter>("all")
  let q = $state("")

  const rows = $derived.by(() => {
    void app.rev
    const cat = catalog()
    const built = new Set(query<{ rooms: { kind: string }[] }>("map").rooms.map((r) => r.kind))
    return (cat.rooms as any[])
      .map((r) => {
        const u = r.unlock ? (cat.unlockables as any[]).find((x) => x.id === r.unlock) : null
        return u && (app.expert || !u.season) ? { kind: r.type as string, goal: u.goal as number } : null
      })
      .filter((x): x is { kind: string; goal: number } => x !== null)
      .sort((a, b) => a.goal - b.goal || a.kind.localeCompare(b.kind))
      .map((r) => ({ ...r, unlocked: query("roomLock", { kind: r.kind }) === null, built: built.has(r.kind) }))
  })

  const recipes = $derived.by(() => {
    const cat = catalog()
    const all: { f: Filter; id: string; name: string; rarity: string }[] = []
    for (const w of cat.weapons as any[]) if (w.recipe) all.push({ f: "weapons", id: w.id, name: nameOf(w.name, app.ru, w.id), rarity: w.rarity })
    for (const o of cat.outfits as any[]) if (o.recipe) all.push({ f: "outfits", id: o.id, name: nameOf(o.name, app.ru, o.id), rarity: o.rarity })
    for (const t of cat.themes as any[]) if (t.recipe) all.push({ f: "themes", id: t.id, name: nameOf(t.name, app.ru, t.id), rarity: "" })
    return all
  })
  const known = $derived(new Set<string>((app.root?.survivalW?.recipes ?? []).filter((x: unknown) => typeof x === "string")))
  const shown = $derived.by(() => {
    const s = q.trim().toLowerCase()
    return recipes
      .filter((r) => filter === "all" || r.f === filter)
      .filter((r) => !s || r.name.toLowerCase().includes(s) || r.id.toLowerCase().includes(s))
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
  })

  const fresh = $derived.by(() => {
    const w = app.root?.survivalW ?? {}
    let n = 0
    for (const k of ["weapons", "outfits", "dwellers", "pets", "breeds", "decorations", "junk"])
      for (const v of w[k] ?? []) if (typeof v === "string" && v.startsWith("N")) n++
    for (const t of w.collectedThemes?.themeList ?? []) if (t?.extraData?.IsNew === true) n++
    for (const t of Object.values(app.root?.specialTheme?.eventsThemes ?? {}) as any[]) if (t?.extraData?.IsNew === true) n++
    return n
  })
</script>

<section class="panel">
  <h3>{app.t("vault.unlocks")}</h3>
  <p class="muted">{app.t("vault.unlocksHelp")}</p>
  {#if app.root?.unlockableMgr === undefined}
    <p class="muted">{app.t("theme.noData")}</p>
  {:else}
    <ul class="list">
      {#each rows as r (r.kind)}
        <li>
          <span title={r.unlocked && r.built && !app.expert ? app.t("vault.lockBuilt") : undefined}>
            <Check
              checked={r.unlocked}
              label={core().kindName(r.kind, 1)}
              disabled={r.unlocked && r.built && !app.expert}
              onchange={(v) => app.act({ action: "setRoomUnlocked", room: r.kind, unlocked: v })}
            />
          </span>
          <span class="muted small">{app.t("vault.unlockAt", { n: r.goal })}</span>
          {#if r.built && !r.unlocked}<span class="danger small"><Icon name="alert" /> {app.t("vault.builtLocked")}</span>{/if}
        </li>
      {/each}
    </ul>
    <button type="button" title={app.t("vault.unlockRoomsHelp")} onclick={() => app.act({ action: "unlockAllRooms" })}>
      <Icon name="lock" /> {app.t("vault.unlockRooms")}
    </button>
  {/if}
</section>

<section class="panel">
  <h3>{app.t("vault.recipesTitle")}</h3>
  <p class="muted">{app.t("vault.recipesHelp")}</p>
  <p>{app.t("vault.recipesCount", { n: recipes.filter((r) => known.has(r.id)).length, total: recipes.length })}</p>
  <div class="row-wrap">
    <Seg
      label={app.t("vault.recipesTitle")}
      value={filter}
      options={[
        { value: "all" as Filter, label: app.t("vault.recipesAll") },
        { value: "weapons" as Filter, label: app.t("vault.recipesWeapons") },
        { value: "outfits" as Filter, label: app.t("vault.recipesOutfits") },
        { value: "themes" as Filter, label: app.t("vault.recipesThemes") },
      ]}
      onchange={(v) => (filter = v)}
    />
    <input type="search" aria-label={app.t("c.search")} placeholder={app.t("c.search")} bind:value={q} />
  </div>
  <ul class="list scroll">
    {#if shown.length === 0}<li class="muted">{app.t("c.nothingFound")}</li>{/if}
    {#each shown as r (r.id)}
      <li>
        <Check checked={known.has(r.id)} label="" aria={r.name} onchange={(v) => app.act({ action: "setRecipe", id: r.id, known: v })} />
        {#if r.f === "themes"}<ThemeCard id={r.id} />{:else}<ItemCard kind={r.f === "weapons" ? "Weapon" : "Outfit"} id={r.id} />{/if}
        {#if r.rarity}<span class="muted small">{rarityName(r.rarity, app.ru)}</span>{/if}
      </li>
    {/each}
  </ul>
  <button type="button" title={app.t("vault.unlockRecipesHelp")} onclick={() => app.act({ action: "unlockAllRecipes" })}>
    <Icon name="scroll" /> {app.t("vault.unlockRecipes")}
  </button>
</section>

<section class="panel">
  <h3>{app.t("vault.journal")}</h3>
  <p class="muted">{app.t("vault.journalHelp")}</p>
  <p>{app.t("vault.newMarks", { n: fresh })}</p>
  <div class="row-wrap">
    <button type="button" disabled={fresh === 0} title={app.t("vault.markReadHelp")} onclick={() => app.act({ action: "markCollectionRead" })}>
      <Icon name="check" /> {app.t("vault.markRead")}
    </button>
    <button type="button" title={app.t("vault.completeCollectionHelp")} onclick={() => app.act({ action: "completeCollection" })}>
      <Icon name="badge" /> {app.t("vault.completeCollection")}
    </button>
  </div>
</section>

<style>
  .list {
    list-style: none;
    padding: 0;
    margin: var(--s-sm) 0;
  }
  .list li {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
    padding: 3px 0;
  }
  .scroll {
    max-height: 420px;
    overflow: auto;
  }
  .danger {
    color: var(--danger);
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }
</style>
