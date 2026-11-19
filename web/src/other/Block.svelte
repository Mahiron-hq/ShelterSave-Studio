<script lang="ts">
  import Fold from "../components/Fold.svelte"
  import ObjectForm from "../fields/ObjectForm.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import NumField from "../fields/NumField.svelte"
  import TextField from "../fields/TextField.svelte"
  import Row from "../fields/Row.svelte"
  import { describe } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props {
    titleKey: string
    roots: string[]
  }
  let { titleKey, roots }: Props = $props()
</script>

<section class="panel">
  <h3>{app.t(titleKey)}</h3>
  {#each roots as key (key)}
    {@const v = app.root?.[key]}
    {#if v !== undefined}
      {#if v !== null && typeof v === "object"}
        <Fold title={describe(app.lang, [key]).label} help={describe(app.lang, [key]).help}>
          <ObjectForm path={[key]} />
        </Fold>
      {:else if v === null}
        <Row path={[key]}><span class="muted">{app.t("c.empty")}</span></Row>
      {:else if typeof v === "boolean"}
        <BoolField path={[key]} />
      {:else if typeof v === "number"}
        <NumField path={[key]} />
      {:else}
        <TextField path={[key]} />
      {/if}
    {/if}
  {/each}
</section>
