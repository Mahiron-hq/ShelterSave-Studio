<script lang="ts">
  import Row from "./Row.svelte"
  import NumPick from "./NumPick.svelte"
  import { app } from "../lib/state.svelte"
  import { duration } from "../lib/format"
  import type { Path } from "../lib/core"

  interface Props {
    path?: Path
    label: string
    help?: string
    secs: number
    /** Seconds in one step: 60 = minutes, 3600 = hours. */
    unit?: number
    onset: (secs: number) => void
  }
  let { path, label, help = "", secs, unit = 60, onset }: Props = $props()

  const n = $derived(Math.ceil(secs / unit))
  const max = $derived(Math.max(n, unit >= 3600 ? 30 * 24 : unit >= 60 ? 24 * 60 : 3600))
  const tail = $derived.by(() => {
    const key = unit >= 3600 ? "o.hours" : unit >= 60 ? "o.minutes" : "o.seconds"
    const t = app.t(key)
    return secs >= 3600 || unit >= 3600 || (unit < 60 && secs >= 60) ? `${t} (${duration(secs)})` : t
  })
</script>

<Row {path} {label} {help} computed>
  <NumPick value={n} {label} lo={0} hi={max} slider={[0, max]} oncommit={(v) => onset(v * unit)} />
  <span class="muted">{tail}</span>
</Row>
