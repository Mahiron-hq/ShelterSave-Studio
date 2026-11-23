<script lang="ts">
  import type { Snippet } from "svelte"
  import { Tooltip, TooltipContent, TooltipTrigger } from "@ui-sdk/svelte/tooltip"
  import Icon from "../components/Icon.svelte"
  import { core, describe, pathStr, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { fmtNum, hexColor, short } from "../lib/format"

  export interface Opts {
    label?: string
    help?: string
    unit?: string
    min?: number
    max?: number
    choice?: boolean
    computed?: boolean
  }

  interface Props extends Opts {
    path?: Path
    children: Snippet
    id?: string
  }
  let { path, label, help, unit = "", min, max, choice = false, computed = false, children, id }: Props = $props()

  const ps = $derived(path ? pathStr(path) : "")
  const info = $derived(path ? describe(app.lang, path) : { label: "", help: "", known: true })
  const state = $derived.by(() => {
    void app.rev
    if (!path) return { modified: false, original: null, out: false }
    const ed = core()
    const origRaw = ed.original(ps)
    const modified = ed.modified(ps) && origRaw !== undefined
    const curRaw = ed.raw(ps)
    const cur = curRaw !== undefined ? JSON.parse(curRaw) : undefined
    let original: string | null = null
    if (modified && !computed && origRaw !== undefined) {
      const o = JSON.parse(origRaw)
      const key = path.at(-1)
      original =
        typeof key === "string" && key.endsWith("Color") && typeof o === "number"
          ? hexColor(o)
          : typeof o === "number"
            ? fmtNum(o)
            : short(o)
    }
    const out = typeof cur === "number" && min !== undefined && max !== undefined && (cur < min || cur > max)
    return { modified, original, out }
  })
  const fullHelp = $derived.by(() => {
    let h = help || info.help
    if (min !== undefined && max !== undefined && !choice) {
      h = (h ? h + "\n\n" : "") + app.t("c.range", { min: fmtNum(min), max: fmtNum(max) })
    }
    return h
  })
  const shownLabel = $derived(label ?? info.label)
  // The help mark stays glued to the last word when a long label wraps.
  const split = $derived.by(() => {
    const t = shownLabel.trimEnd()
    const i = t.lastIndexOf(" ")
    return i < 0 ? ["", t] : [t.slice(0, i + 1), t.slice(i + 1)]
  })
</script>

<div class="row" class:modified={state.modified}>
  <div class="label">
    <span class="name" {id}
      >{split[0]}<span class="tail"
        >{split[1]}{#if fullHelp}
          <Tooltip>
            <TooltipTrigger class="help" aria-label={app.t("tb.help")}>
              <Icon name="help" />
            </TooltipTrigger>
            <TooltipContent class="help-text">{fullHelp}</TooltipContent>
          </Tooltip>
        {/if}</span
      ></span
    >
    {#if app.expert && ps}
      <span class="mono muted path">{ps}</span>
    {/if}
  </div>
  <div class="body">
    <div class="line">
      {@render children()}
      {#if unit}<span class="muted unit">{unit}</span>{/if}
      {#if state.modified}
        <span class="mark">{app.t("c.modified")}</span>
        <button
          type="button"
          class="icon ghost"
          aria-label={app.t("c.revert")}
          title={app.t("c.revert")}
          onclick={() => app.act({ action: "revertValue", path })}><Icon name="reset" /></button
        >
      {/if}
    </div>
    {#if state.original !== null}
      <p class="muted small">{app.t("c.original", { value: state.original })}</p>
    {/if}
    {#if state.out}
      <p class="warn small"><Icon name="warn" /> {app.t("c.outOfNorm", { min: fmtNum(min ?? 0), max: fmtNum(max ?? 0) })}</p>
    {/if}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(140px, 260px) minmax(0, 1fr);
    gap: var(--s-md);
    padding: 6px 8px;
    border-radius: var(--r-md);
    border-inline-start: 3px solid transparent;
  }
  .row.modified {
    background: var(--mark-soft);
    border-inline-start-color: var(--mark);
  }
  .label {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    align-self: start;
    min-height: 32px;
    gap: 2px 6px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .tail {
    white-space: nowrap;
  }
  .tail :global(.help) {
    margin-inline-start: 4px;
    vertical-align: -2px;
  }
  .path {
    font-size: 11px;
    flex-basis: 100%;
    overflow-wrap: anywhere;
  }
  .label :global(.help) {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    color: var(--muted);
    cursor: help;
  }
  :global(.help-text) {
    white-space: pre-line;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .unit {
    white-space: nowrap;
  }
  .mark {
    color: var(--mark-text);
    font-size: 12px;
  }
  p {
    margin: 2px 0 0;
  }
  .warn {
    color: var(--warning);
    display: flex;
    gap: 4px;
    align-items: center;
  }
  @container (max-width: 520px) {
    .row {
      grid-template-columns: minmax(0, 1fr);
      gap: 2px;
    }
  }
</style>
