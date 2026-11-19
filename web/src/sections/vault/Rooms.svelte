<script lang="ts">
  import Icon from "../../components/Icon.svelte"
  import MapView from "../../rooms/MapView.svelte"
  import AddPanel from "../../rooms/AddPanel.svelte"
  import RoomEditor from "../../rooms/RoomEditor.svelte"
  import { rooms as roomsUi } from "../../rooms/rooms.svelte"
  import { app } from "../../lib/state.svelte"
  import { ui } from "../../lib/ui.svelte"

  const rocks = $derived<any[]>(app.root?.vault?.rocks ?? [])
  const rockIndex = $derived(ui.rockSel ? rocks.findIndex((r) => r?.r === ui.rockSel![0] && r?.c === ui.rockSel![1]) : -1)
  const roomExists = $derived(ui.roomSel !== null && (app.root?.vault?.rooms ?? []).some((r: any) => r?.deserializeID === ui.roomSel))
</script>

<div class="layout">
  <div class="map"><MapView /></div>
  <aside class="side">
    {#if roomsUi.add}
      <AddPanel />
    {:else if ui.rockSel && rockIndex >= 0}
      <section class="panel">
        <h3>{app.t("map.rock")}</h3>
        <p class="muted small">{app.t("room.position", { row: ui.rockSel[0], col: ui.rockSel[1] })}</p>
        <p class="muted">{app.t("map.rockHelp")}</p>
        <button
          type="button"
          onclick={() => {
            if (app.act({ action: "removeRock", index: rockIndex })) ui.rockSel = null
          }}><Icon name="trash" /> {app.t("map.removeRock")}</button
        >
      </section>
    {:else if roomExists && ui.roomSel !== null}
      <RoomEditor id={ui.roomSel} />
    {:else}
      <section class="panel"><p class="muted">{app.t("vault.selectRoom")}</p></section>
    {/if}
    <section class="panel">
      <h3>{app.t("vault.rocks")}</h3>
      <p>{app.t("vault.rocksCount", { n: rocks.length })}</p>
      <p class="muted small">{app.t("vault.rocksHelp")}</p>
      {#if rocks.length}
        <button
          type="button"
          onclick={() => {
            app.act({ action: "removeAllRocks" })
            ui.rockSel = null
          }}><Icon name="trash" /> {app.t("vault.removeAllRocks")}</button
        >
      {/if}
    </section>
  </aside>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(260px, 32%, 400px);
    gap: var(--s-md);
    align-items: start;
  }
  .side {
    position: sticky;
    top: var(--s-sm);
    max-height: calc(100vh - 80px);
    overflow: auto;
  }
  @media (max-width: 1000px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .side {
      position: static;
      max-height: none;
    }
  }
</style>
