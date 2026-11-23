<script lang="ts">
  import ArtNotice from "../art/ArtNotice.svelte"
  import { art } from "../lib/art.svelte"
  import ThemeInfo from "../rooms/ThemeInfo.svelte"
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import Fold from "../components/Fold.svelte"
  import Pick from "../components/Pick.svelte"
  import NumField from "../fields/NumField.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import DurationRow from "../fields/DurationRow.svelte"
  import ItemCard from "../items/ItemCard.svelte"
  import ItemPicker from "../items/ItemPicker.svelte"
  import ThemeCard from "../rooms/ThemeCard.svelte"
  import { catalog, consts, core, describe, item, nameOf, pathStr, query, type ItemKind } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ACTORS, arr, DWELLERS, isDead } from "../lib/vault"
  import { nameAny } from "./names"

  const teams = $derived(arr(app.root, ["vault", "wasteland", "teams"]))
  let asRecipe = $state(false)

  function teamDead(t: any): boolean {
    if (t?.status === "Dead" || t?.status === "DeadOnQuest") return true
    const ids: unknown[] = Array.isArray(t?.dwellers) ? t.dwellers : []
    return arr(app.root, DWELLERS).some((d: any) => ids.includes(d?.serializeId) && isDead(d))
  }

  function statusText(s: string): string {
    const k = `o.team.${s}`
    const t = app.t(k)
    return t === k ? s : t
  }

  function groups(t: any) {
    const out: { id: string; kind: string; recipe: boolean; idx: number[] }[] = []
    ;(t.teamEquipment?.inventory?.items ?? []).forEach((it: any, i: number) => {
      const id = it?.id ?? ""
      const kind = it?.type ?? ""
      const recipe = it?.extraData?.isRecipe === true
      const g = out.find((x) => x.id === id && x.kind === kind && x.recipe === recipe)
      if (g) g.idx.push(i)
      else out.push({ id, kind, recipe, idx: [i] })
    })
    return out
  }

  function robots(t: any): string[] {
    return (t.actors ?? [])
      .filter((x: unknown) => typeof x === "number")
      .map((id: number) => {
        const a = arr(app.root, ACTORS).find((x) => x?.serializeId === id)
        return a?.name || app.t("dw.mrHandy")
      })
  }

  function startItems(t: any): string[] {
    const ids: number[] = (t.dwellers ?? []).filter((x: unknown) => typeof x === "number")
    return (t.initialItems ?? []).map((it: any, k: number) => {
      const nm = (kind: ItemKind, key: string) => {
        const id = it?.[key]
        if (typeof id !== "string" || !id) return null
        const info = item(kind, id)
        return info ? nameOf(info.name, app.itemRu, id) : id
      }
      const parts = [nm("Weapon", "initWeapon"), nm("Outfit", "initOutfit"), nm("Pet", "initPet")].filter(Boolean)
      return `${ids[k] !== undefined ? nameAny(ids[k]) : ""}: ${parts.join(", ")}`
    })
  }
</script>

<section class="panel">
  <h3>{app.t("o.explorers")}</h3>
  <p class="muted">{app.t("o.explorersHelp")}</p>
  {#if teams.length === 0}<p class="muted">{app.t("o.noExplorers")}</p>{/if}
</section>

{#each teams as t, i (i)}
  {@const base = ["vault", "wasteland", "teams", i]}
  {@const status = typeof t.status === "string" ? t.status : ""}
  {@const gs = groups(t)}
  {@const load = (void app.rev, query<number | null>("teamLoad", { path: pathStr(base) }) ?? 0)}
  <section class="panel">
    <h3>{(t.dwellers ?? []).filter((x: unknown) => typeof x === "number").map((d: number) => nameAny(d)).join(", ")}</h3>
    <p><span class="muted">{app.t("o.status")}</span> {statusText(status)}</p>
    {#if teamDead(t)}
      {@const ids = (t.dwellers ?? []).filter((x: unknown) => typeof x === "number")}
      <div class="row-wrap">
        <button type="button" title={app.t("dw.reviveTeamHelp")} onclick={() => app.act({ action: "revive", ids })}><Icon name="sparkles" /> {app.t("o.reviveTeam")}</button>
        <button type="button" title={app.t("dw.reviveHomeHelp")} onclick={() => app.act({ action: "revive", ids, home: true })}><Icon name="sparkles" /> {app.t("dw.reviveHome")}</button>
      </div>
    {/if}
    {#if typeof t.elapsedTimeAliveExploring === "number"}
      {#if status === "Exploring" || status === "GoingToQuest"}
        <DurationRow
          path={[...base, "elapsedTimeAliveExploring"]}
          label={app.t("o.exploringTime")}
          help={app.t("o.exploringTimeHelp")}
          secs={t.elapsedTimeAliveExploring}
          unit={3600}
          onset={(v) => app.act({ action: "setInt", path: [...base, "elapsedTimeAliveExploring"], value: Math.round(v) })}
        />
      {:else}
        <p><span class="muted">{app.t("o.exploring")}</span> {core().duration(t.elapsedTimeAliveExploring)}</p>
      {/if}
    {/if}
    {#if status === "ReturningToVault" && typeof t.returnTripDuration === "number" && typeof t.elapsedReturningTime === "number"}
      {@const total = t.returnTripDuration}
      <DurationRow
        path={[...base, "elapsedReturningTime"]}
        label={app.t("o.returnIn")}
        help={app.t("o.returnInHelp")}
        secs={Math.max(0, total - t.elapsedReturningTime)}
        unit={60}
        onset={(v) => app.act({ action: "setInt", path: [...base, "elapsedReturningTime"], value: Math.max(0, Math.round(total - v)) })}
      />
      <button type="button" title={app.t("o.returnNowHelp")} onclick={() => app.act({ action: "setInt", path: [...base, "elapsedReturningTime"], value: Math.max(1, Math.ceil(total)) })}>
        <Icon name="sparkles" /> {app.t("o.returnNow")}
      </button>
    {/if}
    {#each [["Nuka", "o.capsFound", "o.capsFoundHelp", consts().resourceMax], ["StimPack", "o.teamStim", "o.teamStimHelp", 25], ["RadAway", "o.teamRad", "o.teamRadHelp", 25]] as [res, label, help, max] (res)}
      {#if t.teamEquipment?.storage?.resources?.[res] !== undefined}
        <NumField path={[...base, "teamEquipment", "storage", "resources", res]} label={app.t(String(label))} help={app.t(String(help))} min={0} max={Number(max)} />
      {/if}
    {/each}

    <h4 title={app.t("o.lootHelp")}>{app.t("o.loot", { n: load, max: catalog().limits.teamItemsMax })}</h4>
    {#if gs.length === 0}<p class="muted">{app.t("o.lootEmpty")}</p>{/if}
    <ul class="list">
      {#each gs as g (g.kind + g.id + g.recipe)}
        <li>
          {#if ["Weapon", "Outfit", "Pet", "Junk"].includes(g.kind)}<ItemCard kind={g.kind as ItemKind} id={g.id} />{:else}{g.id}{/if}
          <span class="muted small">{app.t(`it.kind.${g.kind}`)}{#if g.recipe}&nbsp;·&nbsp;{app.t("o.lootRecipe")}{/if}</span>
          <span class="mono">× {g.idx.length}</span>
          <button type="button" class="icon ghost" aria-label={app.t("o.lootRemoveOne")} title={app.t("o.lootRemoveOne")} onclick={() => app.act({ action: "removeTeamItems", team: i, indices: [g.idx[g.idx.length - 1]] })}><Icon name="x" /></button>
          {#if g.idx.length > 1}
            <button type="button" class="icon ghost" aria-label={app.t("o.lootRemoveAll")} title={app.t("o.lootRemoveAll")} onclick={() => app.act({ action: "removeTeamItems", team: i, indices: g.idx })}><Icon name="trash" /></button>
          {/if}
        </li>
      {/each}
    </ul>
    <div class="row-wrap">
      <ItemPicker
        kinds={["Weapon", "Outfit", "Junk"]}
        label={app.t("o.loot", { n: load, max: catalog().limits.teamItemsMax })}
        onpick={(kind, id) => app.act({ action: "addTeamItems", team: i, kind, id, count: 1, recipe: asRecipe && kind !== "Junk" })}
      />
      <span title={app.t("o.lootAsRecipeHelp")}><Check checked={asRecipe} label={app.t("o.lootAsRecipe")} onchange={(v) => (asRecipe = v)} /></span>
    </div>

    {#if Array.isArray(t.teamEquipment?.collectedThemes?.themeList)}
      {@const themes = t.teamEquipment.collectedThemes.themeList.filter((th: any) => typeof th?.id === "string")}
      <h4 title={app.t("o.lootThemesHelp")}>{app.t("o.lootThemes")}</h4>
      {#each themes as th (th.id)}
        <p class="row-wrap">
          <ThemeCard id={th.id} />
          <NumPick
            value={th.extraData?.partsCollectedCount ?? 0}
            label={app.t("theme.partsOf")}
            lo={0}
            hi={consts().themeParts}
            oncommit={(v) => app.act({ action: "setTeamThemeParts", team: i, id: th.id, parts: v })}
          />
          <span class="muted">{app.t("theme.partsOf")}</span>
        </p>
      {/each}
      <Pick
        label={app.t("o.lootThemeAdd")}
        value={null}
        placeholder={app.t("o.lootThemeAdd")}
        items={(catalog().themes as any[])
          .filter((x) => x.recipe && !themes.some((th: any) => th.id === x.id))
          .map((x) => ({ value: x.id, label: nameOf(x.name, app.ru, x.id) }))}
        onchange={(id) => app.act({ action: "setTeamThemeParts", team: i, id, parts: 1 })}
      >
        {#snippet preview(id: string)}<ThemeInfo {id} />{/snippet}
      </Pick>
    {/if}

    {#if robots(t).length}<p><span class="muted">{app.t("o.teamRobots")}</span> {robots(t).join(", ")}</p>{/if}
    {#if (t.initialItems ?? []).length}
      <Fold title={app.t("o.teamStart")}>
        {#each startItems(t) as line, k (k)}<p>{line}</p>{/each}
      </Fold>
    {/if}
    {#if t.questOverallPerformance && typeof t.questOverallPerformance === "object"}
      {@const perf = Object.entries(t.questOverallPerformance).filter(([, v]) => typeof v === "number" && v > 0)}
      {#if perf.length}
        <Fold title={app.t("o.teamResults")}>
          {#each perf as [k, v] (k)}
            <p><span class="muted">{describe(app.lang, [...base, "questOverallPerformance", k]).label}</span> {v}</p>
          {/each}
        </Fold>
      {/if}
    {/if}
    {#if (t.logs ?? []).length}
      <Fold title={app.t("o.teamLog", { n: t.logs.length })}>
        {#if !art.texts}<ArtNotice text={app.t("art.web.noTexts")} />{/if}
        <ul class="log">
          {#each [...t.logs].reverse().filter((x: unknown) => typeof x === "string") as code, k (k)}
            {@const [time, text] = (void art.rev, core().logLine(code))}
            <li><span class="mono muted">{time}</span> {text}</li>
          {/each}
        </ul>
      </Fold>
    {/if}
  </section>
{/each}

<style>
  h4 {
    margin: var(--s-md) 0 var(--s-xs);
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s-sm);
  }
  .list li {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
  }
  .log {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 320px;
    overflow: auto;
  }
  .log li {
    padding: 2px 0;
  }
  p {
    margin: 4px 0;
  }
</style>
