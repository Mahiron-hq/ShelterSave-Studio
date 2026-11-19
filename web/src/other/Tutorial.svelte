<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import { describe } from "../lib/core"
  import { app } from "../lib/state.svelte"

  const o = $derived(app.root?.tutorialManager)
  const hints = $derived(o && typeof o === "object" ? Object.keys(o).filter((k) => k.startsWith("Contextual")) : [])
</script>

{#if !o || typeof o !== "object"}
  <section class="panel"><p class="muted">{app.t("c.empty")}</p></section>
{:else}
  <section class="panel">
    <h3>{app.t("o.tutorialMain")}</h3>
    <p class="muted">{app.t("o.tutorialMainHelp")}</p>
    <button type="button" title={app.t("o.completeTutorialHelp")} onclick={() => app.act({ action: "completeTutorial" })}>
      <Icon name="check" /> {app.t("o.completeTutorial")}
    </button>
  </section>
  <section class="panel">
    <h3>{app.t("o.hints", { n: hints.filter((k) => o[k] === true).length, all: hints.length })}</h3>
    <p class="muted">{app.t("o.hintsHelp")}</p>
    <div class="row-wrap">
      <button type="button" onclick={() => app.act({ action: "setHints", shown: true })}>{app.t("o.hintsAllShown")}</button>
      <button type="button" onclick={() => app.act({ action: "setHints", shown: false })}>{app.t("o.hintsAllNew")}</button>
    </div>
    {#each hints as k (k)}
      <BoolField path={["tutorialManager", k]} label={describe(app.lang, ["tutorialManager", k]).label} help={app.t("o.hintHelp")} />
    {/each}
  </section>
{/if}
