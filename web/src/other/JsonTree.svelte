<script lang="ts">
  import { Tree, TreeItem, TreeItemIndicator, TreeItemLabel, visibleTreeRows, type TreeItemData } from "@ui-sdk/svelte/tree"
  import Icon from "../components/Icon.svelte"
  import { at, core, pathStr, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { short } from "../lib/format"

  const MAX_CHILDREN = 400
  const MAX_MATCHES = 300
  const ROW = 26
  const HEIGHT = 520

  interface Node extends TreeItemData {
    path: Path
  }

  // Paths are kept by their string form; the tree only needs unique values.
  const byValue = new Map<string, Path>()
  function build(v: any, path: Path, label: string): Node {
    const value = pathStr(path) || "$"
    byValue.set(value, path)
    if (v !== null && typeof v === "object") {
      const entries: [string | number, any][] = Array.isArray(v) ? v.map((x, i) => [i, x]) : Object.entries(v)
      const kids = entries.slice(0, MAX_CHILDREN).map(([k, x]) => build(x, [...path, k], typeof k === "number" ? `[${k}]` : k))
      return { value, label: `${label}  ${Array.isArray(v) ? `[${v.length}]` : `{${entries.length}}`}`, path, children: kids }
    }
    return { value, label: `${label}: ${v === null ? "null" : typeof v === "string" ? JSON.stringify(v) : short(v)}`, path }
  }

  const items = $derived.by(() => {
    byValue.clear()
    const root = app.root ?? {}
    return Object.entries(root).map(([k, v]) => build(v, [k], k))
  })
  let expanded = $state<string[]>([])
  let selected = $state<string | null>(null)
  let scroller = $state<HTMLDivElement>()
  let scrollTop = $state(0)
  const rows = $derived(visibleTreeRows(items, expanded))
  const start = $derived(Math.max(0, Math.floor(scrollTop / ROW) - 6))
  const end = $derived(Math.min(rows.length, Math.ceil((scrollTop + HEIGHT) / ROW) + 6))

  const matches = $derived.by(() => {
    const q = ui.treeSearch.trim().toLowerCase()
    if (!q) return []
    const out: Path[] = []
    const walk = (v: any, path: Path) => {
      if (out.length >= MAX_MATCHES) return
      const ps = pathStr(path).toLowerCase()
      const hitPath = path.length > 0 && ps.includes(q)
      if (v !== null && typeof v === "object") {
        if (hitPath) out.push(path)
        const entries: [string | number, any][] = Array.isArray(v) ? v.map((x, i) => [i, x]) : Object.entries(v)
        for (const [k, x] of entries) walk(x, [...path, k])
      } else if (hitPath || String(v).toLowerCase().includes(q)) out.push(path)
    }
    walk(app.root ?? {}, [])
    return out
  })

  function show(path: Path) {
    const anc: string[] = []
    for (let k = 1; k < path.length; k++) anc.push(pathStr(path.slice(0, k)))
    expanded = [...new Set([...expanded, ...anc])]
    selected = pathStr(path)
  }

  const selPath = $derived(selected ? (byValue.get(selected) ?? null) : null)
  const selValue = $derived(selPath ? at(app.root, selPath) : undefined)
  const raw = $derived((void app.rev, selPath ? (core().raw(pathStr(selPath)) ?? "") : ""))
  const modified = $derived((void app.rev, selPath ? core().modified(pathStr(selPath)) : false))
  let draft = $state<string | null>(null)
  let addKey = $state("")
  let addValue = $state("")
  const isObj = $derived(selValue !== null && typeof selValue === "object")

  function commit() {
    if (!selPath || draft === null) return
    const t = draft
    draft = null
    if (t !== raw) app.act({ action: "setRaw", path: selPath, json: t })
  }

  function remove() {
    if (!selPath || selPath.length === 0) return
    const parent = selPath.slice(0, -1)
    const last = selPath[selPath.length - 1]
    const pv = at(app.root, parent)
    const index = typeof last === "number" ? last : Object.keys(pv ?? {}).indexOf(String(last))
    if (index >= 0 && app.act({ action: "removeAt", parent, index })) selected = parent.length ? pathStr(parent) : null
  }

  function add() {
    if (!selPath || !isObj) return
    const object = !Array.isArray(selValue)
    const key = object ? addKey.trim() : null
    if (object && !key) return
    const len = Array.isArray(selValue) ? selValue.length : Object.keys(selValue).length
    if (app.act({ action: "insertRaw", parent: selPath, index: len, key, json: addValue.trim() || "null" })) {
      addKey = ""
      addValue = ""
    }
  }
</script>

{#if !app.expert}
  <p class="muted"><Icon name="lock" /> {app.t("o.treeLocked")}</p>
{:else}
  <p class="muted">{app.t("o.treeHelp")}</p>
  <input type="search" class="search" aria-label={app.t("o.searchTree")} placeholder={app.t("o.searchTree")} bind:value={ui.treeSearch} />
  {#if ui.treeSearch.trim()}
    <p class="muted small">{app.t("o.matches", { n: matches.length })}</p>
    <ul class="matches">
      {#each matches as m (pathStr(m))}
        <li><button type="button" class="link mono" onclick={() => show(m)}>{pathStr(m)}</button> <span class="muted small">{short(at(app.root, m))}</span></li>
      {/each}
    </ul>
  {/if}
  <div class="tree-layout">
    <div class="scroll" bind:this={scroller} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
      <Tree
        aria-label={app.t("o.tree")}
        {items}
        bind:expanded
        value={selected}
        onValueChange={(v) => (selected = typeof v === "string" ? v : null)}
        onHighlight={(_, { index }) => {
          const el = scroller
          if (!el) return
          const top = index * ROW
          if (top < el.scrollTop) el.scrollTop = top
          else if (top + ROW > el.scrollTop + el.clientHeight - 10) el.scrollTop = top + ROW - el.clientHeight + 10
          scrollTop = el.scrollTop
        }}
        class="json-tree"
        style="height: {rows.length * ROW}px"
      >
        {#each rows.slice(start, end) as { item, index } (item.value)}
          <TreeItem value={item.value} style="top: {index * ROW}px">
            <TreeItemLabel class="mono">
              {#if item.children}<TreeItemIndicator><Icon name="chevronRight" /></TreeItemIndicator>{/if}
              {item.label}
            </TreeItemLabel>
          </TreeItem>
        {/each}
      </Tree>
    </div>
    <aside class="panel editor">
      {#if !selPath}
        <p class="muted">{app.t("o.treePick")}</p>
      {:else}
        <h4 class="mono">{pathStr(selPath)}</h4>
        {#if !isObj}
          <input
            class="mono value"
            aria-label={app.t("o.value")}
            value={draft ?? raw}
            class:invalid={draft !== null && !(() => {
              try {
                JSON.parse(draft)
                return true
              } catch {
                return false
              }
            })()}
            oninput={(e) => (draft = e.currentTarget.value)}
            onblur={commit}
            onkeydown={(e) => e.key === "Enter" && commit()}
          />
        {/if}
        <div class="row-wrap">
          {#if modified}
            <span class="mark">{app.t("c.modified")}</span>
            <button type="button" onclick={() => app.act({ action: "revertValue", path: selPath })}><Icon name="reset" /> {app.t("c.revert")}</button>
          {/if}
          {#if selPath.length > 0}<button type="button" class="danger" onclick={remove}><Icon name="trash" /> {app.t("c.remove")}</button>{/if}
        </div>
        {#if isObj}
          <div class="row-wrap add">
            {#if !Array.isArray(selValue)}<input aria-label={app.t("o.key")} placeholder={app.t("o.key")} bind:value={addKey} />{/if}
            <input class="mono" aria-label={app.t("o.value")} placeholder={app.t("o.value")} bind:value={addValue} />
            <button type="button" onclick={add}><Icon name="plus" /> {Array.isArray(selValue) ? app.t("o.addElement") : app.t("o.addKey")}</button>
          </div>
        {/if}
      {/if}
    </aside>
  </div>
{/if}

<style>
  .search {
    width: min(420px, 100%);
  }
  .matches {
    list-style: none;
    padding: 0;
    max-height: 200px;
    overflow: auto;
  }
  .link {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    color: var(--accent);
    text-decoration: underline;
  }
  .tree-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 380px);
    gap: var(--s-md);
    margin-top: var(--s-sm);
  }
  .scroll {
    height: 520px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
  }
  :global(.json-tree) {
    position: relative;
  }
  :global(.json-tree [data-slot="tree-item"]) {
    position: absolute;
    left: 0;
    right: 0;
    height: 26px;
  }
  :global(.json-tree [data-slot="tree-item-label"]) {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding-inline-start: calc(var(--tree-level, 1) * 14px);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    font-size: 12.5px;
  }
  :global(.json-tree [data-slot="tree-item"][data-selected] > [data-slot="tree-item-label"]) {
    background: var(--accent-soft);
  }
  :global(.json-tree [data-slot="tree-item-indicator"][data-expanded] .icon),
  :global(.json-tree [data-expanded] > [data-slot="tree-item-label"] .icon) {
    transform: rotate(90deg);
  }
  .editor h4 {
    overflow-wrap: anywhere;
    margin: 0 0 var(--s-sm);
  }
  .value {
    width: 100%;
    margin-bottom: var(--s-sm);
  }
  .mark {
    color: var(--mark-text);
  }
  .add {
    margin-top: var(--s-sm);
  }
  @media (max-width: 900px) {
    .tree-layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
