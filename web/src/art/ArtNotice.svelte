<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import { app } from "../lib/state.svelte"
  import { art, loadGameFile } from "../lib/art.svelte"

  interface Props {
    /** Small box in place of a picture instead of a line over a section. */
    compact?: boolean
    /** What is missing, when it is not pictures. */
    text?: string
  }
  let { compact = false, text }: Props = $props()
</script>

{#if art.status !== "ready" || (text && !art.texts)}
  <div class="notice" class:compact role="note">
    <Icon name="image" />
    {#if art.status === "loading"}
      <span class="text" role="status">{app.t("art.web.loading", { pct: art.progress })}</span>
      <progress max="100" value={art.progress} aria-hidden="true"></progress>
    {:else}
      <span class="text">
        {text ?? (compact ? app.t("art.web.noneShort") : app.t("art.web.none"))}
        {#if art.status === "error"}<span class="err">{art.error}</span>{/if}
      </span>
      <span class="actions">
        <button type="button" title={app.t("art.web.loadHelp")} onclick={() => loadGameFile()}>
          <Icon name="fileUp" /> {app.t("art.web.load")}
        </button>
        <button type="button" class="link" onclick={() => app.push({ kind: "gameFile" })}>{app.t("art.web.how")}</button>
      </span>
    {/if}
  </div>
{/if}

<style>
  .notice {
    display: flex;
    flex-wrap: wrap;
    gap: 6px var(--s-sm);
    align-items: center;
    border: 1px dashed var(--border);
    border-radius: var(--r-md);
    padding: 6px var(--s-sm);
    margin: 0 0 var(--s-sm);
    background: var(--surface-alt);
    font-size: 13px;
  }
  .notice > :global(svg) {
    color: var(--muted);
    flex: none;
  }
  .text {
    flex: 1 1 220px;
    min-width: 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .compact {
    flex-direction: column;
    align-items: flex-start;
    max-width: 260px;
    min-height: 120px;
    justify-content: center;
  }
  .compact .text {
    flex: none;
  }
  .compact .actions {
    flex-direction: column;
    align-items: flex-start;
  }
  .err {
    display: block;
    color: var(--danger);
  }
  progress {
    flex: 1 1 120px;
    accent-color: var(--accent);
  }
  .link {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
</style>
