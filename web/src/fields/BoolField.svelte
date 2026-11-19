<script lang="ts">
  import Row, { type Opts } from "./Row.svelte"
  import Check from "../components/Check.svelte"
  import { at, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props extends Opts {
    path: Path
    /** Stored as 0/1 instead of true/false. */
    intFlag?: boolean
  }
  let { path, intFlag = false, ...o }: Props = $props()
  const cur = $derived(at(app.root, path))
  const on = $derived(intFlag ? cur === 1 : cur === true)
</script>

<Row {path} {...o}>
  <Check
    checked={on}
    label={on ? app.t("c.yesno.true") : app.t("c.yesno.false")}
    onchange={(v) =>
      intFlag ? app.act({ action: "setInt", path, value: v ? 1 : 0 }) : app.act({ action: "setBool", path, value: v })}
  />
</Row>
