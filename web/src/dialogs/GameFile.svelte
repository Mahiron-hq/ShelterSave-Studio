<script lang="ts">
  import Modal from "../components/Modal.svelte"
  import Icon from "../components/Icon.svelte"
  import IconButton from "../components/IconButton.svelte"
  import { catalog } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { art, loadGameFile } from "../lib/art.svelte"

  const STEAM = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Fallout Shelter\\FalloutShelter_Data\\data.unity3d"
  const BETHESDA = "C:\\Program Files (x86)\\Bethesda.net Launcher\\games\\Fallout Shelter\\FalloutShelter_Data\\data.unity3d"

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text)
      app.toast(app.t("art.web.copied"))
    } catch {
      app.toastError(text)
    }
  }

  function choose() {
    void loadGameFile(() => app.pop())
  }
</script>

{#snippet path(text: string)}
  <div class="path">
    <code>{text}</code>
    <IconButton icon="copy" label={app.t("art.web.copy")} onclick={() => copy(text)} />
  </div>
{/snippet}

<Modal title={app.t("art.web.title")} wide>
  <p>{app.t("art.web.why")}</p>

  <h3>{app.t("art.web.fileTitle")}</h3>
  <p>{app.t("art.web.file")}</p>

  <h3><Icon name="monitor" /> {app.t("art.web.steamTitle")}</h3>
  <p>{app.t("art.web.steam")}</p>
  <p class="small muted">{app.t("art.web.steamPath")}</p>
  {@render path(STEAM)}
  <p class="small muted">{app.t("art.web.steamOther")}</p>

  <h3><Icon name="monitor" /> {app.t("art.web.bethesdaTitle")}</h3>
  {@render path(BETHESDA)}

  <p class="small">{app.t("art.web.paste")}</p>

  <div class="callout">
    <Icon name="lock" />
    <div>
      <p>{app.t("art.web.private")}</p>
      <p class="small muted">{app.t("art.web.version", { v: catalog().gameVersion })}</p>
    </div>
  </div>
  <p class="small"><Icon name="smartphone" /> {app.t("art.web.phone")}</p>
  {#if art.status === "error"}<p class="err" role="alert">{art.error}</p>{/if}

  <div class="dialog-actions">
    <a href="/{app.lang}/desktop/" title={app.t("art.web.desktopNote")}>{app.t("art.web.desktop")}</a>
    <button type="button" onclick={() => app.pop()}>{app.t("c.close")}</button>
    <button type="button" class="primary" onclick={choose}><Icon name="fileUp" /> {app.t("art.web.choose")}</button>
  </div>
</Modal>

<style>
  h3 {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: var(--s-md) 0 var(--s-xs);
    font-size: 15px;
  }
  p {
    margin: 0 0 6px;
  }
  .path {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0 0 6px;
  }
  code {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    padding: 4px 8px;
    border-radius: var(--r-sm);
    background: var(--surface-alt);
    font-size: 13px;
  }
  .callout {
    margin: var(--s-md) 0 var(--s-sm);
  }
  .err {
    color: var(--danger);
  }
  .dialog-actions a {
    margin-inline-end: auto;
    align-self: center;
  }
</style>
