<script lang="ts">
  import { Toolbar, ToolbarSeparator } from "@ui-sdk/svelte/toolbar"
  import ToolButton from "./ToolButton.svelte"
  import IconButton from "./IconButton.svelte"
  import Icon from "./Icon.svelte"
  import Search from "./Search.svelte"
  import { app } from "../lib/state.svelte"
  import { openFile, pickFile } from "../lib/files"

  async function open() {
    if (app.dirty) {
      app.push({ kind: "closeConfirm" })
      return
    }
    const f = await pickFile()
    if (f) await openFile(f)
  }
</script>


<header class="toolbar">
  <a class="brand" href="/{app.lang}/">{app.t("app.name")}</a>
  <Toolbar class="tools" aria-label={app.t("app.name")}>
    <ToolButton icon="open" label={app.t("tb.open")} onclick={open} />
    <ToolButton icon="download" label={app.t("tb.download")} onclick={() => app.push({ kind: "review" })} disabled={!app.isOpen} />
    <ToolbarSeparator class="sep" />
    <ToolButton icon="undo" label={`${app.t("tb.undo")} (Ctrl+Z)`} onclick={() => app.undo()} disabled={!app.canUndo} />
    <ToolButton icon="redo" label={`${app.t("tb.redo")} (Ctrl+Y)`} onclick={() => app.redo()} disabled={!app.canRedo} />
    <ToolButton icon="history" label={app.t("tb.journal")} onclick={() => app.push({ kind: "journal" })} disabled={!app.isOpen} />
  </Toolbar>
  {#if app.isOpen}
    <div class="search"><Search /></div>
  {/if}
  <div class="status">
    {#if app.isOpen}
      {#if app.dirty}
        <span class="badge mark"><span aria-hidden="true">●</span> {app.t("tb.unsaved")}</span>
      {:else}
        <span class="badge">{app.t("tb.saved")}</span>
      {/if}
    {/if}
    <span class="badge" class:warning={app.expert} title={app.t("tb.modeHelp")}>
      {app.expert ? app.t("tb.mode.expert") : app.t("tb.mode.normal")}
    </span>
    <IconButton icon="settings" label={app.t("tb.settings")} onclick={() => app.push({ kind: "settings" })} />
    <a class="btn icon ghost" href="/{app.lang}/help/" aria-label={app.t("tb.help")} title={app.t("tb.help")}>
      <Icon name="help" />
    </a>
    <IconButton icon="info" label={app.t("tb.about")} onclick={() => app.push({ kind: "about" })} />
  </div>
</header>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-sm);
    padding: var(--s-sm) var(--s-md);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    font-weight: 600;
    font-size: 16px;
    color: var(--text);
    text-decoration: none;
    margin-inline-end: var(--s-sm);
  }
  .toolbar :global(.tools) {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .toolbar :global(.sep) {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 4px;
  }
  .search {
    flex: 1 1 200px;
    min-width: 0;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-inline-start: auto;
  }
  .btn.icon {
    padding: 6px;
    min-width: 32px;
    justify-content: center;
    border-color: transparent;
    background: transparent;
  }
  @media (max-width: 640px) {
    .brand {
      display: none;
    }
    .status .badge {
      display: none;
    }
  }
</style>
