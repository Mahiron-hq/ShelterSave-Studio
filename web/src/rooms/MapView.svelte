<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import { catalog, consts, core, query, roomShape } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { rooms as roomsUi, type MapAdd } from "./rooms.svelte"

  interface Placed {
    id: number
    kind: string
    row: number
    col: number
    width: number
    merge: number
    level: number
  }
  interface MapData {
    rows: number
    cols: number
    rooms: Placed[]
    rocks: [number, number][]
    burning: number[]
  }

  const data = $derived.by((): MapData => {
    void app.rev
    return query<MapData>("map")
  })
  const shown = $derived.by(() => {
    const used = Math.max(0, ...data.rooms.map((r) => r.row), ...data.rocks.map((r) => r[0]))
    return Math.min(Math.max(used + 2, 1), data.rows)
  })
  const taken = $derived.by(() => {
    const s = new Set<string>()
    for (const r of data.rooms) for (let c = r.col; c < r.col + r.width; c++) s.add(`${r.row}:${c}`)
    for (const [r, c] of data.rocks) {
      s.add(`${r}:${c}`)
      s.add(`${r}:${c + 1}`)
    }
    return s
  })
  const classOf = (kind: string) => (catalog().rooms as any[]).find((r) => r.type === kind)?.class ?? ""

  let grid = $state<HTMLDivElement>()
  let hover = $state<[number, number] | null>(null)
  let drag = $state<{ id: number; grab: number; row: number; col: number; moved: boolean; keys: boolean } | null>(null)

  function cellAt(e: PointerEvent | MouseEvent): [number, number] | null {
    if (!grid) return null
    const b = grid.getBoundingClientRect()
    const col = Math.floor(((e.clientX - b.left) / b.width) * data.cols)
    const row = Math.floor(((e.clientY - b.top) / b.height) * shown)
    return row >= 0 && row < shown && col >= 0 && col < data.cols ? [row, col] : null
  }

  function spot(kind: string, row: number, col: number, merge: number, id: number | null): "ok" | "warn" | "bad" {
    const e = query<string | null>("checkSpot", { kind, row, col, merge, id })
    if (e === null) return "ok"
    return e === "disconnected" && app.expert ? "warn" : "bad"
  }

  function openAdd(row: number, col: number) {
    const prev = roomsUi.add && !roomsUi.add.copy ? roomsUi.add : roomsUi.last
    const next: MapAdd = {
      row,
      at: col,
      kind: prev?.kind ?? "LivingQuarters",
      merge: prev?.merge ?? 1,
      level: prev?.level ?? 1,
      rock: prev?.rock ?? false,
      copy: false,
    }
    roomsUi.add = next
    ui.roomSel = null
    ui.rockSel = null
  }

  function gridClick(e: MouseEvent) {
    if (e.target !== grid) return
    const c = cellAt(e)
    if (c && !taken.has(`${c[0]}:${c[1]}`)) openAdd(c[0], c[1])
  }

  function roomDown(e: PointerEvent, r: Placed) {
    if (consts().fixedRooms.includes(r.kind) || e.button !== 0) return
    const c = cellAt(e)
    if (!c) return
    drag = { id: r.id, grab: Math.min(Math.max(c[1] - r.col, 0), r.width - 1), row: r.row, col: r.col, moved: false, keys: false }
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  function roomMove(e: PointerEvent) {
    if (!drag || drag.keys) return
    const c = cellAt(e)
    if (!c) return
    const col = c[1] - drag.grab
    if (c[0] !== drag.row || col !== drag.col) drag = { ...drag, row: c[0], col, moved: true }
  }
  function roomUp(r: Placed) {
    const d = drag
    if (!d || d.keys) return
    drag = null
    if (d.moved && (d.row !== r.row || d.col !== r.col)) app.act({ action: "moveRoom", room: r.id, row: d.row, col: d.col, apart: ui.keepApart })
    else select(r)
  }
  function select(r: Placed) {
    ui.roomSel = r.id
    ui.rockSel = null
    roomsUi.add = null
  }

  function roomKey(e: KeyboardEvent, r: Placed) {
    if (consts().fixedRooms.includes(r.kind)) return
    const d = drag?.keys && drag.id === r.id ? drag : null
    const step: Record<string, [number, number]> = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] }
    if (e.key in step && (e.altKey || d)) {
      e.preventDefault()
      const [dr, dc] = step[e.key]
      const base = d ?? { id: r.id, grab: 0, row: r.row, col: r.col, moved: false, keys: true }
      drag = { ...base, row: base.row + dr, col: base.col + dc, moved: true, keys: true }
    } else if (d && e.key === "Enter") {
      e.preventDefault()
      drag = null
      if (d.row !== r.row || d.col !== r.col) app.act({ action: "moveRoom", room: r.id, row: d.row, col: d.col, apart: ui.keepApart })
    } else if (d && e.key === "Escape") {
      e.preventDefault()
      drag = null
    }
  }

  const ghost = $derived.by(() => {
    void app.rev
    if (drag?.moved) {
      const r = data.rooms.find((x) => x.id === drag!.id)
      if (!r) return null
      return { row: drag.row, col: drag.col, width: r.width, s: spot(r.kind, drag.row, drag.col, r.merge, r.id) }
    }
    const a = roomsUi.add
    if (a) {
      const col = query<number>("addCol", { kind: a.kind, row: a.row, at: a.at, merge: a.merge, rock: a.rock })
      if (a.rock) {
        const e = query<string | null>("fits", { row: a.row, col, width: 2 })
        return { row: a.row, col, width: 2, s: e === null ? "ok" : "bad" }
      }
      const width = roomShape(a.kind).width[roomShape(a.kind).merges.indexOf(a.merge)] ?? a.merge * 3
      return { row: a.row, col, width, s: spot(a.kind, a.row, col, a.merge, null) }
    }
    return null
  })
</script>

<section class="panel">
  <h3>{app.t("vault.map")}</h3>
  <p class="muted small">{app.t("vault.mapHelp")} {app.t("map.keysHelp")}</p>
  <div class="scroll">
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="map"
      bind:this={grid}
      style:--cols={data.cols}
      style:--rows={shown}
      onclick={gridClick}
      onpointermove={(e) => {
        if (e.target === grid) {
          const c = cellAt(e)
          hover = c && !taken.has(`${c[0]}:${c[1]}`) ? c : null
        } else hover = null
      }}
      onpointerleave={() => (hover = null)}
    >
      {#if hover && !drag}
        <div class="hover" style:grid-row="{hover[0] + 1}" style:grid-column="{hover[1] + 1}"></div>
      {/if}
      {#each data.rocks as [r, c] (`${r}:${c}`)}
        <button
          type="button"
          class="rock"
          class:sel={ui.rockSel?.[0] === r && ui.rockSel?.[1] === c}
          style:grid-row="{r + 1}"
          style:grid-column="{c + 1} / span 2"
          aria-label={app.t("map.rock")}
          title={app.t("map.rockHint")}
          onclick={() => {
            ui.rockSel = [r, c]
            ui.roomSel = null
            roomsUi.add = null
          }}
        ></button>
      {/each}
      {#each data.rooms as r (r.id)}
        {@const fire = data.burning.includes(r.id)}
        {@const name = core().kindName(r.kind, r.level)}
        {@const fixed = consts().fixedRooms.includes(r.kind)}
        {@const lvl = r.kind !== "Elevator" && (roomShape(r.kind).maxLevel[roomShape(r.kind).merges.indexOf(r.merge)] ?? 1) > 1}
        <button
          type="button"
          class="room {classOf(r.kind)}"
          class:elevator={r.kind === "Elevator"}
          class:sel={ui.roomSel === r.id}
          class:fire
          class:dragging={drag?.id === r.id && drag.moved}
          style:grid-row="{r.row + 1}"
          style:grid-column="{r.col + 1} / span {r.width}"
          title="{name}{'\n'}{app.t(fixed ? 'map.fixed' : 'map.dragHint')}"
          aria-label="{name}, {app.t('room.position', { row: r.row, col: r.col })}"
          onpointerdown={(e) => roomDown(e, r)}
          onpointermove={roomMove}
          onpointerup={() => roomUp(r)}
          onpointercancel={() => (drag = null)}
          onkeydown={(e) => roomKey(e, r)}
          onclick={(e) => {
            if (e.detail === 0) select(r)
          }}
        >
          {#if fire}<Icon name="alert" />{/if}
          {#if r.width > 1}
            <span class="name">{name}</span>
            {#if lvl}<span class="lvl">{app.t("map.lvl", { n: r.level })}</span>{/if}
          {/if}
        </button>
      {/each}
      {#if ghost}
        <div class="ghost {ghost.s}" style:grid-row="{ghost.row + 1}" style:grid-column="{ghost.col + 1} / span {ghost.width}"></div>
      {/if}
    </div>
  </div>
</section>

<style>
  .scroll {
    overflow-x: auto;
  }
  .map {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(11px, 1fr));
    grid-template-rows: repeat(var(--rows), 44px);
    min-width: calc(var(--cols) * 14px);
    background-color: var(--bg);
    background-image:
      linear-gradient(to right, color-mix(in srgb, var(--border) 35%, transparent) 1px, transparent 1px),
      linear-gradient(to bottom, color-mix(in srgb, var(--border) 35%, transparent) 1px, transparent 1px);
    background-size: calc(100% / var(--cols)) calc(100% / var(--rows));
    border-radius: var(--r-sm);
    user-select: none;
    touch-action: pan-y;
  }
  .hover {
    background: var(--accent-soft);
    border-radius: 2px;
    margin: 1px;
    pointer-events: none;
  }
  .rock {
    margin: 1px;
    border-radius: 2px;
    background: var(--border);
    border: 1px solid color-mix(in srgb, var(--muted) 50%, transparent);
    min-height: 0;
    padding: 0;
  }
  .rock.sel {
    border: 2px solid var(--focus);
  }
  .room {
    margin: 1px;
    min-height: 0;
    min-width: 0;
    padding: 2px 5px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--surface-alt);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 0;
    overflow: hidden;
    font-size: 11.5px;
    line-height: 1.2;
    cursor: grab;
    text-align: start;
    touch-action: none;
  }
  .room.Production {
    background: var(--accent-soft);
  }
  .room.Training {
    background: var(--success-soft);
  }
  .room.Crafting {
    background: var(--warning-soft);
  }
  .room.Consumable {
    background: var(--danger-soft);
  }
  .room.elevator {
    background: var(--surface-alt);
  }
  .room.sel {
    border: 2px solid var(--focus);
  }
  .room.fire {
    border: 2px solid var(--danger);
    color: var(--danger);
  }
  .room.dragging {
    opacity: 0.4;
  }
  .name {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    max-width: 100%;
  }
  .lvl {
    color: var(--muted);
    font-size: 10.5px;
  }
  .ghost {
    margin: 1px;
    border-radius: 3px;
    border: 2px solid;
    pointer-events: none;
    opacity: 0.85;
  }
  .ghost.ok {
    background: var(--success-soft);
    border-color: var(--success);
  }
  .ghost.warn {
    background: var(--warning-soft);
    border-color: var(--warning);
  }
  .ghost.bad {
    background: var(--danger-soft);
    border-color: var(--danger);
  }
</style>
