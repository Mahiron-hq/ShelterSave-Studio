<script lang="ts">
  import Icon from "../../components/Icon.svelte"
  import NumField from "../../fields/NumField.svelte"
  import Row from "../../fields/Row.svelte"
  import NumPick from "../../fields/NumPick.svelte"
  import { at, consts } from "../../lib/core"
  import { app } from "../../lib/state.svelte"

  const c = consts()
  let all = $state(0)

  const groups = $derived(
    [
      { bonus: false, title: "vault.resources", help: "vault.resourcesHelp", base: ["vault", "storage", "resources"] },
      { bonus: true, title: "vault.bonus", help: "vault.bonusHelp", base: ["vault", "storage", "bonus"] },
    ].filter((g) => (!g.bonus || app.expert) && at(app.root, g.base) !== undefined),
  )

  const lunch = $derived.by(() => {
    const types: unknown[] = at(app.root, ["vault", "LunchBoxesByType"]) ?? []
    const counts = [0, 0, 0, 0]
    let other = 0
    for (const t of types) {
      if (typeof t === "number" && t >= 0 && t <= 3) counts[t]++
      else other++
    }
    return { counts, other }
  })

  function setLunch(i: number, v: number) {
    const next = [...lunch.counts]
    next[i] = Math.max(0, Math.round(v))
    if (next[i] !== lunch.counts[i]) app.act({ action: "setLunchboxes", counts: next })
  }
</script>

{#each groups as g (g.title)}
  <section class="panel">
    <h3>{app.t(g.title)}</h3>
    <p class="muted small">{app.t(g.help)}</p>
    <div class="row-wrap bar">
      <button type="button" title={app.t("vault.fillHelp")} onclick={() => app.act({ action: "fillResources", bonus: g.bonus })}>
        <Icon name="sparkles" /> {app.t("vault.fill")}
      </button>
      <span>{app.t("vault.setAll")}</span>
      <NumPick value={all} label={app.t("vault.setAll")} lo={0} hi={c.resourceMax} oncommit={(v) => (all = v)} />
      <button type="button" onclick={() => app.act({ action: "setAllResources", bonus: g.bonus, value: all })}>{app.t("c.apply")}</button>
    </div>
    {#each c.resourceKeys as key (key)}
      {#if (app.expert || c.playerResources.includes(key)) && at(app.root, [...g.base, key]) !== undefined}
        <NumField
          path={[...g.base, key]}
          label={app.t(`res.${key}`)}
          help={["Lunchbox", "MrHandy", "PetCarrier"].includes(key) ? app.t("res.counterHelp") : undefined}
          min={0}
          max={c.resourceMax}
        />
      {/if}
    {/each}
  </section>
{/each}

<section class="panel">
  <h3>{app.t("vault.lunchboxes")}</h3>
  <p class="muted small">{app.t("vault.lunchboxesHelp")}</p>
  {#each lunch.counts as n, i (i)}
    <Row label={app.t(`vault.lb.${i}`)} help={app.t(`vault.lbHelp.${i}`)}>
      <NumPick
        value={n}
        label={app.t(`vault.lb.${i}`)}
        lo={0}
        hi={Math.max(c.lunchboxMax, n)}
        slider={[0, Math.max(c.lunchboxMax, n)]}
        oncommit={(v) => setLunch(i, v)}
      />
    </Row>
  {/each}
  {#if lunch.other > 0}
    <p class="small">{app.t("vault.lb.other", { n: lunch.other })}</p>
  {/if}
</section>

<style>
  .bar {
    margin: var(--s-sm) 0;
  }
</style>
