<script lang="ts">
  import Modal from "../components/Modal.svelte"
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import Settings from "./Settings.svelte"
  import Review from "./Review.svelte"
  import Journal from "./Journal.svelte"
  import Custom from "./Custom.svelte"
  import GameFile from "./GameFile.svelte"
  import { catalog, core } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { clearSession } from "../lib/session"

  const top = $derived(app.modals.at(-1))
  let keepSession = $state(false)

  function formatDate(ms: number): string {
    return new Intl.DateTimeFormat(app.lang, { dateStyle: "medium", timeStyle: "short" }).format(new Date(ms))
  }

  async function restore(session: string) {
    app.pop()
    try {
      core().restore(session)
      app.refresh(false)
      app.toast(app.t("recover.done"))
    } catch {
      void clearSession()
      app.toastError(app.t("err.not_json").replace("{detail}", ""))
    }
  }
</script>

{#if top?.kind === "settings"}
  <Settings />
{:else if top?.kind === "review"}
  <Review />
{:else if top?.kind === "journal"}
  <Journal />
{:else if top?.kind === "gameFile"}
  <GameFile />
{:else if top?.kind === "about"}
  <Modal title={app.t("about.title")}>
    <p><strong>{app.t("app.name")}</strong> — {app.t("app.tagline")}</p>
    <p>{app.t("about.version", { v: "1.0.0" })}</p>
    <p>{app.t("app.copyright")}</p>
    <p>{app.t("about.license")}</p>
    <p class="muted">{app.t("about.catalog", { c: catalog().catalogVersion, g: catalog().gameVersion })}</p>
    <p class="muted">{app.t("about.disclaimer")}</p>
    <p class="muted">{app.t("app.unaffiliated")}</p>
    <ul class="links">
      <li><a href="https://github.com/Mahiron-hq/ShelterSave-Studio" rel="noopener">{app.t("about.repo")} <Icon name="external" /></a></li>
      <li><a href="/{app.lang}/privacy/">{app.t("about.privacy")}</a></li>
      <li><a href="/{app.lang}/license/">{app.t("about.thirdParty")}</a></li>
    </ul>
    <div class="dialog-actions">
      <button type="button" class="primary" onclick={() => app.pop()}>{app.t("c.close")}</button>
    </div>
  </Modal>
{:else if top?.kind === "expertConfirm"}
  <Modal title={app.t("set.expertTitle")}>
    <div class="callout warning"><Icon name="warn" /><p>{app.t("set.expertText")}</p></div>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="primary"
        onclick={() => {
          app.settings.expert = true
          app.saveSettings()
          app.pop()
        }}>{app.t("set.expertEnable")}</button
      >
    </div>
  </Modal>
{:else if top?.kind === "closeConfirm"}
  <Modal title={app.t("close.title")}>
    <p>{app.t("close.text")}</p>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="danger"
        onclick={() => {
          app.pop()
          app.close()
        }}>{app.t("close.discard")}</button
      >
      <button
        type="button"
        class="primary"
        onclick={() => {
          app.pop()
          app.push({ kind: "review" })
        }}><Icon name="download" /> {app.t("tb.download")}</button
      >
    </div>
  </Modal>
{:else if top?.kind === "confirmMass"}
  <Modal title={app.t("c.confirmTitle")}>
    <p>{app.t("c.confirmMass", { n: top.count })}</p>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="primary"
        onclick={() => {
          const a = top.action
          app.pop()
          app.act(a)
        }}>{app.t("c.confirm")}</button
      >
    </div>
  </Modal>
{:else if top?.kind === "restoreSession"}
  <Modal
    title={app.t("web.restore.title")}
    onclose={() => {
      app.pop()
    }}
  >
    <p>{app.t("web.restore.text", { file: top.fileName, date: formatDate(top.savedAt) })}</p>
    <div class="dialog-actions">
      <button
        type="button"
        class="danger"
        onclick={() => {
          app.pop()
          void clearSession()
        }}>{app.t("web.restore.no")}</button
      >
      <button type="button" class="primary" onclick={() => restore(top.session)}>{app.t("web.restore.yes")}</button>
    </div>
  </Modal>
{:else if top?.kind === "afterDownload"}
  <Modal title={app.t("web.download.title")}>
    <p>{app.t("review.saved", { file: top.name })}</p>
    <div class="callout info"><Icon name="info" /><p>{app.t("review.downloadHint")}</p></div>
    <p class="muted small">{app.t("web.where.steamPath")}</p>
    <p class="muted small">{app.t("web.where.androidPath")}</p>
    {#if app.settings.keepSession}
      <Check checked={keepSession} label={app.t("web.download.keep")} onchange={(v) => (keepSession = v)} />
    {/if}
    <div class="dialog-actions">
      <button
        type="button"
        class="primary"
        onclick={() => {
          if (!keepSession) void clearSession()
          keepSession = false
          app.pop()
        }}>{app.t("c.ok")}</button
      >
    </div>
  </Modal>
{:else if top?.kind === "custom" && top.id === "replaceFile"}
  <Modal title={app.t("close.title")}>
    <p>{app.t("close.text")}</p>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="danger"
        onclick={() => {
          const run = top.data
          app.pop()
          run?.()
        }}>{app.t("close.discard")}</button
      >
    </div>
  </Modal>
{:else if top?.kind === "custom"}
  <Custom modal={top} />
{/if}

<style>
  .links {
    padding-inline-start: 18px;
  }
  .callout p {
    margin: 0;
  }
</style>
