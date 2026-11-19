<script lang="ts">
  import Icon from "./Icon.svelte"
  import { app } from "../lib/state.svelte"
</script>

{#if !app.settings.firstNoticeSeen}
  <aside class="notice" aria-label={app.t("web.notice.policy")}>
    <Icon name="lock" />
    <p>
      {app.t("web.notice.text")}
      <a href="/{app.lang}/privacy/">{app.t("web.notice.policy")}</a>
    </p>
    <div class="actions">
      <button
        type="button"
        onclick={() => {
          app.settings.stats = false
          app.settings.firstNoticeSeen = true
          app.saveSettings()
        }}>{app.t("web.notice.optOut")}</button
      >
      <button
        type="button"
        class="primary"
        onclick={() => {
          app.settings.firstNoticeSeen = true
          app.saveSettings()
        }}>{app.t("web.notice.ok")}</button
      >
    </div>
  </aside>
{/if}

<style>
  .notice {
    position: fixed;
    inset-inline: 16px;
    inset-block-end: 16px;
    z-index: 75;
    max-width: 720px;
    margin: 0 auto;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-sm) var(--s-md);
    align-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    padding: var(--s-md);
  }
  p {
    margin: 0;
    flex: 1 1 300px;
  }
  .actions {
    display: flex;
    gap: var(--s-sm);
  }
</style>
