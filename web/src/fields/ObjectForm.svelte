<script lang="ts">
  import ObjectForm from "./ObjectForm.svelte"
  import NumField from "./NumField.svelte"
  import TextField from "./TextField.svelte"
  import BoolField from "./BoolField.svelte"
  import Row from "./Row.svelte"
  import Fold from "../components/Fold.svelte"
  import { at, describe, isIntFlag, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { short } from "../lib/format"

  interface Props {
    path: Path
    depth?: number
  }
  let { path, depth = 0 }: Props = $props()
  const v = $derived(at(app.root, path))
  const entries = $derived.by((): [string | number, any, string][] => {
    if (Array.isArray(v)) return v.slice(0, 500).map((x, i) => [i, x, `#${i + 1}`])
    if (v && typeof v === "object") return Object.entries(v).map(([k, x]) => [k, x, ""])
    return []
  })
</script>

{#if entries.length === 0}
  <p class="muted">{app.t("c.empty")}</p>
{/if}
{#each entries as [key, child, index] (key)}
  {@const p = [...path, key]}
  {@const label = index || describe(app.lang, p).label}
  {#if typeof child === "number" && typeof key === "string" && isIntFlag(key) && (child === 0 || child === 1)}
    <BoolField path={p} {label} intFlag />
  {:else if typeof child === "number"}
    <NumField path={p} {label} />
  {:else if typeof child === "string"}
    <TextField path={p} {label} />
  {:else if typeof child === "boolean"}
    <BoolField path={p} {label} />
  {:else if child === null}
    <Row path={p} {label}><span class="muted">{app.t("c.empty")}</span></Row>
  {:else if Array.isArray(child) && child.length <= 12 && child.every((x) => x === null || typeof x !== "object")}
    <Row path={p} {label}>
      <span class="mono">{child.length ? child.map(short).join(", ") : app.t("c.empty")}</span>
    </Row>
  {:else}
    <Fold title={Array.isArray(child) ? `${label} [${child.length}]` : label} help={index ? "" : describe(app.lang, p).help}>
      {#if depth < 12}<ObjectForm path={p} depth={depth + 1} />{/if}
    </Fold>
  {/if}
{/each}
