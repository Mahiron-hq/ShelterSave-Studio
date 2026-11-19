<script lang="ts">
  import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from "@ui-sdk/svelte/command"
  import { Dialog, DialogBackdrop, DialogContent, DialogShortcut, DialogTitle, DialogTrigger } from "@ui-sdk/svelte/dialog"
  import Icon from "./Icon.svelte"
  import { catalog, nameOf } from "../lib/core"
  import { app, type Section } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import type { IconName } from "../lib/icons"

  const PLACES: [string, string][] = [
    ["nav.save", "s"],
    ["save.general", "s"],
    ["save.time", "s"],
    ["save.summary", "s"],
    ["save.integrity", "s"],
    ["save.json", "s"],
    ["nav.vault", "v"],
    ["vault.resources", "v:resources"],
    ["vault.bonus", "v:resources"],
    ["vault.lunchboxes", "v:resources"],
    ["res.Nuka", "v:resources"],
    ["res.Food", "v:resources"],
    ["res.Energy", "v:resources"],
    ["res.Water", "v:resources"],
    ["res.StimPack", "v:resources"],
    ["res.RadAway", "v:resources"],
    ["res.NukaColaQuantum", "v:resources"],
    ["vault.storage", "v:storage"],
    ["vault.rooms", "v:rooms"],
    ["vault.map", "v:rooms"],
    ["vault.rocks", "v:rooms"],
    ["vault.themes", "v:themes"],
    ["vault.unlocks", "v:book"],
    ["vault.recipesTitle", "v:book"],
    ["vault.journal", "v:book"],
    ["o.incidents", "o:events"],
    ["nav.dwellers", "d"],
    ["dw.pets", "d"],
    ["dw.waiting", "d"],
    ["nav.other", "o"],
    ["o.quests", "o:quests"],
    ["o.wasteland", "o:wasteland"],
    ["o.objectives", "o:objectives"],
    ["o.stats", "o:stats"],
    ["o.tutorial", "o:tutorial"],
    ["o.events", "o:events"],
    ["o.service", "o:service"],
    ["o.tree", "o:tree"],
    ["set.title", "set"],
    ["set.language", "set"],
    ["set.theme", "set"],
    ["set.mode", "set"],
    ["set.itemNames", "set"],
  ]

  interface Hit {
    value: string
    icon: IconName
    title: string
    sub: string
  }

  let open = $state(false)
  let query = $state("")

  const kinds: Record<string, string> = { Weapon: "weapons", Outfit: "outfits", Pet: "pets", Junk: "junk" }

  const hits = $derived.by((): Hit[] => {
    const q = query.trim().toLowerCase()
    if (!q) return []
    const out: Hit[] = []
    for (const [key, code] of PLACES) {
      const text = app.t(key)
      if (text.toLowerCase().includes(q) && (app.isOpen || code === "set"))
        out.push({ value: `p|${code}|${key}`, icon: "layers", title: text, sub: "" })
    }
    const root = app.root
    if (root) {
      for (const d of root.dwellers?.dwellers ?? []) {
        const name = `${d.name ?? ""} ${d.lastName ?? ""}`.trim()
        const id = d.serializeId
        if (name.toLowerCase().includes(q) || String(id) === q)
          out.push({ value: `d|${id}`, icon: "users", title: name, sub: `#${id}` })
        if (out.length > 60) break
      }
      const seen = new Set<string>()
      const cat = catalog()
      for (const it of root.vault?.inventory?.items ?? []) {
        const kind = kinds[it.type]
        if (!kind || typeof it.id !== "string") continue
        const k = `${it.type}|${it.id}`
        if (seen.has(k)) continue
        seen.add(k)
        const info = (cat[kind] as any[] | undefined)?.find((x) => x.id === it.id)
        const hit =
          it.id.toLowerCase().includes(q) ||
          (info && (info.name.en.toLowerCase().includes(q) || info.name.ru.toLowerCase().includes(q)))
        if (hit) out.push({ value: `i|${k}`, icon: "archive", title: info ? nameOf(info.name, app.itemRu, it.id) : it.id, sub: it.id })
        if (out.length > 100) break
      }
    }
    return out
  })

  function run(value: string) {
    open = false
    query = ""
    const [kind, a, b] = value.split("|")
    if (kind === "p") {
      if (a === "set") return app.push({ kind: "settings" })
      const [sec, tab] = a.split(":")
      const map: Record<string, Section> = { s: "save", v: "vault", d: "dwellers", o: "other" }
      app.section = map[sec] ?? "save"
      if (tab && sec === "v") app.vaultTab = tab
      if (tab && sec === "o") app.otherTab = tab
    } else if (kind === "d") {
      const id = Number(a)
      app.section = "dwellers"
      ui.dw.focus = id
      ui.dw.selected = new Set([id])
    } else if (kind === "i") {
      app.section = "vault"
      app.vaultTab = "storage"
      ui.st.kind = a
      ui.st.search = b
    }
  }
</script>

<Dialog bind:open>
  <DialogTrigger shortcut="mod+k" class="search-trigger">
    <Icon name="search" />
    <span class="label">{app.t("tb.search")}</span>
    <DialogShortcut />
  </DialogTrigger>
  <DialogBackdrop />
  <DialogContent class="search-dialog">
    <DialogTitle class="sr-only">{app.t("tb.search")}</DialogTitle>
    <Command filter={false} bind:query onAction={run} autoHighlight>
      <div class="row">
        <Icon name="search" />
        <CommandInput aria-label={app.t("tb.search")} placeholder={app.t("tb.search")} />
      </div>
      {#if query.trim()}
        <CommandEmpty>{app.t("c.nothingFound")}</CommandEmpty>
      {/if}
      <CommandList>
        {#each hits as h (h.value)}
          <CommandItem value={h.value}>
            <Icon name={h.icon} />
            <span class="title">{h.title}</span>
            {#if h.sub}<span class="mono muted small">{h.sub}</span>{/if}
          </CommandItem>
        {/each}
      </CommandList>
    </Command>
  </DialogContent>
</Dialog>

<style>
  :global(.search-trigger) {
    width: 100%;
    max-width: 360px;
    justify-content: flex-start;
    color: var(--muted);
  }
  :global(.search-trigger .label) {
    flex: 1;
    text-align: start;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  :global(.search-trigger [data-slot="dialog-shortcut"]) {
    font-family: var(--mono);
    font-size: 11px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .row :global(input) {
    flex: 1;
  }
  :global(.search-dialog [data-slot="command-list"]) {
    max-height: 60vh;
    overflow: auto;
  }
  :global(.search-dialog [data-slot="command-item"]) {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 8px;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  :global(.search-dialog [data-slot="command-item"][data-highlighted]) {
    background: var(--accent-soft);
  }
  .title {
    flex: 1;
  }
</style>
