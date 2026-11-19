<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Seg from "../components/Seg.svelte"
  import Row from "../fields/Row.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import { consts, pathStr, query, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props {
    room: any
    path: Path
  }
  let { room, path }: Props = $props()
  const c = consts()
  const id = $derived(room.deserializeID as number)
  const max = $derived((void app.rev, query<number | null>("roomMaxHealth", { path: pathStr(path) })))
  const burning = $derived(room.currentStateName === "RoomEmergency")
  const mode = $derived(typeof room.currentState?.mode === "number" ? room.currentState.mode : -1)
  const dmg = $derived(room.roomHealth?.damageValue)
  let kind = $state(c.roomIncidents[0])
  let strength = $state(45)

  function incName(m: number): string {
    const k = `inc.mode.${m}`
    const s = app.t(k)
    return s === k ? String(m) : s
  }
</script>

{#if max !== null && !c.noIncidentRooms.includes(room.type)}
  <Row label={app.t("room.incident")} help={app.t("o.incidentStartHelp")}>
    <div class="col">
      {#if burning}
        <p class="danger"><Icon name="alert" /> {incName(mode)}</p>
        {#if typeof dmg === "number" && c.roomIncidents.includes(mode)}
          {@const pct = Math.min(100, Math.max(1, Math.round((dmg / max) * 100)))}
          <span class="row-wrap">
            {app.t("o.incidentStrength")}
            <NumPick value={pct} label={app.t("o.incidentStrength")} lo={1} hi={100} slider={[1, 100]} oncommit={(v) => app.act({ action: "setIncidentStrength", room: id, value: v / 100 })} />
            <span class="muted">%</span>
          </span>
        {/if}
        <button type="button" title={app.t("o.incidentFinishHelp")} onclick={() => app.act({ action: "finishEmergency" })}>
          <Icon name="check" /> {app.t("o.incidentFinish")}
        </button>
      {:else if app.root?.vault?.emergencyData?.active}
        <p class="muted small">{app.t("room.incidentBusy")}</p>
      {:else}
        <Seg label={app.t("room.incident")} value={kind} options={c.roomIncidents.map((m) => ({ value: m, label: incName(m) }))} onchange={(v) => (kind = v)} />
        <span class="row-wrap" title={app.t("o.incidentStrengthHelp")}>
          {app.t("o.incidentStrength")}
          <NumPick value={strength} label={app.t("o.incidentStrength")} lo={10} hi={100} slider={[10, 100]} oncommit={(v) => (strength = v)} />
          <span class="muted">%</span>
        </span>
        <button
          type="button"
          title={app.t("o.incidentStartButtonHelp")}
          onclick={() => app.act({ action: "startIncident", room: id, kind, strength: strength / 100 })}
        >
          <Icon name="alert" /> {app.t("o.incidentStartButton")}
        </button>
      {/if}
    </div>
  </Row>
{/if}

<style>
  .col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
  }
  .danger {
    color: var(--danger);
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0;
  }
</style>
