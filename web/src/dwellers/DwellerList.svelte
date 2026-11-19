<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import Pick from "../components/Pick.svelte"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { importDwellers } from "./importDwellers"
  import { dwellerRows, FILTERS, label, SORTS } from "./rows"

  const all = $derived(dwellerRows(app.root))
  const shown = $derived.by(() => {
    const q = ui.dw.search.trim().toLowerCase()
    const keep = FILTERS[ui.dw.filter]?.[1] ?? (() => true)
    const cmp = SORTS[ui.dw.sort]?.[1] ?? SORTS[0][1]
    return all
      .filter(keep)
      .filter((r) => !q || r.name.toLowerCase().includes(q) || String(r.id) === q)
      .sort((a, b) => (ui.dw.desc ? -cmp(a, b) : cmp(a, b)))
  })

  function toggle(id: number, on: boolean) {
    const s = new Set(ui.dw.selected)
    if (on) {
      s.add(id)
      if (ui.dw.focus === null) ui.dw.focus = id
    } else {
      s.delete(id)
      if (ui.dw.focus === id) ui.dw.focus = [...s][0] ?? null
    }
    ui.dw.selected = s
  }

  function open(id: number, e: MouseEvent) {
    ui.dw.focus = id
    if (e.ctrlKey || e.metaKey) toggle(id, !ui.dw.selected.has(id))
    else if (!ui.dw.selected.has(id)) ui.dw.selected = new Set([id])
  }
</script>

<div class="list-panel">
  <input type="search" aria-label={app.t("c.search")} placeholder={app.t("c.search")} bind:value={ui.dw.search} />
  <div class="row-wrap">
    <Pick
      label={app.t("dw.filter.all")}
      value={String(ui.dw.filter)}
      items={FILTERS.map(([f], i) => ({ value: String(i), label: app.t(f) }))}
      onchange={(v) => (ui.dw.filter = Number(v))}
    />
    <Pick
      label={app.t("dw.sort.name")}
      value={String(ui.dw.sort)}
      items={SORTS.map(([s], i) => ({ value: String(i), label: app.t(s) }))}
      onchange={(v) => (ui.dw.sort = Number(v))}
    />
    <button type="button" class="icon" aria-label={app.t("dw.sortDesc")} aria-pressed={ui.dw.desc} onclick={() => (ui.dw.desc = !ui.dw.desc)}>
      <Icon name="sort" />
    </button>
  </div>
  <div class="row-wrap">
    <button type="button" class="small-btn" onclick={() => (ui.dw.selected = new Set(shown.map((r) => r.id)))}>{app.t("c.selectAll")}</button>
    <button type="button" class="small-btn" onclick={() => (ui.dw.selected = new Set())}>{app.t("c.selectNone")}</button>
    <span class="muted small">{app.t("c.selected", { n: ui.dw.selected.size })}</span>
  </div>
  <div class="row-wrap">
    <button type="button" onclick={() => app.push({ kind: "custom", id: "newDweller" })}><Icon name="userPlus" /> {app.t("dw.add")}</button>
    <button type="button" title={app.t("dw.importHelp")} onclick={importDwellers}><Icon name="download" /> {app.t("dw.import")}</button>
    <button type="button" title={app.t("dw.fromJournalHelp")} onclick={() => app.push({ kind: "custom", id: "fromJournal", data: { pets: false } })}>
      <Icon name="star" /> {app.t("dw.fromJournal")}
    </button>
  </div>
  {#if shown.length === 0}
    <p class="muted">{all.length === 0 ? app.t("dw.emptyVault") : app.t("c.noResults")}</p>
  {:else}
    <ul class="rows" aria-label={app.t("nav.dwellers")}>
      {#each shown as r (r.id)}
        <li class:focused={ui.dw.focus === r.id}>
          <Check checked={ui.dw.selected.has(r.id)} label="" aria={label(r)} onchange={(v) => toggle(r.id, v)} />
          <button type="button" class="open" aria-current={ui.dw.focus === r.id ? "true" : undefined} onclick={(e) => open(r.id, e)}>
            <span class="name" class:dead={r.dead}>
              {#if r.dead}<Icon name="skull" />{/if}
              {label(r)}
            </span>
            <span class="muted small sub">
              {app.t("dw.level")}
              {r.level} · <Icon name="smile" />
              {Math.round(r.happiness)} · <Icon name="heartPulse" />
              {Math.round(r.health)}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .list-panel {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
    min-width: 0;
  }
  .small-btn {
    min-height: 26px;
    padding: 2px 8px;
    font-size: 12px;
  }
  .rows {
    list-style: none;
    padding: 0;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface);
    max-height: calc(100vh - 260px);
    overflow: auto;
  }
  .rows li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-bottom: 1px solid var(--border);
  }
  .rows li.focused {
    background: var(--accent-soft);
  }
  .open {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    flex-direction: column;
    align-items: flex-start;
    gap: 0;
    padding: 4px 0;
  }
  .open:hover {
    background: transparent !important;
  }
  .name {
    font-weight: 600;
    display: flex;
    gap: 4px;
    align-items: center;
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name.dead {
    color: var(--muted);
    text-decoration: line-through;
  }
  .sub {
    display: flex;
    gap: 4px;
    align-items: center;
    white-space: nowrap;
  }
</style>
