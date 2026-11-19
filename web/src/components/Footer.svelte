<script lang="ts">
  import { app } from "../lib/state.svelte"

  const legal = ["privacy", "terms", "disclaimer", "storage", "license", "contacts"]
</script>

<footer class="foot">
  <p>
    Copyright (C) 2026 Lev Burmistrov (Mahiron-hq) · {app.t("about.version", { v: "1.0.0" })} ·
    <a href="https://github.com/Mahiron-hq/ShelterSave-Studio" rel="noopener">GitHub</a> ·
    <a href="https://github.com/Mahiron-hq/ShelterSave-Studio/blob/main/LICENSE" rel="noopener">GPL-3.0</a>
  </p>
  <p>
    {#each legal as page, i (page)}{#if i > 0}&nbsp;·&nbsp;{/if}<a href="/{app.lang}/{page}/">{app.t(`page.${page}`)}</a>{/each}
  </p>
  <p class="muted">{app.t("app.unaffiliated")}</p>
</footer>

<style>
  .foot {
    border-top: 1px solid var(--border);
    background: var(--surface);
    padding: var(--s-md) var(--s-lg);
    font-size: 12px;
    text-align: center;
  }
  p {
    margin: 2px 0;
  }
</style>
