<script lang="ts">
  import Row, { type Opts } from "./Row.svelte"
  import NumEditor from "./NumEditor.svelte"
  import { describe, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props extends Opts {
    path: Path
    readOnly?: boolean
  }
  let { path, readOnly = false, ...o }: Props = $props()
  const label = $derived(o.label ?? describe(app.lang, path).label)
</script>

<Row {path} {...o}>
  <NumEditor {path} min={o.min} max={o.max} {readOnly} {label} />
</Row>
