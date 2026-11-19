<script lang="ts">
  import { FileInput, FileInputDropzone, FileInputTrigger } from "@ui-sdk/svelte/file-input"
  import Icon from "./Icon.svelte"
  import { app } from "../lib/state.svelte"
  import { MAX_FILE, openFile } from "../lib/files"
</script>

<div class="start">
  <FileInput
    maxSize={MAX_FILE}
    value={[]}
    onValueChange={(files) => {
      if (files[0]) void openFile(files[0])
    }}
    onReject={(rejected) => {
      const r = rejected[0]
      if (r?.reasons.includes("file-too-large")) app.loadError = app.t("err.too_large")
    }}
  >
    <FileInputDropzone class="drop">
      <Icon name="fileUp" big />
      <h1 class="title">{app.t("start.title")}</h1>
      <p class="muted">{app.t("web.dropHere")}</p>
      <p class="muted small">{app.t("web.or")}</p>
      <FileInputTrigger class="primary"><Icon name="open" /> {app.t("start.choose")}</FileInputTrigger>
      <p class="muted small">{app.t("start.text")}</p>
    </FileInputDropzone>
  </FileInput>

  <p class="local"><Icon name="lock" /> {app.t("start.local")}</p>

  {#if app.loadError}
    <div class="callout danger" role="alert">
      <Icon name="alert" />
      <div>
        <strong>{app.t("err.title")}</strong>
        <p>{app.loadError}</p>
        <p class="small">{app.t("err.what")}</p>
      </div>
    </div>
  {/if}

  <section class="panel where" aria-labelledby="where-title">
    <h2 id="where-title">{app.t("web.where.title")}</h2>
    <h3><Icon name="monitor" /> {app.t("web.where.steam")}</h3>
    <p>{app.t("web.where.steamPath")}</p>
    <h3><Icon name="smartphone" /> {app.t("web.where.android")}</h3>
    <p>{app.t("web.where.androidPath")}</p>
    <p class="names"><Icon name="info" /> {app.t("web.where.names")}</p>
    <div class="callout warning">
      <Icon name="warn" />
      <div>
        <p>{app.t("start.gameClosed")}</p>
        <p>{app.t("web.where.cloud")}</p>
      </div>
    </div>
    <p>{app.t("web.where.back")}</p>
    <a href="/{app.lang}/help/">{app.t("web.where.more")}</a>
  </section>
</div>

<style>
  .start {
    max-width: 760px;
    margin: 0 auto;
    padding: var(--s-lg) 0;
  }
  .start :global([data-slot="file-input-dropzone"]) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-xs);
    padding: 40px var(--s-lg);
    border: 2px dashed var(--border);
    border-radius: var(--r-lg);
    background: var(--surface);
    text-align: center;
  }
  .start :global([data-slot="file-input-dropzone"][data-dragging]) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .start :global([data-slot="file-input-dropzone"][data-drag-reject]) {
    border-color: var(--danger);
    background: var(--danger-soft);
  }
  .start :global([data-slot="file-input-dropzone"] > .icon) {
    color: var(--accent);
  }
  .start :global([data-slot="file-input-dropzone"] p) {
    margin: 0;
  }
  .title {
    font-size: 20px;
  }
  .local {
    display: flex;
    gap: 6px;
    align-items: center;
    justify-content: center;
    color: var(--muted);
  }
  .where h2 {
    margin-bottom: var(--s-md);
  }
  .where h3 {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-top: var(--s-md);
  }
  .names {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    font-weight: 600;
  }
  .names :global(svg) {
    flex: none;
    margin-top: 3px;
  }
  .callout p {
    margin: 0 0 4px;
  }
</style>
