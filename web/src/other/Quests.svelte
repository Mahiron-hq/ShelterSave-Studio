<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import Seg from "../components/Seg.svelte"
  import NumField from "../fields/NumField.svelte"
  import QuestPicker from "./QuestPicker.svelte"
  import { catalog, consts, core, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { arr, dwellerName } from "../lib/vault"
  import { nameAny } from "./names"

  const r = $derived(app.root)
  const qdm = $derived(r?.questDataManager ?? {})
  const current = $derived<string>(qdm.questTeam?.CurrentQuestID ?? "")
  const running = $derived(current && !qdm.questDone && !qdm.cancelled)
  const status = $derived(
    qdm.cancelled
      ? app.t("o.quest.cancelled")
      : qdm.questDone && qdm.questSucceeded
        ? app.t("o.quest.succeeded")
        : qdm.questDone
          ? app.t("o.quest.failed")
          : app.t("o.quest.running"),
  )
  const team = $derived<number[]>((qdm.questTeam?.DwellersDictionary ?? []).filter((x: unknown) => typeof x === "number"))
  const questDwellers = $derived(arr(r, ["questDwellers", "dwellers"]))

  const offered = $derived.by(() => {
    const rows: { kind: string; index: number; label: string; name: string; end?: string }[] = []
    const c = r?.completedQuestDataManager ?? {}
    ;(c.dailyQuestPicker?.currentDailies ?? []).forEach((q: any, i: number) => q?.questName && rows.push({ kind: "daily", index: i, label: "o.daily", name: q.questName, end: q.endDate }))
    ;(c.weeklyQuestPicker?.currentWeeklies ?? []).forEach((q: any, i: number) => q?.questName && rows.push({ kind: "weekly", index: i, label: "o.weekly", name: q.questName, end: q.endDate }))
    const sa = c.standaloneQuestPicker?.currentStandalone
    if (typeof sa === "string" && sa) rows.push({ kind: "standalone", index: 0, label: "o.standalone", name: sa })
    return rows
  })

  const done = $derived<string[]>((r?.completedQuestDataManager?.completedQuests ?? []).filter((x: unknown) => typeof x === "string"))
  let filter = $state<"done" | "todo" | "all">("done")
  let q = $state("")
  const questRows = $derived.by(() => {
    const ids = [...(catalog().quests as any[]).map((x) => x.id as string)]
    for (const d of done) if (!ids.includes(d)) ids.push(d)
    const s = q.trim().toLowerCase()
    const set = new Set(done)
    return ids
      .map((id) => ({ id, name: core().questLine(id), done: set.has(id) }))
      .filter((x) => (filter === "done" ? x.done : filter === "todo" ? !x.done : true))
      .filter((x) => !s || x.name.toLowerCase().includes(s) || x.id.toLowerCase().includes(s))
  })

  const at = (p: string[]): Path => p
</script>

<section class="panel">
  <h3>{app.t("o.currentQuest")}</h3>
  {#if !current}
    <p class="muted">{app.t("o.noQuest")}</p>
  {:else}
    <dl class="info">
      <dt>{app.t("o.quest")}</dt>
      <dd>{core().questLine(current)}</dd>
      <dt>{app.t("o.status")}</dt>
      <dd>{status}</dd>
      {#if team.length}<dt>{app.t("o.team")}</dt>
        <dd>{team.map((d) => nameAny(d)).join(", ")}</dd>{/if}
    </dl>
    {#if running}
      {#each [["currentStimpakCount", "o.questStim", "o.questStimHelp"], ["currentRadawayCount", "o.questRad", "o.questRadHelp"]] as [key, label, help] (key)}
        {#if qdm.questTeam?.[key] !== undefined}
          <NumField path={at(["questDataManager", "questTeam", key])} label={app.t(label)} help={app.t(help)} min={0} max={25} />
        {/if}
      {/each}
      {#if questDwellers.length}<h4 title={app.t("o.questTeamHealthHelp")}>{app.t("o.questTeamHealth")}</h4>{/if}
      {#each questDwellers as d, i (i)}
        <p class="who">{dwellerName(d)}</p>
        {#each [["healthValue", "o.questHp"], ["radiationValue", "o.questRads"]] as [key, label] (key)}
          {#if d.health?.[key] !== undefined}
            <NumField path={["questDwellers", "dwellers", i, "health", key]} label={app.t(label)} min={0} max={Math.max(1, d.health?.maxHealth ?? 100)} />
          {/if}
        {/each}
      {/each}
    {/if}
  {/if}
</section>

<section class="panel">
  <h3>{app.t("o.dailyWeekly")}</h3>
  <p class="muted">{app.t("o.dailyWeeklyHelp")}</p>
  {#if offered.length === 0}<p class="muted">{app.t("o.noDaily")}</p>{/if}
  {#each offered as o (o.kind + o.index)}
    <div class="offer">
      <p><span class="muted">{app.t(o.label)}</span> {core().questLine(o.name)}{#if o.end}&nbsp;·&nbsp;{app.t("o.until", { date: core().gameDate(o.end) })}{/if}</p>
      {#if running && current === o.name}
        <p class="muted small">{app.t("o.questRunningNoReplace")}</p>
      {:else}
        <QuestPicker
          prefix={o.kind === "daily" ? "Daily_" : o.kind === "weekly" ? "Weekly_" : "Standalone_"}
          current={o.name}
          onpick={(id) => app.act({ action: "replaceQuest", kind: o.kind, index: o.index, id })}
        />
      {/if}
    </div>
  {/each}
</section>

{#if r?.completedQuestDataManager?.completedQuests !== undefined}
  <section class="panel">
    <h3>{app.t("o.completedQuests", { n: done.length, all: catalog().quests.length })}</h3>
    <p class="muted">{app.t("o.completedQuestsHelp")}</p>
    <div class="row-wrap">
      <input type="search" aria-label={app.t("c.search")} placeholder={app.t("c.search")} bind:value={q} />
      <Seg
        label={app.t("o.completedQuests", { n: done.length, all: catalog().quests.length })}
        value={filter}
        options={[
          { value: "done" as const, label: app.t("o.filter.done") },
          { value: "todo" as const, label: app.t("o.filter.todo") },
          { value: "all" as const, label: app.t("o.filter.all") },
        ]}
        onchange={(v) => (filter = v)}
      />
    </div>
    <ul class="list">
      {#if questRows.length === 0}<li class="muted">{app.t("c.nothingFound")}</li>{/if}
      {#each questRows as x (x.id)}
        <li title={app.expert ? x.id : app.t("o.questToggleHelp")}>
          <Check checked={x.done} label={x.name} onchange={(v) => app.act({ action: "markQuest", id: x.id, done: v })} />
        </li>
      {/each}
    </ul>
  </section>
{/if}

<section class="panel">
  <h3>{app.t("o.questMemory")}</h3>
  <p class="muted">{app.t("o.questMemoryHelp")}</p>
  {#each consts().questMemory as [key, path] (key)}
    {@const list = arr(r, path)}
    {#if list.length}
      <div class="row-wrap mem">
        <strong>{app.t(`o.qm.${key}`)}</strong>
        <button type="button" class="small" onclick={() => app.act({ action: "forgetQuest", list: key, quest: "" })}>{app.t("o.qmClear")}</button>
      </div>
      {#each list as e, i (i)}
        {#if typeof e?.questName === "string"}
          <p class="row-wrap">
            <button type="button" class="icon ghost" aria-label="{app.t('o.qmForget')}: {core().questLine(e.questName)}" title={app.t("o.qmForget")} onclick={() => app.act({ action: "forgetQuest", list: key, quest: e.questName })}>
              <Icon name="x" />
            </button>
            {core().questLine(e.questName)}
            {#if e.startDate}<span class="muted small">{core().gameDate(e.startDate)}</span>{/if}
          </p>
        {/if}
      {/each}
    {/if}
  {/each}
  {#if arr(r, ["completedQuestDataManager", "foundClues"]).length}
    <p><span class="muted">{app.t("o.questClues")}</span> {arr(r, ["completedQuestDataManager", "foundClues"]).filter((x) => typeof x === "string").join(", ")}</p>
  {/if}
</section>

<style>
  .info {
    display: grid;
    grid-template-columns: calc(268px + var(--s-md)) 1fr;
    gap: 4px 0;
    margin: 0 0 var(--s-sm);
  }
  dt {
    padding-inline-start: 8px;
  }
  @media (max-width: 520px) {
    .info {
      grid-template-columns: max-content 1fr;
      gap: 4px 12px;
    }
    dt {
      padding-inline-start: 0;
    }
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  h4 {
    margin: var(--s-sm) 0 0;
  }
  .who {
    font-weight: 600;
    margin: var(--s-sm) 0 0;
  }
  .offer {
    border-top: 1px solid var(--border);
    padding: var(--s-sm) 0;
  }
  .offer p {
    margin: 0 0 4px;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: var(--s-sm) 0 0;
    max-height: 420px;
    overflow: auto;
  }
  .list li {
    padding: 2px 0;
  }
  .mem {
    margin-top: var(--s-sm);
  }
  .small {
    min-height: 26px;
    font-size: 12px;
  }
  p {
    margin: 2px 0;
  }
</style>
