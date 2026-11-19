<script lang="ts">
  import Row, { type Opts } from "./Row.svelte"
  import { at, describe, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props extends Opts {
    path: Path
  }
  let { path, ...o }: Props = $props()
  const label = $derived(o.label ?? describe(app.lang, path).label)
  const cur = $derived(at(app.root, path))
  let text = $state<string | null>(null)

  function commit() {
    if (text === null) return
    const v = text
    text = null
    if (v !== cur) app.act({ action: "setText", path, value: v })
  }
</script>

<Row {path} {...o}>
  {#if typeof cur === "string"}
    <input
      type="text"
      aria-label={label}
      value={text ?? cur}
      oninput={(e) => (text = e.currentTarget.value)}
      onblur={commit}
      onkeydown={(e) => e.key === "Enter" && commit()}
    />
  {:else}
    <span class="mono">{cur ?? ""}</span>
  {/if}
</Row>

<style>
  input {
    width: min(260px, 100%);
  }
</style>
