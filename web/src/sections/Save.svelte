<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import TextField from "../fields/TextField.svelte"
  import NumField from "../fields/NumField.svelte"
  import ChoiceField from "../fields/ChoiceField.svelte"
  import TicksField from "../fields/TicksField.svelte"
  import Row from "../fields/Row.svelte"
  import { core } from "../lib/core"
  import { arr, DWELLERS, isDead } from "../lib/vault"
  import { app } from "../lib/state.svelte"
  import { download, pickFile } from "../lib/files"
  import { fmtNum, short } from "../lib/format"
  import { goTo } from "../lib/goto"

  const root = $derived(app.root)

  const summary = $derived.by(() => {
    const dw = arr(root, DWELLERS)
    const alive = dw.filter((d) => !isDead(d)).length
    const actors: any[] = root?.dwellers?.actors ?? []
    const pets = actors.filter((a) => a.characterType === 3).length
    const res = root?.vault?.storage?.resources ?? {}
    const rows: [string, string][] = [
      [app.t("sum.dwellers"), `${dw.length} (${app.t("sum.alive", { n: alive })})`],
      [app.t("sum.rooms"), String((root?.vault?.rooms ?? []).length)],
      [app.t("sum.items"), String((root?.vault?.inventory?.items ?? []).length)],
      [app.t("sum.pets"), String(pets)],
      [app.t("sum.robots"), String(actors.length - pets)],
      [app.t("sum.version"), String(root?.appVersion ?? "").trim()],
      [app.t("sum.format"), core().fileName().toLowerCase().endsWith(".json") ? "JSON" : ".sav"],
      [app.t("sum.size"), `${Math.floor(app.size / 1024)} KB`],
    ]
    if (app.expert) rows.push([app.t("sum.rootKeys"), String(Object.keys(root ?? {}).length)])
    for (const k of ["Nuka", "Food", "Energy", "Water", "StimPack", "RadAway", "NukaColaQuantum"]) {
      if (typeof res[k] === "number") rows.push([app.t(`res.${k}`), fmtNum(Math.floor(res[k]))])
    }
    return rows
  })

  const issues = $derived.by(() => {
    void app.rev
    return JSON.parse(core().issues()) as { new: any[]; old: any[] }
  })

  function exportJson() {
    const name = core().fileName().replace(/\.[^.]*$/, "") || "vault"
    download(`${name}.json`, core().toJson(), "application/json")
  }

  async function importJson() {
    const f = await pickFile(".json,application/json")
    if (!f) return
    let text: string
    try {
      text = await f.text()
    } catch (e) {
      app.toastError(app.t("err.io", { detail: String(e) }))
      return
    }
    app.act({ action: "setRawRoot", json: text })
  }
</script>

<h1 class="section-title"><Icon name="shield" /> {app.t("nav.save")}</h1>

<section class="panel">
  <h3>{app.t("save.general")}</h3>
  <TextField path={["vault", "VaultName"]} />
  <ChoiceField
    path={["vault", "VaultMode"]}
    choices={[
      { value: "Normal", label: app.t("vm.Normal") },
      { value: "Survival", label: app.t("vm.Survival") },
    ]}
  />
  {#each ["deviceName", "appVersion", "seasonId", "versionCount"] as key (key)}
    {#if root?.[key] !== undefined}
      {#if app.expert || key === "deviceName"}
        {#if typeof root[key] === "number"}
          <NumField path={[key]} />
        {:else}
          <TextField path={[key]} />
        {/if}
      {:else}
        <Row path={[key]}><span>{short(root[key]).trim()}</span></Row>
      {/if}
    {/if}
  {/each}
</section>

<section class="panel">
  <h3>{app.t("save.time")}</h3>
  <p class="muted small">{app.t("time.local")}</p>
  <TicksField path={["timeMgr", "timeGameBegin"]} label={app.t("time.begin")} />
  <TicksField path={["timeMgr", "timeSaveDate"]} label={app.t("time.saved")} />
</section>

<section class="panel">
  <h3>{app.t("save.summary")}</h3>
  <table class="grid">
    <tbody>
      {#each summary as [k, v] (k)}
        <tr><th scope="row">{k}</th><td class="mono">{v}</td></tr>
      {/each}
    </tbody>
  </table>
</section>

<section class="panel">
  <h3>{app.t("save.integrity")}</h3>
  {#if !issues.new.length && !issues.old.length}
    <p class="ok"><Icon name="check" /> {app.t("integ.ok")}</p>
  {:else}
    {#if [...issues.new, ...issues.old].some((i) => i.fixable)}
      <button type="button" onclick={() => app.act({ action: "fixIssues" })}><Icon name="wrench" /> {app.t("integ.fixAll")}</button>
    {/if}
    {#each [[app.t("integ.new"), issues.new], [app.t("integ.old"), issues.old]] as [title, list] (title)}
      {#if list.length}
        <h4>{title}: {list.length}</h4>
        <ul class="issues">
          {#each list.slice(0, 200) as i (i.path + i.code)}
            <li class={i.severity}>
              <Icon name={i.severity === "error" ? "alert" : "warn"} />
              <span>{i.text}</span>
              {#if !i.fixable}<span class="muted small">{app.t("integ.manual")}</span>{/if}
              <button type="button" class="small-btn" onclick={() => goTo(i.segs)}>{app.t("integ.goTo")}</button>
            </li>
          {/each}
        </ul>
      {/if}
    {/each}
  {/if}
</section>

<section class="panel">
  <h3>{app.t("save.json")}</h3>
  <div class="row-wrap">
    <button type="button" onclick={exportJson}><Icon name="download" /> {app.t("save.export")}</button>
    <button type="button" title={app.t("save.importHelp")} onclick={importJson}><Icon name="upload" /> {app.t("save.import")}</button>
  </div>
  <p class="muted small">{app.t("save.exportHelp")}</p>
</section>

<style>
  .ok {
    color: var(--success);
    display: flex;
    gap: 6px;
    align-items: center;
  }
  h4 {
    margin: var(--s-md) 0 var(--s-xs);
  }
  .issues {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .issues li {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 4px 0;
  }
  .issues .error :global(.icon) {
    color: var(--danger);
  }
  .issues .warning :global(.icon) {
    color: var(--warning);
  }
  .small-btn {
    min-height: 24px;
    padding: 2px 8px;
    font-size: 12px;
  }
</style>
