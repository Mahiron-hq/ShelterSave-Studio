<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Fold from "../components/Fold.svelte"
  import ObjectForm from "../fields/ObjectForm.svelte"
  import TextField from "../fields/TextField.svelte"
  import NumField from "../fields/NumField.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import { catalog, core, item, nameOf } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { ACTORS, arr, DWELLERS, dwellerName, ROOMS } from "../lib/vault"

  const actors = $derived(arr(app.root, ACTORS))
  const max = $derived(catalog().limits.mrHandyHealth ?? 5000)

  function title(a: any): string {
    if (a.characterType === 3) {
      const pet = item("Pet", a.actorDataId ?? "")
      const breed = pet ? nameOf(pet.name, app.itemRu, a.actorDataId) : (a.actorDataId ?? "")
      const name: string = a.name ?? ""
      const plain = pet && (pet.name.en === name || pet.name.ru === name)
      let t = name && name !== breed && !plain ? `${breed} «${name}»` : breed
      const owner = arr(app.root, DWELLERS).find((d) => d.serializeId === a.FollowedID)
      if (owner) t += `  ·  ${app.t("dw.follows", { name: dwellerName(owner) })}`
      return t
    }
    const room = arr(app.root, ROOMS).find((r) => r.deserializeID === a.savedRoom)
    return room ? `${app.t("dw.mrHandy")}  ·  ${core().kindName(room.type, room.level ?? 1)}` : app.t("dw.mrHandy")
  }

  function addHandy() {
    const counts = [0, 0, 0, 0]
    for (const t of app.root?.vault?.LunchBoxesByType ?? []) if (typeof t === "number" && t >= 0 && t < 4) counts[t]++
    counts[1]++
    app.act({ action: "setLunchboxes", counts })
  }
</script>

<section class="panel">
  <h3>{app.t("dw.pets")}</h3>
  <p class="muted small">{app.t("dw.petsHelp")}</p>
  <button type="button" title={app.t("dw.petFromJournalHelp")} onclick={() => app.push({ kind: "custom", id: "fromJournal", data: { pets: true } })}>
    <Icon name="paw" /> {app.t("dw.petFromJournal")}
  </button>
  {#if actors.length === 0}<p class="muted">{app.t("c.empty")}</p>{/if}
  {#each actors as a, i (i)}
    {@const at = (k: string) => [...ACTORS, i, k]}
    {#if app.expert}
      <Fold title={title(a)}><ObjectForm path={[...ACTORS, i]} depth={1} /></Fold>
    {:else if a.characterType === 3}
      <p class="pet"><Icon name="paw" /> {title(a)}</p>
    {:else}
      <Fold title={title(a)}>
        {#if a.name !== undefined}<TextField path={at("name")} label={app.t("dw.handyName")} />{/if}
        {#if a.death === true}
          <p class="row-wrap">
            <span class="danger">{app.t("dw.handyBroken")}</span>
            <button
              type="button"
              onclick={() => {
                app.act({ action: "setBool", path: at("death"), value: false })
                app.act({ action: "setNumber", path: at("health"), value: max })
              }}><Icon name="sparkles" /> {app.t("dw.handyRepair")}</button
            >
          </p>
        {/if}
        {#if a.health !== undefined}
          <NumField path={at("health")} label={app.t("dw.handyHealth")} help={app.t("dw.handyHealthHelp")} min={0} max={max} unit={app.t("c.units.hp")} />
        {/if}
        {#if a.canCollect !== undefined}
          <BoolField path={at("canCollect")} label={app.t("dw.handyCollect")} help={app.t("dw.handyCollectHelp")} />
        {/if}
      </Fold>
    {/if}
  {/each}
  <button type="button" class="add" title={app.t("dw.handyAddHelp")} onclick={addHandy}><Icon name="wrench" /> {app.t("dw.handyAdd")}</button>
</section>

{#if arr(app.root, ["dwellerSpawner", "dwellersWaiting"]).length || app.expert}
  <section class="panel">
    <h3>{app.t("dw.waiting")}</h3>
    <p class="muted small">{app.t("dw.waitingHelp")}</p>
    {#each arr(app.root, ["dwellerSpawner", "dwellersWaiting"]) as w, i (i)}
      {@const d = arr(app.root, DWELLERS).find((x) => x.serializeId === w?.dwellerId)}
      <p class="pet"><Icon name="userPlus" /> {w?.charType === "MrHandy" ? app.t("dw.mrHandy") : d ? dwellerName(d) : app.t("dw.waitingNew")}</p>
    {/each}
    {#if app.expert}
      <Fold title={app.t("c.raw")}><ObjectForm path={["dwellerSpawner", "dwellersWaiting"]} /></Fold>
    {/if}
  </section>
{/if}

<style>
  .pet {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 4px 0;
  }
  .danger {
    color: var(--danger);
  }
  .add {
    margin-top: var(--s-sm);
  }
</style>
