<script lang="ts">
  import DurationRow from "./DurationRow.svelte"
  import Icon from "../components/Icon.svelte"
  import { app } from "../lib/state.svelte"

  interface Props {
    id: number
    label: string
    help?: string
  }
  let { id, label, help = "" }: Props = $props()

  const task = $derived.by(() => {
    const tasks: any[] = app.root?.taskMgr?.tasks ?? []
    const i = tasks.findIndex((t) => t?.id === id)
    const now = app.root?.taskMgr?.time
    if (i < 0 || typeof now !== "number" || typeof tasks[i].endTime !== "number") return null
    return { i, left: Math.max(0, tasks[i].endTime - now) }
  })
</script>

{#if task}
  <DurationRow
    path={["taskMgr", "tasks", task.i]}
    {label}
    {help}
    secs={task.left}
    onset={(secs) => app.act({ action: "setTaskTime", id, seconds: secs })}
  />
  {#if task.left > 2}
    <button type="button" class="now" title={app.t("o.timerNowHelp")} onclick={() => app.act({ action: "setTaskTime", id, seconds: 1 })}>
      <Icon name="check" /> {app.t("o.timerNow")}
    </button>
  {/if}
{/if}

<style>
  .now {
    margin: 0 0 var(--s-xs) var(--s-sm);
    min-height: 26px;
    font-size: 12px;
  }
</style>
