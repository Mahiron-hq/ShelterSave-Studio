<script lang="ts">
  import Modal from "../components/Modal.svelte"
  import Choice from "../components/Choice.svelte"
  import { app } from "../lib/state.svelte"
  import { clearSettings, defaults, type LangPref, type ThemePref } from "../lib/settings"
  import { clearAll } from "../lib/session"
  import { art, loadGameFile } from "../lib/art.svelte"

  function set<K extends keyof typeof app.settings>(k: K, v: (typeof app.settings)[K]) {
    app.settings[k] = v
    app.saveSettings()
  }

  function setLang(v: LangPref) {
    set("lang", v)
    const wanted = app.wantedLang()
    if (wanted !== app.lang) {
      app.keepSession()
      setTimeout(() => location.assign(`/${wanted}/`), 450)
    }
  }

  const onOff = $derived([
    { value: "on" as const, label: app.t("set.on") },
    { value: "off" as const, label: app.t("set.off") },
  ])
</script>

<Modal title={app.t("set.title")} wide>
  <h3>{app.t("set.appearance")}</h3>
  <Choice
    label={app.t("set.language")}
    help={app.t("set.languageHelp")}
    value={app.settings.lang}
    options={[
      { value: "system", label: app.t("set.lang.browser") },
      { value: "ru", label: app.t("set.lang.ru") },
      { value: "en", label: app.t("set.lang.en") },
    ]}
    onchange={setLang}
  />
  <Choice
    label={app.t("set.theme")}
    help={app.t("set.themeHelp")}
    value={app.settings.theme}
    options={[
      { value: "system" as ThemePref, label: app.t("set.theme.system") },
      { value: "light" as ThemePref, label: app.t("set.theme.light") },
      { value: "dark" as ThemePref, label: app.t("set.theme.dark") },
    ]}
    onchange={(v) => set("theme", v)}
  />

  <h3>{app.t("set.editor")}</h3>
  <Choice
    label={app.t("set.itemNames")}
    help={app.t("set.itemNamesHelp")}
    value={app.settings.russianItemNames ? "ru" : "en"}
    options={[
      { value: "en", label: app.t("set.itemNames.en") },
      { value: "ru", label: app.t("set.itemNames.ru") },
    ]}
    onchange={(v) => set("russianItemNames", v === "ru")}
  />
  <Choice
    label={app.t("set.mode")}
    help={app.t("set.modeHelp")}
    value={app.settings.expert ? "expert" : "normal"}
    options={[
      { value: "normal", label: app.t("set.mode.normal") },
      { value: "expert", label: app.t("set.mode.expert") },
    ]}
    onchange={(v) => {
      if (v === "expert" && !app.settings.expert) app.push({ kind: "expertConfirm" })
      else if (v === "normal") set("expert", false)
    }}
  />
  <Choice
    label={app.t("set.confirmMass")}
    help={app.t("set.confirmMassHelp")}
    value={app.settings.confirmMass ? "on" : "off"}
    options={onOff}
    onchange={(v) => set("confirmMass", v === "on")}
  />
  <Choice
    label={app.t("set.dwellerScroll")}
    help={app.t("set.dwellerScrollHelp")}
    value={app.settings.dwellerScroll}
    options={[
      { value: "shared", label: app.t("set.dwellerScroll.shared") },
      { value: "top", label: app.t("set.dwellerScroll.top") },
    ]}
    onchange={(v) => set("dwellerScroll", v)}
  />

  <h3>{app.t("art.web.title")}</h3>
  <div class="row">
    <div>
      <strong>
        {#if art.status === "ready"}{app.t("art.web.ready")}
        {:else if art.status === "loading"}{app.t("art.web.loading", { pct: art.progress })}
        {:else}{app.t("art.web.none")}{/if}
      </strong>
      {#if art.status === "error"}<p class="small error">{art.error}</p>{/if}
      <p class="muted small">{app.t("art.web.loadHelp")}</p>
      <button type="button" class="link small" onclick={() => app.push({ kind: "gameFile" })}>{app.t("art.web.how")}</button>
    </div>
    <button type="button" disabled={art.status === "loading"} onclick={() => loadGameFile()}>{app.t("art.web.load")}</button>
  </div>

  <h3>{app.t("set.privacy")}</h3>
  <Choice
    label={app.t("set.restoreSession")}
    help={app.t("set.restoreSessionHelp")}
    value={app.settings.keepSession ? "on" : "off"}
    options={onOff}
    onchange={(v) => set("keepSession", v === "on")}
  />
  <Choice
    label={app.t("set.stats")}
    help={app.t("set.statsHelp")}
    value={app.settings.stats ? "on" : "off"}
    options={onOff}
    onchange={(v) => set("stats", v === "on")}
  />
  <div class="row">
    <div>
      <strong>{app.t("set.clearData")}</strong>
      <p class="muted small">{app.t("set.clearDataHelp")}</p>
    </div>
    <button
      type="button"
      class="danger"
      onclick={async () => {
        await clearAll()
        Object.assign(app.settings, defaults)
        app.saveSettings()
        clearSettings()
        app.toast(app.t("set.clearDataDone"))
      }}>{app.t("set.clearData")}</button
    >
  </div>
  <div class="row">
    <div>
      <strong>{app.t("set.reset")}</strong>
      <p class="muted small">{app.t("set.resetHelp")}</p>
    </div>
    <button
      type="button"
      onclick={() => {
        const seen = app.settings.firstNoticeSeen
        Object.assign(app.settings, defaults, { firstNoticeSeen: seen })
        app.saveSettings()
        app.toast(app.t("set.resetDone"))
      }}>{app.t("set.reset")}</button
    >
  </div>
  <div class="dialog-actions">
    <button type="button" class="primary" onclick={() => app.pop()}>{app.t("c.close")}</button>
  </div>
</Modal>

<style>
  .link {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  h3 {
    margin-top: var(--s-lg);
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: var(--s-md);
    align-items: center;
    padding: var(--s-sm) 0;
    border-bottom: 1px solid var(--border);
  }
  .row p {
    margin: 2px 0 0;
  }
  .error {
    color: var(--danger);
  }
</style>
