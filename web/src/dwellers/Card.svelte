<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Pick from "../components/Pick.svelte"
  import TextField from "../fields/TextField.svelte"
  import NumField from "../fields/NumField.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import ChoiceField from "../fields/ChoiceField.svelte"
  import Row from "../fields/Row.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import Looks from "./Looks.svelte"
  import Equipment from "./Equipment.svelte"
  import Relations from "./Relations.svelte"
  import { catalog, consts, core, describe, rarityName, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { arr, DWELLERS, dwellerName, isDead, ROOMS, SPECIAL_LETTERS } from "../lib/vault"

  interface Props {
    id: number
  }
  let { id }: Props = $props()

  const index = $derived(arr(app.root, DWELLERS).findIndex((d) => d.serializeId === id))
  const d = $derived(arr(app.root, DWELLERS)[index])
  const base = $derived<Path>([...DWELLERS, index])
  const at = (...rest: (string | number)[]): Path => [...base, ...rest]
  const lim = $derived(catalog().limits)
  const level = $derived<number>(d?.experience?.currentLevel ?? 1)
  const maxHp = $derived<number>(Math.floor(d?.health?.maxHealth ?? 100))

  const inTeam = $derived(
    arr(app.root, ["vault", "wasteland", "teams"]).some((t: any) => Array.isArray(t?.dwellers) && t.dwellers.includes(id)),
  )
  const rooms = $derived(arr(app.root, ROOMS))
  const current = $derived(rooms.find((r) => Array.isArray(r?.dwellers) && r.dwellers.includes(id)))
  const roomItems = $derived([
    { value: "-1", label: app.t("dw.noRoom") },
    ...rooms
      .filter((r) => !["Elevator", "Entrance", "FakeWasteland"].includes(r?.type))
      .sort((a, b) => (a.row ?? 0) - (b.row ?? 0) || (a.col ?? 0) - (b.col ?? 0))
      .map((r) => ({
        value: String(r.deserializeID),
        label: `${core().kindName(r.type, r.level ?? 1)}  ·  ${app.t("room.position", { row: r.row ?? 0, col: r.col ?? 0 })}`,
      })),
  ])
</script>

{#if d}
  <section class="panel">
    <div class="row-wrap head">
      <h2>{dwellerName(d)}</h2>
      {#if app.expert}<span class="mono muted">#{id}</span>{/if}
      {#if isDead(d)}<span class="badge dead">{app.t("dw.dead")}</span>{/if}
    </div>
    <div class="row-wrap">
      {#if isDead(d)}
        <button type="button" title={inTeam ? app.t("dw.reviveTeamHelp") : undefined} onclick={() => app.act({ action: "revive", ids: [id] })}><Icon name="sparkles" /> {app.t("dw.revive")}</button>
        {#if inTeam}
          <button type="button" title={app.t("dw.reviveHomeHelp")} onclick={() => app.act({ action: "revive", ids: [id], home: true })}><Icon name="sparkles" /> {app.t("dw.reviveHome")}</button>
        {/if}
      {/if}
      <button type="button" title={app.t("dw.cloneHelp")} onclick={() => app.act({ action: "cloneDweller", id })}><Icon name="copy" /> {app.t("dw.clone")}</button>
      <button type="button" class="danger" onclick={() => app.push({ kind: "custom", id: "confirmDelete", data: [id] })}><Icon name="trash" /> {app.t("dw.delete")}</button>
    </div>
  </section>

  <section class="panel">
    <h3>{app.t("dw.identity")}</h3>
    <TextField path={at("name")} label={app.t("dw.firstName")} />
    <TextField path={at("lastName")} label={app.t("dw.lastName")} />
    <ChoiceField
      path={at("gender")}
      label={app.t("dw.gender")}
      int
      choices={[
        { value: "1", label: app.t("dw.gender.1") },
        { value: "2", label: app.t("dw.gender.2") },
      ]}
    />
    <ChoiceField
      path={at("rarity")}
      label={app.t("dw.rarity")}
      choices={consts()
        .rarities.filter((r) => r !== "Common" || d.rarity === "Common")
        .map((r) => ({ value: r, label: rarityName(r, app.ru) }))}
    />
    {#if app.expert && d.uniqueData !== undefined}
      <TextField path={at("uniqueData")} label={app.t("dw.unique")} />
    {/if}
  </section>

  <section class="panel">
    <h3>{app.t("dw.progress")}</h3>
    <Row path={at("experience", "currentLevel")} label={app.t("dw.level")} help={app.t("dw.levelHelp")} min={1} max={lim.dwellerLevelMax}>
      <NumPick
        value={level}
        label={app.t("dw.level")}
        lo={Math.min(1, level)}
        hi={Math.max(app.expert ? 999 : lim.dwellerLevelMax, level)}
        slider={[1, Math.max(lim.dwellerLevelMax, level)]}
        oncommit={(l) => app.act({ action: "setLevel", ids: [id], level: l })}
      />
    </Row>
    {#if d.experience?.experienceValue !== undefined}
      <NumField path={at("experience", "experienceValue")} label={app.t("dw.xp")} unit={app.t("c.units.xp")} />
    {/if}
    {#if d.experience?.wastelandExperience !== undefined}
      <NumField path={at("experience", "wastelandExperience")} unit={app.t("c.units.xp")} />
    {/if}
    {#each [["hoursOnWasteland", "dw.wlHours"], ["daysOnWasteland", "dw.wlDays"]] as [key, label] (key)}
      {#if d[key] !== undefined}
        <NumField path={at(key)} label={app.t(label)} help={app.t("dw.wlTimeHelp")} min={0} max={100000} />
      {/if}
    {/each}
    <NumField path={at("happiness", "happinessValue")} label={app.t("dw.happiness")} min={lim.happinessMin} max={lim.happinessMax} unit={app.t("c.units.percent")} />
  </section>

  <section class="panel">
    <h3>{app.t("dw.state")}</h3>
    <NumField path={at("health", "healthValue")} label={app.t("dw.health")} min={0} max={maxHp} unit={app.t("c.units.hp")} />
    <NumField path={at("health", "maxHealth")} label={app.t("dw.maxHealth")} unit={app.t("c.units.hp")} />
    <NumField path={at("health", "radiationValue")} label={app.t("dw.radiation")} min={0} max={maxHp} unit={app.t("c.units.rad")} />
    {#if typeof d.health?.permaDeath === "boolean"}<BoolField path={at("health", "permaDeath")} />{/if}
    {#if app.expert && d.health?.lastLevelUpdated !== undefined}
      {#if typeof d.health.lastLevelUpdated === "boolean"}<BoolField path={at("health", "lastLevelUpdated")} />{:else}<NumField
          path={at("health", "lastLevelUpdated")}
        />{/if}
    {/if}
    {#each ["deathTime", "deathSource"] as key (key)}
      {#if app.expert && typeof d[key] === "number"}<NumField path={at(key)} />{/if}
    {/each}
  </section>

  <section class="panel">
    <h3>{app.t("dw.special")}</h3>
    {#each [1, 2, 3, 4, 5, 6, 7] as s (s)}
      <NumField
        path={at("stats", "stats", s, "value")}
        label="{SPECIAL_LETTERS[s - 1]} – {app.t(`dw.specialName.${s}`)}"
        help={app.t(`dw.specialHelp.${s}`)}
        min={lim.specialMin}
        max={lim.specialMax}
      />
    {/each}
  </section>

  <Looks {d} {id} {index} {base} />
  <Equipment {d} {id} {base} />
  <Relations {d} {id} {base} />

  <section class="panel">
    <h3>{app.t(app.expert ? "dw.flags" : "dw.placement")}</h3>
    {#if app.expert}
      {#each ["pregnant", "babyReady", "assigned", "sawIncident", "WillGoToWasteland", "WillBeEvicted", "IsEvictedWaitingForFollowers", "wasTemporarilyAssigned"] as key (key)}
        {#if typeof d[key] === "boolean"}<BoolField path={at(key)} />{/if}
      {/each}
    {/if}
    <Row path={at("savedRoom")} label={app.t("dw.room")} help={describe(app.lang, at("savedRoom")).help} choice>
      <Pick
        label={app.t("dw.room")}
        value={String(current?.deserializeID ?? -1)}
        items={roomItems}
        onchange={(v) => app.act({ action: "assignRoom", ids: [id], room: Number(v) })}
      />
    </Row>
  </section>
{/if}

<style>
  .head h2 {
    font-size: 20px;
  }
  .badge.dead {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
