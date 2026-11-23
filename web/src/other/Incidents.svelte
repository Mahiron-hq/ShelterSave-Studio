<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Pick from "../components/Pick.svelte"
  import Seg from "../components/Seg.svelte"
  import Row from "../fields/Row.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import NumField from "../fields/NumField.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import TaskTimer from "../fields/TaskTimer.svelte"
  import { consts, core, pathStr, query } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { arr, ROOMS } from "../lib/vault"

  const c = consts()
  const em = $derived(app.root?.vault?.emergencyData ?? {})
  const active = $derived(em.active === true)
  const rooms = $derived(arr(app.root, ROOMS))
  const burning = $derived(rooms.map((r, i) => ({ r, i })).filter(({ r }) => r?.currentStateName === "RoomEmergency"))
  const roomTitle = (r: any) => `${core().kindName(r.type, r.level ?? 1)}  ·  ${app.t("room.position", { row: r.row ?? 0, col: r.col ?? 0 })}`
  const maxHp = (i: number) => (void app.rev, query<number | null>("roomMaxHealth", { path: pathStr([...ROOMS, i]) }))

  let pickRoom = $state<number | null>(null)
  let pickKind = $state(c.roomIncidents[0])
  let strength = $state(45)
  const candidates = $derived(
    rooms
      .map((r, i) => ({ r, i }))
      .filter(({ r, i }) => !c.noIncidentRooms.includes(r?.type) && maxHp(i) !== null)
      .map(({ r }) => ({ value: String(r.deserializeID), label: roomTitle(r) })),
  )
  const task = $derived<number>(typeof em.randomEventTaskId === "number" ? em.randomEventTaskId : -1)
  const left = $derived.by(() => {
    const t = (app.root?.taskMgr?.tasks ?? []).find((x: any) => x?.id === task)
    const now = app.root?.taskMgr?.time
    return t && typeof now === "number" && typeof t.endTime === "number" ? Math.max(0, t.endTime - now) : null
  })
  const season = $derived(typeof app.root?.seasonId === "string" && app.root.seasonId !== "" && em.currentWave !== undefined)
  const base = ["vault", "emergencyData"]
</script>

<h3>{app.t("o.incidents")}</h3>
<p class="muted">{app.t("o.incidentsHelp")}</p>
<p>
  <span class="muted">{app.t("o.status")}</span>
  {active ? app.t("o.incidentNow", { kind: core().incidentName(em.mode ?? -1) }) : burning.length ? app.t("o.incidentRooms") : app.t("o.incidentNone")}
</p>
{#each burning as { r, i } (r.deserializeID)}
  {@const mode = r.currentState?.mode ?? -1}
  {@const max = maxHp(i)}
  <p class="danger"><Icon name="alert" /> {roomTitle(r)}  ·  {core().incidentName(mode)}</p>
  {#if typeof r.roomHealth?.damageValue === "number" && max && c.roomIncidents.includes(mode)}
    <Row label={app.t("o.incidentStrength")} help={app.t("o.incidentStrengthHelp")}>
      <NumPick
        value={Math.min(100, Math.max(1, Math.round((r.roomHealth.damageValue / max) * 100)))}
        label={app.t("o.incidentStrength")}
        lo={1}
        hi={100}
        slider={[1, 100]}
        oncommit={(v) => app.act({ action: "setIncidentStrength", room: r.deserializeID, value: v / 100 })}
      />
      <span class="muted">%</span>
    </Row>
  {/if}
{/each}
{#if active && typeof em.raider?.life === "number" && typeof em.raider?.lifeMax === "number"}
  {@const max = Math.max(1, Math.round(em.raider.lifeMax))}
  <Row label={app.t("o.raidersHealth")} help={app.t("o.raidersHealthHelp")}>
    <NumPick
      value={Math.min(max, Math.max(1, Math.round(em.raider.life)))}
      label={app.t("o.raidersHealth")}
      lo={1}
      hi={max}
      slider={[1, max]}
      oncommit={(v) => app.act({ action: "setIncidentStrength", room: null, value: v })}
    />
    <span class="muted">{app.t("o.ofMax", { n: max })}</span>
  </Row>
{:else if active && burning.length === 0}
  <p class="muted">{app.t("o.incidentDoorNote")}</p>
{/if}
{#if active || burning.length}
  <button type="button" title={app.t("o.incidentFinishHelp")} onclick={() => app.act({ action: "finishEmergency" })}><Icon name="check" /> {app.t("o.incidentFinish")}</button>
{:else}
  <h4 title={app.t("o.incidentStartHelp")}>{app.t("o.incidentStart")}</h4>
  <div class="start">
    <Pick label={app.t("o.incidentPickRoom")} value={pickRoom === null ? null : String(pickRoom)} placeholder={app.t("o.incidentPickRoom")} items={candidates} onchange={(v) => (pickRoom = Number(v))} />
    <Seg label={app.t("o.incidentStart")} value={pickKind} options={c.roomIncidents.map((m) => ({ value: m, label: core().incidentName(m) }))} onchange={(v) => (pickKind = v)} />
    <Row label={app.t("o.incidentStrength")} help={app.t("o.incidentStrengthHelp")} unit="%">
          <NumPick value={strength} label={app.t("o.incidentStrength")} lo={10} hi={100} slider={[10, 100]} oncommit={(v) => (strength = v)} />
        </Row>
    <button
      type="button"
      disabled={pickRoom === null}
      title={app.t("o.incidentStartButtonHelp")}
      onclick={() => pickRoom !== null && app.act({ action: "startIncident", room: pickRoom, kind: pickKind, strength: strength / 100 })}
    >
      <Icon name="alert" /> {app.t("o.incidentStartButton")}
    </button>
  </div>
{/if}
{#if task >= 0}
  <TaskTimer id={task} label={app.t("o.incidentNextCheck")} help={app.t("o.incidentNextCheckHelp")} />
  {#if left !== null}
    <p class="row-wrap" title={app.t("o.incidentPauseHelp")}>
      {app.t("o.incidentPause")}
      {#each [[1, "o.pause1h"], [6, "o.pause6h"], [24, "o.pause24h"]] as [h, key] (key)}
        <button type="button" onclick={() => app.act({ action: "setTaskTime", id: task, seconds: (left ?? 0) + Number(h) * 3600 })}>{app.t(String(key))}</button>
      {/each}
    </p>
  {/if}
{/if}
{#if season}
  <h4 title={app.t("o.wavesHelp")}>{app.t("o.waves")}</h4>
  <NumField path={[...base, "currentWave"]} label={app.t("o.waveNumber")} min={0} max={100} />
  {#if em.failedPreviousWave !== undefined}<BoolField path={[...base, "failedPreviousWave"]} label={app.t("o.waveFailed")} help={app.t("o.waveFailedHelp")} />{/if}
  {#each [["currentWaveRemainingMelee", "o.waveMelee"], ["currentWaveRemainingGunner", "o.waveGunner"], ["currentWaveRemainingHeavy", "o.waveHeavy"]] as [k, label] (k)}
    {#if em[k] !== undefined}<NumField path={[...base, k]} label={app.t(label)} min={0} max={50} />{/if}
  {/each}
{/if}

<style>
  .danger {
    color: var(--danger);
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .start {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
    align-items: flex-start;
  }
  h4 {
    margin: var(--s-md) 0 var(--s-xs);
  }
</style>
