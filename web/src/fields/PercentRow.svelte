<script lang="ts">
  import Row from "./Row.svelte"
  import NumPick from "./NumPick.svelte"
  import { at, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props {
    path: Path
    label: string
    help?: string
  }
  let { path, label, help = "" }: Props = $props()
  const v = $derived(at(app.root, path))
  const pct = $derived(typeof v === "number" ? Math.round(v * 100) : null)
</script>

{#if pct !== null}
  <Row {path} {label} {help} computed>
    <NumPick
      value={pct}
      {label}
      lo={0}
      hi={Math.max(100, pct)}
      slider={[0, 100]}
      oncommit={(n) => app.act({ action: "setNumber", path, value: n / 100 })}
    />
    <span class="muted">%</span>
  </Row>
{/if}
