<script lang="ts">
  import Row, { type Opts } from "./Row.svelte"
  import Pick from "../components/Pick.svelte"
  import { at, describe, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props extends Opts {
    path: Path
    choices: { value: string; label: string }[]
    /** Values are numbers stored as integers. */
    int?: boolean
  }
  let { path, choices, int = false, ...o }: Props = $props()
  const label = $derived(o.label ?? describe(app.lang, path).label)
  const cur = $derived(at(app.root, path))
  const items = $derived.by(() => {
    const c = String(cur ?? "")
    return choices.some((x) => x.value === c) || cur === undefined ? choices : [...choices, { value: c, label: c }]
  })
</script>

<Row {path} {...o} choice>
  <Pick
    {label}
    value={cur === undefined ? null : String(cur)}
    {items}
    onchange={(v) =>
      int ? app.act({ action: "setNumber", path, value: Number(v) }) : app.act({ action: "setText", path, value: v })}
  />
</Row>
