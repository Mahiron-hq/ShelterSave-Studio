<script lang="ts">
  import Icon from "./Icon.svelte"
  import { app } from "../lib/state.svelte"
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast" class:error={t.error}>
      <Icon name={t.error ? "alert" : "check"} />
      <p>{t.text}</p>
      <div class="actions">
        {#if t.undo}
          <button
            type="button"
            onclick={() => {
              app.dismiss(t.id)
              app.undo()
            }}>{app.t("c.undo")}</button
          >
        {/if}
        <button type="button" class="icon ghost" aria-label={app.t("c.close")} onclick={() => app.dismiss(t.id)}>
          <Icon name="x" />
        </button>
      </div>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    inset-inline-end: 16px;
    inset-block-end: 16px;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: min(420px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    padding: 12px;
  }
  .toast :global(.icon) {
    color: var(--success);
    margin-top: 2px;
  }
  .toast.error {
    background: var(--danger-soft);
    border-color: var(--danger);
  }
  .toast.error :global(.icon) {
    color: var(--danger);
  }
  p {
    margin: 0;
    flex: 1;
  }
  .actions {
    display: flex;
    gap: 4px;
  }
</style>
