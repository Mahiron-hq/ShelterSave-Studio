<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Pick from "../components/Pick.svelte"
  import Seg from "../components/Seg.svelte"
  import Row from "../fields/Row.svelte"
  import { catalog, consts, core, describe, query, roomShape } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { rooms as roomsUi } from "./rooms.svelte"

  const add = $derived(roomsUi.add!)
  const kinds = $derived.by(() => {
    const c = consts()
    const built = new Set(query<{ rooms: { kind: string }[] }>("map").rooms.map((r) => r.kind))
    return (catalog().rooms as any[])
      .filter((r) => !c.fixedRooms.includes(r.type))
      .filter((r) => app.expert || (!c.notBuildable.includes(r.type) && !(c.uniqueRooms.includes(r.type) && built.has(r.type))))
      .map((r) => ({ kind: r.type as string, names: core().levelNames(r.type) as string[] }))
      .sort((a, b) => (a.names[0] ?? a.kind).localeCompare(b.names[0] ?? b.kind))
  })
  const shape = $derived(roomShape(add.kind))
  const maxLevel = $derived(shape.maxLevel[shape.merges.indexOf(add.merge)] ?? 1)
  const col = $derived((void app.rev, query<number>("addCol", { kind: add.kind, row: add.row, at: add.at, merge: add.merge, rock: add.rock })))
  const locked = $derived((void app.rev, !add.rock && query<string | null>("roomLock", { kind: add.kind }) !== null))
  const status = $derived.by(() => {
    void app.rev
    if (add.rock) {
      const e = query<string | null>("fits", { row: add.row, col, width: 2 })
      return e === null ? { s: "ok", key: "map.rockOk" } : { s: "bad", key: e === "outOfGrid" ? "map.outOfGrid" : "map.overlap" }
    }
    const e = query<string | null>("checkSpot", { kind: add.kind, row: add.row, col, merge: add.merge })
    if (e === null) return { s: "ok", key: "map.ok" }
    if (e === "disconnected") return app.expert ? { s: "warn", key: "map.disconnectedExpert" } : { s: "bad", key: "map.disconnected" }
    return { s: "bad", key: e === "outOfGrid" ? "map.outOfGrid" : "map.overlap" }
  })

  // Keeps the chosen size and level valid for the chosen type.
  $effect(() => {
    if (!kinds.some((k) => k.kind === add.kind) && kinds[0]) roomsUi.add = { ...add, kind: kinds[0].kind }
    else if (!shape.merges.includes(add.merge)) {
      const m = [...shape.merges].sort((a, b) => Math.abs(a - add.merge) - Math.abs(b - add.merge))[0] ?? 1
      roomsUi.add = { ...add, merge: m }
    } else if (add.level > maxLevel || add.level < 1) roomsUi.add = { ...add, level: Math.min(Math.max(add.level, 1), maxLevel) }
  })

  function set(patch: Partial<typeof add>) {
    roomsUi.add = { ...add, ...patch }
  }

  function build() {
    const a = add
    if (a.rock) {
      if (app.act({ action: "addRock", row: a.row, col })) {
        roomsUi.last = { ...a }
        roomsUi.add = null
        ui.rockSel = [a.row, col]
      }
      return
    }
    if (app.act({ action: "addRoom", kind: a.kind, row: a.row, col, merge: a.merge, level: a.level, unlock: locked })) {
      if (!a.copy) roomsUi.last = { ...a }
      roomsUi.add = null
      ui.roomSel = query<{ rooms: { id: number; row: number; col: number }[] }>("map").rooms.find((r) => r.row === a.row && r.col === col)?.id ?? null
    }
  }
  const level = $derived(describe(app.lang, ["vault", "rooms", 0, "level"]))
</script>

<section class="panel">
  <h3>{app.t(add.copy ? "map.copy" : "map.new")}</h3>
  <p class="muted small">{app.t("map.position", { row: add.row, col })}</p>
  <Seg
    label={app.t("map.new")}
    value={add.rock ? "rock" : "room"}
    options={[
      { value: "room", label: app.t("map.placeRoom") },
      { value: "rock", label: app.t("map.placeRock"), title: app.t("map.placeRockHelp") },
    ]}
    onchange={(v) => set({ rock: v === "rock" })}
  />
  <Row label={app.t("map.row")}>
    <Pick
      label={app.t("map.row")}
      value={String(add.row)}
      items={Array.from({ length: query<{ rows: number }>("map").rows }, (_, i) => ({ value: String(i), label: String(i) }))}
      onchange={(v) => set({ row: Number(v) })}
    />
    <span class="muted">{app.t("map.col")}</span>
    <Pick
      label={app.t("map.col")}
      value={String(add.at)}
      items={Array.from({ length: query<{ cols: number }>("map").cols }, (_, i) => ({ value: String(i), label: String(i) }))}
      onchange={(v) => set({ at: Number(v) })}
    />
  </Row>
  {#if add.rock}
    <p class="muted">{app.t("map.placeRockHelp")}</p>
  {:else}
    <Row label={app.t("map.type")} help={app.t("map.typeHelp")}>
      <Pick
        label={app.t("map.type")}
        value={add.kind}
        items={kinds.map((k) => ({
          value: k.kind,
          label: [k.names.join(" · "), (void app.rev, query("roomLock", { kind: k.kind })) ? app.t("map.lockedShort") : ""].filter(Boolean).join("  ·  "),
        }))}
        onchange={(v) => set({ kind: v })}
      />
    </Row>
    {#if shape.merges.length > 1}
      <Row label={app.t("map.size")} help={app.t("map.sizeHelp")}>
        <Seg label={app.t("map.size")} value={add.merge} options={shape.merges.map((m) => ({ value: m, label: String(m) }))} onchange={(m) => set({ merge: m })} />
      </Row>
    {/if}
    {#if maxLevel > 1}
      <Row label={level.label} help={level.help}>
        <Seg
          label={level.label}
          value={add.level}
          options={Array.from({ length: maxLevel }, (_, i) => ({ value: i + 1, label: String(i + 1) }))}
          onchange={(l) => set({ level: l })}
        />
      </Row>
    {/if}
  {/if}
  <p class="status {status.s}">
    <Icon name={status.s === "ok" ? "check" : status.s === "warn" ? "warn" : "x"} />
    {app.t(status.key)}
  </p>
  {#if locked}
    <p class="status bad"><Icon name="lock" /> {app.t("map.locked", { room: core().kindName(add.kind, 1) })}</p>
  {/if}
  <div class="row-wrap">
    <button type="button" class="primary" disabled={status.s === "bad"} title={locked ? app.t("map.unlockBuildHelp") : undefined} onclick={build}>
      <Icon name="plus" />
      {app.t(locked ? "map.unlockBuild" : add.rock ? "map.placeRockButton" : "map.build")}
    </button>
    <button type="button" onclick={() => (roomsUi.add = null)}>{app.t("c.cancel")}</button>
  </div>
</section>

<style>
  .status {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .status.ok {
    color: var(--success);
  }
  .status.warn {
    color: var(--warning);
  }
  .status.bad {
    color: var(--danger);
  }
</style>
