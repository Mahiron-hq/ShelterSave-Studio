<script lang="ts">
  import NumField from "../fields/NumField.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import TicksField from "../fields/TicksField.svelte"
  import { describe } from "../lib/core"
  import { app } from "../lib/state.svelte"

  const base = ["StatsWindow", "vaultData"]
  const hidden = ["previousVResources", "vaultCreatedBeforeUpdate", "weaponFactoryBuilt", "outfitFactoryBuilt", "measuringSince", "collectedRes"]
  const o = $derived(app.root?.StatsWindow?.vaultData)
</script>

{#if !o || typeof o !== "object"}
  <section class="panel"><p class="muted">{app.t("c.empty")}</p></section>
{:else}
  <section class="panel">
    <h3>{app.t("o.stats")}</h3>
    <p class="muted">{app.t("o.statsHelp")}</p>
    {#if typeof o.measuringSince === "number"}
      <TicksField path={[...base, "measuringSince"]} label={describe(app.lang, [...base, "measuringSince"]).label} />
    {/if}
    {#each Object.entries(o) as [k, v] (k)}
      {#if !hidden.includes(k)}
        {#if typeof v === "number"}<NumField path={[...base, k]} />{:else if typeof v === "boolean"}<BoolField path={[...base, k]} />{/if}
      {/if}
    {/each}
  </section>
  {#if o.collectedRes && typeof o.collectedRes === "object"}
    <section class="panel">
      <h3>{app.t("o.collectedRes")}</h3>
      <p class="muted">{app.t("o.collectedResHelp")}</p>
      {#each Object.keys(o.collectedRes) as k (k)}
        {#if app.t(`res.${k}`) !== `res.${k}`}
          <NumField path={[...base, "collectedRes", k]} label={app.t(`res.${k}`)} />
        {/if}
      {/each}
    </section>
  {/if}
{/if}
