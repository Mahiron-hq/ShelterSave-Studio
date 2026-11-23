<script lang="ts">
  import Modal from "../components/Modal.svelte"
  import Icon from "../components/Icon.svelte"
  import { core } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { download } from "../lib/files"
  import { fmtNum } from "../lib/format"
  import { hit } from "../lib/stats"

  interface Change {
    kind: "changed" | "added" | "removed"
    path: string
    place: string
    before: string | null
    after: string | null
  }
  interface Issue {
    severity: "error" | "warning"
    path: string
    text: string
  }
  interface Warning {
    code: string
    place: string
    value: string
  }
  interface Report {
    changes: Change[]
    newIssues: Issue[]
    oldIssues: Issue[]
    newWarnings: Warning[]
    oldWarnings: Warning[]
  }

  const report = $derived.by((): Report => {
    void app.rev
    return JSON.parse(core().saveReport())
  })
  const errors = $derived(report.newIssues.filter((i) => i.severity === "error").length)
  const LIMIT = 300

  function num(s: string | null): string | null {
    if (s === null) return null
    if (s.endsWith(" keys}") || s.endsWith(" items]")) return null
    return /^-?[\d.]+$/.test(s) && !Number.isNaN(Number(s)) ? fmtNum(Number(s)) : s
  }

  function save() {
    const name = core().fileName() || "Vault1.sav"
    const text = core().save()
    download(name, text)
    hit("save")
    app.refresh()
    app.pop()
    app.push({ kind: "afterDownload", name })
  }
</script>

<Modal title={app.t("review.title")} wide>
  <p class="muted"><Icon name="warn" /> {app.t("review.closeGame")}</p>
  <h3>{app.t("review.changes", { n: report.changes.length })}</h3>
  {#if report.changes.length === 0}
    <p class="muted">{app.t("review.noChanges")}</p>
  {:else}
    <ul class="list">
      {#each report.changes.slice(0, LIMIT) as ch (ch.kind + ch.path)}
        {@const b = num(ch.before)}
        {@const a = num(ch.after)}
        <li class={ch.kind}>
          <Icon name={ch.kind === "added" ? "plus" : ch.kind === "removed" ? "trash" : "pencil"} />
          <span class="sr-only">{app.t(`review.${ch.kind === "changed" ? "was" : ch.kind}`)}</span>
          <span>
            {ch.place}{#if b !== null && a !== null}: <span class="muted">{b}</span> → {a}{:else if a !== null}: {a}{:else if b !== null}: <span class="muted">{b}</span>{/if}
            {#if app.expert}<br /><span class="mono muted small">{ch.path}</span>{/if}
          </span>
        </li>
      {/each}
    </ul>
    {#if report.changes.length > LIMIT}
      <p class="muted">{app.t("review.more", { n: report.changes.length - LIMIT })}</p>
    {/if}
  {/if}

  {#if report.newIssues.length || report.oldIssues.length}
    <h3>{app.t("review.problems")}</h3>
    <ul class="list short">
      {#each [...report.newIssues.map((i) => [i, "integ.new"] as const), ...report.oldIssues.map((i) => [i, "integ.old"] as const)] as [i, label] (label + i.path + i.text)}
        <li><Icon name="alert" /> <span>{i.text} – {app.t(label)}</span></li>
      {/each}
    </ul>
    <button type="button" onclick={() => app.act({ action: "fixIssues" })}><Icon name="wrench" /> {app.t("integ.fixAll")}</button>
  {/if}

  {#if report.newWarnings.length || report.oldWarnings.length}
    <h3>{app.t("review.warnings")}</h3>
    <ul class="list short warn">
      {#each [...report.newWarnings.map((w) => [w, "integ.new"] as const), ...report.oldWarnings.map((w) => [w, "integ.old"] as const)] as [w, label] (label + w.code + w.place + w.value)}
        <li><Icon name="warn" /> <span>{app.t(`warn.${w.code}`)} · {w.place} = {w.value} – {app.t(label)}</span></li>
      {/each}
    </ul>
  {/if}

  <div class="dialog-actions">
    <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
    <button type="button" class="primary" onclick={save}>
      <Icon name="download" />
      {errors > 0 ? app.t("review.saveAnyway") : app.t("tb.download")}
    </button>
  </div>
</Modal>

<style>
  h3 {
    margin-top: var(--s-md);
  }
  .list {
    list-style: none;
    padding: 0;
    margin: var(--s-sm) 0;
    max-height: 300px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
  }
  .list.short {
    max-height: 160px;
  }
  .list li {
    display: flex;
    gap: 8px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    overflow-wrap: anywhere;
  }
  .list li:last-child {
    border-bottom: 0;
  }
  .list :global(.icon) {
    margin-top: 3px;
  }
  .changed :global(.icon) {
    color: var(--mark);
  }
  .added :global(.icon) {
    color: var(--success);
  }
  .removed :global(.icon) {
    color: var(--danger);
  }
  .warn li {
    color: var(--warning);
  }
</style>
