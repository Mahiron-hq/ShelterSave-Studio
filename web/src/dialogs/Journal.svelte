<script lang="ts">
  import Modal from "../components/Modal.svelte"
  import Icon from "../components/Icon.svelte"
  import { core } from "../lib/core"
  import { app } from "../lib/state.svelte"

  const h = $derived.by(() => {
    void app.rev
    return JSON.parse(core().history()) as { done: { text: string; affected: number }[]; undone: string[] }
  })

  function goTo(n: number) {
    if (core().goTo(n)) app.refresh()
  }
</script>

<Modal title={app.t("journal.title")}>
  <ol class="steps" start="0">
    <li>
      <button type="button" class="step" class:current={h.done.length === 0} onclick={() => goTo(0)} title={app.t("journal.goTo")}>
        {app.t("journal.opened")}
      </button>
    </li>
    {#each h.done as d, i (i)}
      <li>
        <button type="button" class="step" class:current={h.done.length === i + 1} onclick={() => goTo(i + 1)} title={app.t("journal.goTo")}>
          {d.text} ({d.affected})
        </button>
      </li>
    {/each}
    {#each h.undone as u, i (i)}
      <li>
        <button type="button" class="step undone" onclick={() => goTo(h.done.length + i + 1)} title={app.t("journal.goTo")}>
          {u} – {app.t("journal.undone")}
        </button>
      </li>
    {/each}
  </ol>
  <div class="dialog-actions">
    <button
      type="button"
      title={app.t("journal.resetHelp")}
      onclick={() => {
        if (core().resetToOriginal()) app.refresh()
      }}><Icon name="reset" /> {app.t("journal.reset")}</button
    >
    <button type="button" class="primary" onclick={() => app.pop()}>{app.t("c.close")}</button>
  </div>
</Modal>

<style>
  .steps {
    max-height: 420px;
    overflow: auto;
    padding-inline-start: 32px;
    margin: 0;
  }
  .step {
    border-color: transparent;
    background: transparent;
    text-align: start;
    width: 100%;
    justify-content: flex-start;
    min-height: 28px;
    padding: 2px 6px;
  }
  .step.current {
    background: var(--accent-soft);
    font-weight: 600;
  }
  .step.undone {
    color: var(--muted);
  }
</style>
