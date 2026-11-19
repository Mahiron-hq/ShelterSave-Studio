<script lang="ts">
  import Icon from "./Icon.svelte"
  import { app, type Section } from "../lib/state.svelte"
  import type { IconName } from "../lib/icons"

  const items: [Section, IconName, string, number][] = [
    ["save", "shield", "nav.save", 1],
    ["vault", "house", "nav.vault", 2],
    ["dwellers", "users", "nav.dwellers", 3],
    ["other", "layers", "nav.other", 4],
  ]
</script>

<nav class="nav" aria-label={app.t("nav.sections")}>
  <ul>
    {#each items as [s, icon, key, n] (s)}
      <li>
        <button
          type="button"
          class="item"
          class:active={app.section === s}
          aria-current={app.section === s ? "page" : undefined}
          aria-keyshortcuts="Alt+{n}"
          title="Alt+{n}"
          onclick={() => (app.section = s)}
        >
          <Icon name={icon} />
          <span>{app.t(key)}</span>
        </button>
      </li>
    {/each}
  </ul>
  <div class="file">
    <strong>{app.fileName}</strong>
    <button type="button" onclick={() => (app.dirty ? app.push({ kind: "closeConfirm" }) : app.close())}>
      <Icon name="x" />
      {app.t("tb.close")}
    </button>
  </div>
</nav>

<style>
  .nav {
    background: var(--surface);
    border-inline-end: 1px solid var(--border);
    padding: var(--s-md);
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    width: 100%;
    border-color: transparent;
    background: transparent;
    font-size: 15px;
    min-height: 36px;
    justify-content: flex-start;
  }
  .item.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .file {
    border-top: 1px solid var(--border);
    padding-top: var(--s-md);
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
    overflow-wrap: anywhere;
  }
  @media (max-width: 820px) {
    .nav {
      border-inline-end: 0;
      border-bottom: 1px solid var(--border);
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
      padding: var(--s-xs) var(--s-sm);
    }
    ul {
      flex-direction: row;
      flex-wrap: wrap;
    }
    .item {
      width: auto;
    }
    .file {
      border-top: 0;
      padding-top: 0;
      flex-direction: row;
      align-items: center;
      margin-inline-start: auto;
    }
  }
</style>
