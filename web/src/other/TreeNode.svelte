<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import TreeNode from "./TreeNode.svelte"
  import { core, pathStr, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { treeOpen, treeAdd } from "./treeState.svelte"

  const MAX_CHILDREN = 400

  interface Props {
    value: any
    path: Path
    label: string
  }
  let { value, path, label }: Props = $props()

  const ps = $derived(pathStr(path))
  const isObj = $derived(value !== null && typeof value === "object")
  const isArr = $derived(Array.isArray(value))
  const entries = $derived<[string | number, any][]>(isObj ? (isArr ? value.map((x: any, i: number) => [i, x]) : Object.entries(value)) : [])
  const open = $derived(treeOpen.has(ps))

  let draft = $state<string | null>(null)
  const raw = $derived((void app.rev, isObj ? "" : (core().raw(ps) ?? "")))
  const modified = $derived((void app.rev, !isObj && core().modified(ps)))
  const valid = $derived.by(() => {
    if (draft === null) return true
    try {
      JSON.parse(draft)
      return true
    } catch {
      return false
    }
  })

  function toggle() {
    if (open) treeOpen.delete(ps)
    else treeOpen.add(ps)
  }
  function commit() {
    if (draft === null) return
    const t = draft
    draft = null
    if (t !== raw) app.act({ action: "setRaw", path, json: t })
  }
  function remove(index: number) {
    app.act({ action: "removeAt", parent: path, index })
  }
  function add() {
    const st = treeAdd
    const key = isArr ? null : st.key.trim()
    if (!isArr && !key) return
    if (app.act({ action: "insertRaw", parent: path, index: entries.length, key, json: st.value.trim() || "null" })) {
      st.key = ""
      st.value = ""
    }
  }
</script>

{#if isObj}
  <button type="button" class="head mono" class:open aria-expanded={open} onclick={toggle}>
    <Icon name="chevronRight" />
    {label}&nbsp;&nbsp;{isArr ? `[${entries.length}]` : `{${entries.length}}`}
  </button>
  {#if open}
    <div class="kids">
      {#each entries.slice(0, MAX_CHILDREN) as [k, child], i (k)}
        <div class="kid">
          <button type="button" class="icon ghost x" aria-label={app.t("c.remove")} title={app.t("c.remove")} onclick={() => remove(i)}><Icon name="x" /></button>
          <div class="sub"><TreeNode value={child} path={[...path, k]} label={typeof k === "number" ? `[${k}]` : k} /></div>
        </div>
      {/each}
      {#if entries.length > MAX_CHILDREN}<p class="muted">… {entries.length - MAX_CHILDREN}</p>{/if}
      <div class="add">
        {#if !isArr}<input aria-label={app.t("o.key")} placeholder={app.t("o.key")} bind:value={treeAdd.key} />{/if}
        <input class="mono" aria-label={app.t("o.value")} placeholder={app.t("o.value")} bind:value={treeAdd.value} />
        <button type="button" class="small" onclick={add}><Icon name="plus" /> {isArr ? app.t("o.addElement") : app.t("o.addKey")}</button>
      </div>
    </div>
  {/if}
{:else}
  <div class="leaf">
    <span class="mono muted key">{label}</span>
    <input
      class="mono value"
      class:invalid={!valid}
      aria-label={label}
      value={draft ?? raw}
      oninput={(e) => (draft = e.currentTarget.value)}
      onblur={commit}
      onkeydown={(e) => {
        if (e.key === "Enter") commit()
        else if (e.key === "Escape") draft = null
      }}
    />
    {#if modified}
      <span class="dot" title={app.t("c.modified")}>●</span>
      <button type="button" class="icon ghost" aria-label={app.t("c.revert")} title={app.t("c.revert")} onclick={() => app.act({ action: "revertValue", path })}><Icon name="reset" /></button>
    {/if}
  </div>
{/if}

<style>
  .head {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: transparent;
    padding: 2px 4px;
    min-height: 30px;
    font-size: 13px;
    color: var(--text);
    border-radius: var(--r-sm);
  }
  .head:hover {
    background: var(--surface-alt);
  }
  .head :global(.icon) {
    transition: transform 0.15s ease;
  }
  .head.open :global(.icon) {
    transform: rotate(90deg);
  }
  .kids {
    padding-inline-start: 18px;
    border-inline-start: 1px solid var(--border);
    margin-inline-start: 10px;
  }
  .kid {
    display: flex;
    align-items: flex-start;
    gap: 2px;
  }
  .x {
    min-width: 28px;
    min-height: 28px;
    padding: 4px;
    margin-top: 1px;
  }
  .sub {
    min-width: 0;
    flex: 1;
  }
  .leaf {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
  }
  .key {
    font-size: 13px;
  }
  .value {
    width: 260px;
    max-width: 100%;
    font-size: 13px;
    min-height: 26px;
    padding: 2px 6px;
  }
  .value.invalid {
    color: var(--danger);
    border-color: var(--danger);
  }
  .dot {
    color: var(--mark);
  }
  .add {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 4px 0 6px;
  }
  .add input {
    width: 160px;
    min-height: 26px;
    font-size: 13px;
  }
</style>
