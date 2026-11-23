<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import TreeNode from "./TreeNode.svelte"
  import { at, pathStr, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"

  const MAX_MATCHES = 300

  const matches = $derived.by(() => {
    const q = ui.treeSearch.trim().toLowerCase()
    if (!q) return []
    const out: Path[] = []
    const walk = (v: any, path: Path) => {
      if (out.length >= MAX_MATCHES) return
      const hitPath = path.length > 0 && pathStr(path).toLowerCase().includes(q)
      if (v !== null && typeof v === "object") {
        if (hitPath) out.push(path)
        const entries: [string | number, any][] = Array.isArray(v) ? v.map((x, i) => [i, x]) : Object.entries(v)
        for (const [k, x] of entries) walk(x, [...path, k])
      } else if (hitPath || String(v).toLowerCase().includes(q)) out.push(path)
    }
    walk(app.root ?? {}, [])
    return out
  })
</script>

{#if !app.expert}
  <p class="muted"><Icon name="lock" /> {app.t("o.treeLocked")}</p>
{:else}
  <p class="muted">{app.t("o.treeHelp")}</p>
  <input type="search" class="search" aria-label={app.t("o.searchTree")} placeholder={app.t("o.searchTree")} bind:value={ui.treeSearch} />
  <div class="tree">
    {#if ui.treeSearch.trim()}
      <p class="muted small">{app.t("o.matches", { n: matches.length })}</p>
      {#each matches as m (pathStr(m))}
        <TreeNode value={at(app.root, m)} path={m} label={pathStr(m)} />
      {/each}
    {:else}
      <TreeNode value={app.root ?? {}} path={[]} label="root" />
    {/if}
  </div>
{/if}

<style>
  .search {
    width: min(420px, 100%);
  }
  .tree {
    margin-top: var(--s-sm);
    max-height: 70vh;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    padding: var(--s-sm);
  }
</style>
