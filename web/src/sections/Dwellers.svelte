<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import DwellerList from "../dwellers/DwellerList.svelte"
  import MassBar from "../dwellers/MassBar.svelte"
  import Card from "../dwellers/Card.svelte"
  import Pets from "../dwellers/Pets.svelte"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { arr, DWELLERS } from "../lib/vault"

  let main = $state<HTMLElement>()
  const exists = $derived(ui.dw.focus !== null && arr(app.root, DWELLERS).some((d) => d.serializeId === ui.dw.focus))

  $effect(() => {
    void ui.dw.focus
    if (app.settings.dwellerScroll === "top" && main) main.scrollIntoView({ block: "start" })
  })
</script>

<h1 class="section-title"><Icon name="users" /> {app.t("nav.dwellers")}</h1>
<div class="layout">
  <aside class="list"><DwellerList /></aside>
  <div class="main" bind:this={main}>
    {#if ui.dw.selected.size >= 2}
      <MassBar />
    {:else}
      <p class="muted small">{app.t("dw.massHint")}</p>
    {/if}
    {#if exists && ui.dw.focus !== null}
      {#key ui.dw.focus}
        <Card id={ui.dw.focus} />
      {/key}
    {:else}
      <p class="muted">{app.t("dw.none")}</p>
    {/if}
    <Pets />
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
    gap: var(--s-md);
    align-items: start;
  }
  .list {
    position: sticky;
    top: var(--s-sm);
  }
  .main {
    min-width: 0;
    scroll-margin-top: var(--s-md);
  }
  @media (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .list {
      position: static;
    }
  }
</style>
