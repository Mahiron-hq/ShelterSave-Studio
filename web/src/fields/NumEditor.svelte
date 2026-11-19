<script lang="ts">
  import NumPick from "./NumPick.svelte"
  import { core, pathStr, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { fmtNum } from "../lib/format"

  interface Props {
    path: Path
    min?: number
    max?: number
    readOnly?: boolean
    label: string
  }
  let { path, min, max, readOnly = false, label }: Props = $props()

  const raw = $derived.by(() => {
    void app.rev
    return core().raw(pathStr(path))
  })
  const isNum = $derived(raw !== undefined && /^-?\d/.test(raw))
  const int = $derived(isNum && !/[.eE]/.test(raw ?? ""))
  const value = $derived(isNum ? Number(raw) : 0)
  const big = $derived(Math.abs(value) >= 1e12)
  // The norm is widened to the current value so showing a value never clamps it.
  const lo = $derived(app.expert || min === undefined ? undefined : Math.min(min, value))
  const hi = $derived(app.expert || max === undefined ? undefined : Math.max(max, value))
  const slider = $derived<[number, number] | null>(
    min !== undefined && max !== undefined ? [Math.min(min, value), Math.max(max, value)] : null,
  )
  let text = $state<string | null>(null)

  function commit(v: number) {
    if (int) app.act({ action: "setInt", path, value: v })
    else app.act({ action: "setNumber", path, value: v })
  }

  function commitText() {
    if (text === null) return
    const t = text.trim()
    text = null
    if (t === raw) return
    if (/^-?\d+$/.test(t) && int) app.act({ action: "setInt", path, value: "__RAW__" }, t)
    else {
      const v = Number(t.replace(",", "."))
      if (Number.isFinite(v)) app.act({ action: "setNumber", path, value: v })
      else app.toastError(app.t("aerr.wrongType"))
    }
  }
</script>

{#if !isNum}
  <span class="mono">{raw ?? ""}</span>
{:else if readOnly}
  <span class="mono">{fmtNum(value)}</span>
{:else if big}
  <input
    class="mono big"
    aria-label={label}
    value={text ?? raw}
    class:invalid={text !== null && !Number.isFinite(Number(text.trim().replace(",", ".")))}
    oninput={(e) => (text = e.currentTarget.value)}
    onblur={commitText}
    onkeydown={(e) => e.key === "Enter" && commitText()}
  />
{:else}
  <NumPick {value} {label} {lo} {hi} {slider} {int} oncommit={commit} />
{/if}

<style>
  .big {
    width: 22ch;
  }
</style>
