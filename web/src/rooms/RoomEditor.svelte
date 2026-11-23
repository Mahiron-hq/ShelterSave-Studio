<script lang="ts">
  import ArtNotice from "../art/ArtNotice.svelte"
  import Icon from "../components/Icon.svelte"
  import Pick from "../components/Pick.svelte"
  import Seg from "../components/Seg.svelte"
  import Row from "../fields/Row.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import NumField from "../fields/NumField.svelte"
  import ChoiceField from "../fields/ChoiceField.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import RoomPicture from "./RoomPicture.svelte"
  import ThemePicker from "./ThemePicker.svelte"
  import RoomTimers from "./RoomTimers.svelte"
  import FutureChildren from "./FutureChildren.svelte"
  import RoomIncident from "./RoomIncident.svelte"
  import { catalog, consts, core, describe, nameOf, query, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { fmtNum } from "../lib/format"
  import { arr, DWELLERS, dwellerName, isDead, ROOMS } from "../lib/vault"
  import { rooms as roomsUi } from "./rooms.svelte"

  interface Props {
    id: number
  }
  let { id }: Props = $props()

  interface Info {
    index: number
    sizes: { merge: number; fits: boolean; free: boolean }[]
    splits: { parts: number[]; error: "size" | "busy" | "other" | null }[]
    maxLevel: number
    neighbours: boolean
    canRemove: boolean
    busy: boolean
    batch: Record<string, number> | null
    product: string[]
    picture: [string, string]
    pictureAlt: string | null
  }
  const info = $derived((void app.rev, query<Info | null>("room", { id })))
  const r = $derived(info ? arr(app.root, ROOMS)[info.index] : null)
  const base = $derived<Path>(info ? ["vault", "rooms", info.index] : [])
  const at = (...rest: (string | number)[]): Path => [...base, ...rest]
  const kind = $derived<string>(r?.type ?? "")
  const fixed = $derived(consts().fixedRooms.includes(kind))
  const themed = $derived(kind === "FakeWasteland" || (catalog().themes as any[]).some((t) => t.room === kind))
  const levelInfo = $derived(describe(app.lang, at("level")))
  const c = consts()

  const stored = $derived.by(() => {
    const st = r?.storage?.resources
    if (!st || typeof st !== "object" || !info) return []
    return Object.entries(st)
      .filter(([k, v]) => app.expert || info.product.includes(k) || (c.resourceKeys.includes(k) && typeof v === "number" && v > 0))
      .map(([k, v]) => ({ key: k, value: typeof v === "number" ? v : 0, full: info.batch?.[k] ?? 0 }))
  })
  const ids = $derived<number[]>(Array.isArray(r?.dwellers) ? r.dwellers.filter((x: unknown) => typeof x === "number") : [])
  const cap = $derived(kind === "Elevator" ? 0 : 2 * (r?.mergeLevel ?? 1))
  const dwellers = $derived(arr(app.root, DWELLERS))
  const nameOfDweller = (d: number) => {
    const x = dwellers.find((y) => y.serializeId === d)
    return x ? dwellerName(x) : `#${d}`
  }
  const blocked = $derived(app.expert ? null : info?.busy ? app.t("aerr.roomBusy") : info && !info.canRemove ? app.t("aerr.roomDisconnected") : null)
  const noCopy = $derived(
    app.expert ? null : c.uniqueRooms.includes(kind) ? app.t("room.cloneUnique") : c.notBuildable.includes(kind) ? app.t("room.cloneNotBuildable") : null,
  )

  function remove() {
    if (blocked) return
    if (app.act({ action: "removeRoom", room: id })) ui.roomSel = null
  }

  function startCopy() {
    if (!r || noCopy) return
    const spot = query<{ row: number; col: number; ok: boolean } | null>("copySpot", { id })
    if (!spot) return
    if (!spot.ok) app.toast(app.t("map.copyNoSpot"))
    const { row, col } = spot
    roomsUi.add = { row, at: col, kind, merge: r.mergeLevel ?? 1, level: r.level ?? 1, rock: false, copy: true }
    ui.roomSel = null
    ui.rockSel = null
  }

  function keys(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return
    if (app.modals.length || fixed) return
    if (e.key === "Delete" && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault()
      remove()
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "d") {
      e.preventDefault()
      startCopy()
    }
  }
</script>

<svelte:window onkeydown={keys} />

{#if r && info}
  <section class="panel">
    <h3>{core().kindName(kind, r.level ?? 1)}{#if app.expert}&nbsp;&nbsp;#{id}{/if}</h3>
    <p class="muted small">{app.t("room.position", { row: r.row ?? 0, col: r.col ?? 0 })}</p>
    <ArtNotice />
    <RoomPicture kind={info.picture[0]} id={info.picture[1]} alt={info.pictureAlt} label={core().kindName(kind, r.level ?? 1)} />
    {#if fixed}<p class="muted small">{app.t("map.fixed")}</p>{/if}

    {#if themed}
      <Row label={app.t("room.theme")} help={app.t("room.themeHelp")}>
        <ThemePicker room={kind} />
      </Row>
      {#if kind !== "FakeWasteland" && !(r.mergeLevel === 3 && r.level === 3)}
        <p class="muted small"><Icon name="info" /> {app.t("room.themeHidden")}</p>
      {/if}
    {/if}

    {#if r.level !== undefined && info.maxLevel > 1}
      <Row path={at("level")} label={levelInfo.label} help={levelInfo.help} choice>
        <Seg
          label={levelInfo.label}
          value={r.level}
          options={Array.from({ length: Math.max(info.maxLevel, r.level) }, (_, i) => ({ value: i + 1, label: String(i + 1) }))}
          onchange={(l) => app.act({ action: "setRoomLevel", room: id, level: l, apart: ui.keepApart })}
        />
      </Row>
    {/if}

    {#if kind !== "Elevator" && !fixed}
      <Row path={at("mergeLevel")} label={app.t("room.merge")} help={app.t("room.mergeHelp")} choice>
        <Seg
          label={app.t("room.merge")}
          value={r.mergeLevel ?? 1}
          options={info.sizes.map((s) => ({
            value: s.merge,
            label: String(s.merge),
            disabled: !s.fits && s.merge !== (r.mergeLevel ?? 1),
            title: !s.fits ? app.t(s.free ? "room.mergeCuts" : "room.mergeNoSpace") : undefined,
          }))}
          onchange={(m) => app.act({ action: "setMergeLevel", room: id, merge: m, apart: ui.keepApart })}
        />
      </Row>
      {#if info.splits.length}
        <Row label={app.t("room.split")} help={app.t("room.splitHelp")}>
          <div class="row-wrap">
            {#each info.splits as s (s.parts.join("+"))}
              <button
                type="button"
                disabled={s.error !== null}
                title={s.error === "size" ? app.t("room.splitSize") : s.error === "busy" ? app.t("room.splitBusy") : undefined}
                onclick={() => app.act({ action: "splitRoom", room: id, parts: s.parts })}>{s.parts.join(" + ")}</button
              >
            {/each}
          </div>
        </Row>
      {/if}
    {/if}

    {#each [["power", "room.power"], ["broken", "room.broken"], ["withHole", "room.withHole"], ["emergencyDone", "room.emergencyDone"]] as [key, label] (key)}
      {#if (app.expert || key !== "emergencyDone") && r[key] !== undefined}
        <BoolField path={at(key)} label={app.t(label)} help={key !== "emergencyDone" ? app.t(`${label}Help`) : undefined} />
      {/if}
    {/each}
    {#if r.onlyIncreaseHappiness !== undefined}
      <BoolField path={at("onlyIncreaseHappiness")} label={app.t("room.radioVault")} help={app.t("room.radioVaultHelp")} />
    {/if}
    {#if r.newDwellerReady !== undefined}
      <BoolField path={at("newDwellerReady")} label={app.t("room.radioReady")} help={app.t("room.radioReadyHelp")} />
    {/if}

    <RoomTimers room={r} {base} />
    <FutureChildren room={r} {base} />
    <RoomIncident room={r} path={base} />

    {#if typeof r.roomHealth?.damageValue === "number" && (app.expert || kind === "Entrance")}
      <NumField
        path={at("roomHealth", "damageValue")}
        label={app.t(kind === "Entrance" ? "room.doorDamage" : "room.damage")}
        help={app.t(kind === "Entrance" ? "room.doorDamageHelp" : "room.damageHelp")}
      />
    {/if}
    {#if app.expert && r.currentStateName !== undefined}
      <ChoiceField
        path={at("currentStateName")}
        label={app.t("room.state")}
        choices={[
          { value: "Idle", label: app.t("room.state.Idle") },
          { value: "Working", label: app.t("room.state.Working") },
        ]}
      />
    {/if}
    {#if app.expert && r.assignedDecoration !== undefined}
      <ChoiceField
        path={at("assignedDecoration")}
        label={app.t("room.decoration")}
        choices={[{ value: "", label: app.t("c.none") }, ...(catalog().decorations as any[]).map((d) => ({ value: d.id, label: nameOf(d.name, app.ru, d.id) }))]}
      />
    {/if}

    {#if stored.length}
      <h4 title={app.t("room.storedHelp")}>{app.t("room.stored")}</h4>
      {#each stored as s (s.key)}
        {#if s.full >= 1}
          {@const shown = Math.min(Math.floor(s.value), s.full)}
          <Row path={at("storage", "resources", s.key)} label={app.t(`res.${s.key}`)} help={app.t("room.storedRowHelp")} computed>
            <NumPick value={shown} label={app.t(`res.${s.key}`)} lo={0} hi={s.full} slider={[0, s.full]} oncommit={(v) => app.act({ action: "setStored", room: id, resource: s.key, value: v })} />
            <span class="muted">{app.t("room.storedOf", { n: fmtNum(s.full) })}</span>
          </Row>
          <p class="small" class:ok={s.value >= s.full - 0.005} class:muted={s.value < s.full - 0.005}>
            {s.value >= s.full - 0.005
              ? app.t("room.storedReady")
              : app.t("room.storedFilling", { pct: Math.min(99, Math.max(0, Math.floor((s.value / s.full) * 100))) })}
          </p>
        {:else}
          <NumField path={at("storage", "resources", s.key)} label={app.t(`res.${s.key}`)} />
        {/if}
      {/each}
    {/if}

    <h4>{app.t("room.dwellers")}</h4>
    <p class="muted small">{app.t("room.capacity", { n: ids.length, max: cap })}</p>
    <ul class="people">
      {#each ids as d (d)}
        <li>
          <Icon name="users" />
          <span>{nameOfDweller(d)}</span>
          <button type="button" class="icon ghost" aria-label="{app.t('c.remove')}: {nameOfDweller(d)}" onclick={() => app.act({ action: "assignRoom", ids: [d], room: -1 })}>
            <Icon name="x" />
          </button>
        </li>
      {/each}
    </ul>
    {#if kind !== "Elevator"}
      <Pick
        label={app.t("room.assign")}
        value={null}
        placeholder={app.t("room.assign")}
        items={dwellers.filter((d) => typeof d.serializeId === "number" && !ids.includes(d.serializeId) && !isDead(d)).map((d) => ({ value: String(d.serializeId), label: dwellerName(d) }))}
        onchange={(v) => app.act({ action: "assignRoom", ids: [Number(v)], room: id })}
      />
    {/if}
    {#if info.neighbours}
      <p class="warn small"><Icon name="info" /> {app.t("room.mergeNeighbours")}</p>
    {/if}
    {#if !fixed}
      <div class="row-wrap actions">
        <button type="button" disabled={!!noCopy} title={noCopy ?? app.t("room.cloneHelp")} aria-keyshortcuts="Control+D" onclick={startCopy}>
          <Icon name="copy" /> {app.t("room.clone")}
        </button>
        <button type="button" class="danger" disabled={!!blocked} title={blocked ?? app.t("room.removeHelp")} aria-keyshortcuts="Delete" onclick={remove}>
          <Icon name="trash" /> {app.t("room.remove")}
        </button>
      </div>
    {/if}
  </section>
{/if}

<style>
  h4 {
    margin: var(--s-md) 0 var(--s-xs);
  }
  .people {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s-sm);
  }
  .people li {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .warn {
    color: var(--warning);
  }
  .ok {
    color: var(--success);
  }
  .actions {
    margin-top: var(--s-md);
  }
  p {
    margin: 2px 0;
  }
</style>
