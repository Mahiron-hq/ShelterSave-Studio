<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Pick from "../components/Pick.svelte"
  import ItemPicker from "../items/ItemPicker.svelte"
  import { app } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { arr, DWELLERS, isDead } from "../lib/vault"
  import type { IconName } from "../lib/icons"
  import type { ItemKind } from "../lib/core"

  const ids = $derived([...ui.dw.selected])
  const n = $derived(ids.length)
  const anyDead = $derived(arr(app.root, DWELLERS).some((d) => ui.dw.selected.has(d.serializeId) && isDead(d)))
  let slot = $state<"weapon" | "outfit" | "pet">("weapon")
  let field = $state("")
  let value = $state("")
  const kindOf: Record<string, ItemKind> = { weapon: "Weapon", outfit: "Outfit", pet: "Pet" }

  const actions = $derived<[IconName, string, any][]>([
    ["dna", "dw.mass.special", { action: "maxSpecial", ids }],
    ["star", "dw.mass.level", { action: "setLevel", ids, level: 50 }],
    ["heartPulse", "dw.mass.health", { action: "fullHealth", ids }],
    ["radiation", "dw.mass.radiation", { action: "clearRadiation", ids }],
    ["smile", "dw.mass.happiness", { action: "maxHappiness", ids }],
    ...(anyDead ? [["sparkles", "dw.mass.revive", { action: "revive", ids }] as [IconName, string, any]] : []),
  ])

  function applyField() {
    const rel = field
      .split(".")
      .filter(Boolean)
      .map((s) => (/^\d+$/.test(s) ? Number(s) : s))
    if (rel.length) app.actMass({ action: "setDwellerField", ids, rel, json: value.trim() }, n)
  }
</script>

<section class="panel">
  <h3>{app.t("dw.mass")} – {app.t("c.selected", { n })}</h3>
  <div class="row-wrap">
    {#each actions as [icon, key, action] (key)}
      <button type="button" onclick={() => app.actMass(action, n)}><Icon name={icon} /> {app.t(key)}</button>
    {/each}
    <button type="button" class="danger" onclick={() => app.push({ kind: "custom", id: "confirmDelete", data: ids })}>
      <Icon name="trash" /> {app.t("dw.delete")}
    </button>
  </div>
  <div class="row-wrap line">
    <span>{app.t("dw.mass.equip")}</span>
    <Pick
      label={app.t("dw.mass.equip")}
      value={slot}
      items={(["weapon", "outfit", "pet"] as const).map((s) => ({ value: s, label: app.t(`it.kind.${kindOf[s]}`) }))}
      onchange={(v) => (slot = v)}
    />
    <ItemPicker kinds={[kindOf[slot]]} label={app.t("dw.mass.equip")} onpick={(_, id) => app.actMass({ action: "equip", ids, slot, item: id }, n)} />
  </div>
  {#if app.expert}
    <div class="row-wrap line">
      <span title={app.t("dw.mass.fieldHelp")}>{app.t("dw.mass.field")}</span>
      <input class="mono" aria-label={app.t("dw.mass.field")} placeholder="health.radiationValue" bind:value={field} />
      <span>=</span>
      <input class="mono short" aria-label={app.t("c.edit")} placeholder="0" bind:value />
      <button type="button" onclick={applyField}>{app.t("c.apply")}</button>
    </div>
  {/if}
</section>

<style>
  .line {
    margin-top: var(--s-sm);
  }
  .short {
    width: 12ch;
  }
</style>
