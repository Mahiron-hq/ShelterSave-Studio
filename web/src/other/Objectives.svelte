<script lang="ts">
  import { Combobox, ComboboxContent, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList } from "@ui-sdk/svelte/combobox"
  import { Editor } from "../wasm/shelter_wasm.js"
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import NumField from "../fields/NumField.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import TaskTimer from "../fields/TaskTimer.svelte"
  import { catalog, core, nameOf } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { arr } from "../lib/vault"

  const slots = $derived(arr(app.root, ["objectiveMgr", "slotArray"]))
  const used = $derived<string[]>(slots.map((s) => s?.objective?.objectiveID).filter((x) => typeof x === "string"))
  const survival = $derived(app.root?.vault?.VaultMode === "Survival")
  let queries = $state<Record<number, string>>({})

  function options(i: number, id: string, level: number) {
    const q = (queries[i] ?? "").trim().toLowerCase()
    const list = JSON.parse(core().objectiveOptions(level, survival, used, id)) as { id: string; text: string; tier: number; reward: string; level: number }[]
    return list.filter((o) => o.id !== id && (!q || o.text.toLowerCase().includes(q) || o.id.toLowerCase().includes(q)))
  }

  const ach = $derived(app.root?.vault?.Achievements)
  const done = $derived<string[]>((ach?.completed ?? []).filter((x: unknown) => typeof x === "string"))
  const achList = $derived((catalog().achievements as any[]).filter((a) => app.expert || a.steamId))

  function achName(a: any): string {
    const k = `ach.${a.id}`
    const t = app.t(k)
    return t === k ? nameOf(a.name, app.ru, a.id) : t
  }

  function progress(id: string) {
    const list = arr(app.root, ["vault", "Achievements", "objectivesInProgress"])
    const i = list.findIndex((o) => o?.objectiveID === id)
    if (i < 0) return null
    const req = list[i]?.requirements?.[0]
    if (!req || typeof req !== "object") return null
    // The first number is the counter; ids and flags are skipped.
    const entry = Object.entries(req).find(([k, v]) => !["dweller", "acceptedRoom", "requirementID"].includes(k) && typeof v === "number")
    if (!entry) return null
    return { path: ["vault", "Achievements", "objectivesInProgress", i, "requirements", 0, entry[0]], value: entry[1] as number }
  }
</script>

<section class="panel">
  <h3>{app.t("o.currentObjectives")}</h3>
  <p class="muted">{app.t("o.currentObjectivesHelp")}</p>
  {#if slots.length === 0}<p class="muted">{app.t("c.empty")}</p>{/if}
</section>

{#each slots as s, i (i)}
  {@const id = s?.objective?.objectiveID}
  {#if typeof id === "string"}
    {@const level = s.objective.incrementLevel ?? 0}
    {@const req = s.objective.requirements?.[0]}
    {@const isDone = s.objective.completed === true}
    {@const max = Editor.objectiveMaxLevel(id)}
    <section class="panel">
      <h3 class="obj"><Icon name="flag" /> {core().objectiveText(id, level, req ? JSON.stringify(req) : "")}</h3>
      <p><span class="muted">{app.t("o.reward")}</span> {core().rewardText(id, level)}</p>
      {#if app.expert}<p class="mono muted small">{id}</p>{/if}
      {#if isDone}<p class="ok"><Icon name="check" /> {app.t("o.objectiveDone")}</p>{/if}
      {#if max !== undefined && max > 0}
        <NumField path={["objectiveMgr", "slotArray", i, "objective", "incrementLevel"]} label={app.t("o.objLevel")} help={app.t("o.objLevelHelp")} min={0} {max} />
      {/if}
      <div class="row-wrap">
        {#if !isDone}
          <button type="button" title={app.t("o.completeObjectiveHelp")} onclick={() => app.act({ action: "completeObjective", slot: i })}>
            <Icon name="check" /> {app.t("o.completeObjective")}
          </button>
        {/if}
        <Combobox
          filter={false}
          bind:query={() => queries[i] ?? "", (v) => (queries[i] = v)}
          value={null}
          onValueChange={(picked) => {
            const v = picked as unknown
            const o = typeof v === "string" ? options(i, id, level).find((x) => x.id === v) : null
            if (o) app.act({ action: "replaceObjective", slot: i, id: o.id, level: o.level })
          }}
        >
          <ComboboxInput aria-label={app.t("o.replaceObjective")} placeholder={app.t("o.replaceObjective")} title={app.t("o.replaceObjectiveHelp")} class="picker-input" />
          <ComboboxContent>
            <ComboboxEmpty>{app.t("c.nothingFound")}</ComboboxEmpty>
            <ComboboxList>
              {#each options(i, id, level) as o (o.id)}
                <ComboboxItem value={o.id} label={o.text}>
                  <span class="opt">{o.text}<span class="muted small">{app.t("o.objTier", { n: o.tier })}  ·  {o.reward}</span></span>
                </ComboboxItem>
              {/each}
            </ComboboxList>
          </ComboboxContent>
        </Combobox>
      </div>
    </section>
  {/if}
{/each}

{#if app.root?.objectiveMgr}
  {@const m = app.root.objectiveMgr}
  <section class="panel">
    <h3>{app.t("o.objSettings")}</h3>
    {#if m.canDiscard !== undefined}<BoolField path={["objectiveMgr", "canDiscard"]} label={app.t("o.freeSkip")} help={app.t("o.freeSkipHelp")} />{/if}
    {#if m.nukaQuantumIncrement !== undefined}
      <NumField path={["objectiveMgr", "nukaQuantumIncrement"]} label={app.t("o.quantumSkips")} help={app.t("o.quantumSkipsHelp")} min={0} max={10} />
    {/if}
    {#if typeof m.taskID === "number"}<TaskTimer id={m.taskID} label={app.t("o.newObjectivesIn")} help={app.t("o.newObjectivesInHelp")} />{/if}
  </section>
{/if}

{#if ach}
  <section class="panel">
    <h3>{app.t("o.achievements", { n: achList.filter((a) => done.includes(a.objectiveId)).length, all: achList.length })}</h3>
    <p class="muted">{app.t("o.achievementsHelp")}</p>
    <ul class="list">
      {#each achList as a (a.id)}
        {@const on = done.includes(a.objectiveId)}
        {@const pr = on ? null : progress(a.objectiveId)}
        <li title={app.expert ? `${a.objectiveId} / ${a.steamId}` : app.t("o.achievementToggleHelp")}>
          <Check checked={on} label={achName(a)} onchange={(v) => app.act({ action: "setAchievement", id: a.objectiveId, done: v })} />
          {#if pr}
            <NumPick
              value={pr.value}
              label={achName(a)}
              lo={0}
              hi={Math.max(a.goal > 0 ? a.goal : 1000000, pr.value)}
              oncommit={(v) => app.act({ action: "setNumber", path: pr.path, value: v })}
            />
            {#if a.goal > 0}<span class="muted small">{app.t("o.ofMax", { n: a.goal })}</span>{/if}
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .obj {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .obj :global(.icon) {
    color: var(--accent);
    margin-top: 3px;
  }
  .ok {
    color: var(--success);
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .opt {
    display: flex;
    flex-direction: column;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .list li {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
    padding: 3px 0;
  }
  p {
    margin: 4px 0;
  }
</style>
