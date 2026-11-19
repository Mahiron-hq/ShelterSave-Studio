<script lang="ts">
  import Row, { type Opts } from "./Row.svelte"
  import { core, pathStr, ticksFormat, ticksParse, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props extends Opts {
    path: Path
  }
  let { path, ...o }: Props = $props()
  const raw = $derived.by(() => {
    void app.rev
    return core().raw(pathStr(path)) ?? ""
  })
  const shown = $derived(ticksFormat(raw) ?? raw)
  let text = $state<string | null>(null)
  const valid = $derived(text === null || ticksParse(text, raw) !== null)

  function commit() {
    if (text === null) return
    const t = text
    text = null
    if (t === shown) return
    const v = ticksParse(t, raw)
    if (v === null) app.toastError(app.t("time.invalid"))
    else app.act({ action: "setInt", path, value: "__RAW__" }, v)
  }
</script>

<Row {path} {...o}>
  <input
    class="mono"
    class:invalid={!valid}
    aria-label={o.label}
    title={app.t("time.local")}
    value={text ?? shown}
    oninput={(e) => (text = e.currentTarget.value)}
    onblur={commit}
    onkeydown={(e) => e.key === "Enter" && commit()}
  />
  {#if app.expert}<span class="mono muted small">{raw}</span>{/if}
</Row>

<style>
  input {
    width: 22ch;
  }
</style>
