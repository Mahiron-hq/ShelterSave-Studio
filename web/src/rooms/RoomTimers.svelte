<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import TaskTimer from "../fields/TaskTimer.svelte"
  import ItemCard from "../items/ItemCard.svelte"
  import ThemeCard from "./ThemeCard.svelte"
  import { catalog, nameOf, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { arr, DWELLERS, dwellerName } from "../lib/vault"

  interface Props {
    room: any
    base: Path
  }
  let { room, base }: Props = $props()

  function name(id: number): string {
    const d = arr(app.root, DWELLERS).find((x) => x.serializeId === id)
    return d ? dwellerName(d) : `#${id}`
  }

  const running = $derived.by(() => {
    const rows: [number, string, string][] = []
    for (const s of room.slots ?? [])
      if (s?.dwellerID >= 0 && s?.taskID >= 0) rows.push([s.taskID, app.t("room.trainingOf", { name: name(s.dwellerID) }), "room.trainingHelp"])
    for (const pr of room.partners ?? [])
      if (pr?.s === "RaisingBaby" && typeof pr.f === "number" && pr.t >= 0) rows.push([pr.t, app.t("room.birthOf", { name: name(pr.f) }), "room.birthHelp"])
    for (const ch of room.children ?? [])
      if (typeof ch?.dwellerID === "number" && ch.taskID >= 0) rows.push([ch.taskID, app.t("room.growUpOf", { name: name(ch.dwellerID) }), "room.growUpHelp"])
    if (room.type === "Radio" && room.currentState?.taskId >= 0) rows.push([room.currentState.taskId, app.t("room.radioIn"), "room.radioInHelp"])
    const tasks: any[] = app.root?.taskMgr?.tasks ?? []
    return rows.filter(([t]) => tasks.some((x) => x?.id === t))
  })

  const craft = $derived.by(() => {
    const id = typeof room.CraftingItemId === "string" && room.CraftingItemId ? room.CraftingItemId : null
    if (!id) return null
    const cat = catalog()
    const w = (cat.weapons as any[]).find((x) => x.id === id)
    const o = (cat.outfits as any[]).find((x) => x.id === id)
    const t = (cat.themes as any[]).find((x) => x.id === id)
    const kind = w ? "Weapon" : o ? "Outfit" : t ? "Theme" : null
    const info = w ?? o ?? t
    const required: number = info?.craft?.time ?? 0
    const done: number = typeof room.CompletedTime === "number" ? room.CompletedTime : 0
    return { id, kind, name: info ? nameOf(info.name, app.ru, id) : id, required, done, ready: required > 0 && done >= required }
  })
</script>

{#if running.length || craft}
  <h4 title={app.t("room.timersHelp")}>{app.t("room.timers")}</h4>
  {#each running as [t, label, help] (t)}
    <TaskTimer id={t} {label} help={app.t(help)} />
  {/each}
  {#if craft}
    <p>
      {#if craft.kind === "Weapon" || craft.kind === "Outfit"}
        {app.t("room.craftingOf", { name: "" })}<ItemCard kind={craft.kind} id={craft.id} text={craft.name} />
      {:else if craft.kind === "Theme"}
        {app.t("room.craftingOf", { name: "" })}<ThemeCard id={craft.id} text={craft.name} />
      {:else}
        {app.t("room.craftingOf", { name: craft.name })}
      {/if}
    </p>
    {#if craft.required > 0}
      {@const part = Math.min(1, Math.max(0, craft.done / craft.required))}
      <progress max="1" value={part} title={app.t("room.craftProgressHelp")}></progress>
      <span class="small muted">{app.t("room.craftProgress", { pct: (part * 100).toFixed(1) })}</span>
    {/if}
    {#if craft.ready}
      <p class="ok"><Icon name="check" /> {app.t("room.craftReady")}</p>
    {:else}
      <button
        type="button"
        title={app.t("room.craftNowHelp")}
        onclick={() => app.act({ action: "setNumber", path: [...base, "CompletedTime"], value: craft.required > 0 ? craft.required : 9999999 })}
      >
        <Icon name="check" /> {app.t("room.craftNow")}
      </button>
    {/if}
  {/if}
{/if}

<style>
  h4 {
    margin: var(--s-sm) 0 0;
  }
  progress {
    width: min(320px, 100%);
    display: block;
  }
  .ok {
    color: var(--success);
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
