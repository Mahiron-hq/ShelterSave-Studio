<script lang="ts">
  import CopyId from "../components/CopyId.svelte"
  import Row from "../fields/Row.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import Pick from "../components/Pick.svelte"
  import ItemCard from "../items/ItemCard.svelte"
  import ItemPicker from "../items/ItemPicker.svelte"
  import { catalog, item, nameOf, type ItemKind, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { SPECIAL_LETTERS } from "../lib/vault"

  interface Props {
    d: any
    id: number
    base: Path
  }
  let { d, id, base }: Props = $props()

  const slots: ["weapon" | "outfit" | "pet", ItemKind, string, string][] = [
    ["weapon", "Weapon", "equipedWeapon", "dw.weapon"],
    ["outfit", "Outfit", "equipedOutfit", "dw.outfit"],
    ["pet", "Pet", "equippedPet", "dw.pet"],
  ]

  function detail(kind: ItemKind, itemId: string): string {
    const info = item(kind, itemId)
    if (!info) return ""
    if (kind === "Weapon") return app.t("it.damage", { min: info.damageMin, max: info.damageMax })
    if (kind === "Outfit")
      return SPECIAL_LETTERS.map((l, i) => [l, info.special?.[i] ?? 0] as const)
        .filter(([, v]) => v !== 0)
        .map(([l, v]) => `${l}+${v}`)
        .join(" ")
    return ""
  }

  const extra = $derived(d.equippedPet?.extraData)
  const petId = $derived<string>(d.equippedPet?.id ?? "")
  const bonus = $derived<string>(extra?.bonus ?? "")
  const bonusValue = $derived<number>(typeof extra?.bonusValue === "number" ? extra.bonusValue : 0)
  const petName = $derived<string>(extra?.uniqueName ?? "")
  const petRange = $derived.by(() => {
    const p = item("Pet", petId)
    return p && p.bonus === bonus ? [Math.floor(p.bonusMin), Math.ceil(p.bonusMax)] : null
  })
  let nameDraft = $state<string | null>(null)

  function setPet(patch: { name?: string; bonus?: string; bonusValue?: number }) {
    app.act({
      action: "setPetDetails",
      id,
      name: patch.name ?? petName,
      bonus: patch.bonus ?? bonus,
      bonusValue: patch.bonusValue ?? bonusValue,
    })
  }
</script>

<section class="panel">
  <h3>{app.t("dw.equipment")}</h3>
  {#each slots as [slot, kind, key, label] (slot)}
    {@const cur = typeof d[key]?.id === "string" ? (d[key].id as string) : null}
    <Row path={[...base, key]} label={app.t(label)}>
      <div class="col">
        <div class="row-wrap">
          {#if cur}
            <ItemCard {kind} id={cur} />
            {#if app.expert}<CopyId id={cur} />{/if}
            {#if !item(kind, cur)}<span class="small warn">{app.t("c.unknownItem")}</span>{/if}
            {#if detail(kind, cur)}<span class="small">{detail(kind, cur)}</span>{/if}
          {:else}
            <span class="muted">{app.t("dw.noPet")}</span>
          {/if}
        </div>
        <div class="row-wrap">
          <ItemPicker kinds={[kind]} label={app.t(label)} gender={d.gender === 2 ? 2 : 1} onpick={(_, itemId) => app.act({ action: "equip", ids: [id], slot, item: itemId })} />
          {#if slot === "pet" && cur}
            <button type="button" onclick={() => app.act({ action: "equip", ids: [id], slot, item: null })}>{app.t("dw.noPet")}</button>
          {/if}
        </div>
      </div>
    </Row>
  {/each}
  {#if extra}
    <Row label={app.t("dw.petName")}>
      <input
        type="text"
        aria-label={app.t("dw.petName")}
        value={nameDraft ?? petName}
        oninput={(e) => (nameDraft = e.currentTarget.value)}
        onblur={() => {
          if (nameDraft !== null && nameDraft !== petName) setPet({ name: nameDraft })
          nameDraft = null
        }}
      />
    </Row>
    <Row label={app.t("dw.petBonus")}>
      <Pick
        label={app.t("dw.petBonus")}
        value={bonus}
        items={Object.entries(catalog().petBonuses ?? {}).map(([k, n]: [string, any]) => ({ value: k, label: nameOf(n, app.ru, k).replace("{0}", "…") }))}
        onchange={(v) => setPet({ bonus: v })}
      />
    </Row>
    <Row label={app.t("dw.petBonusValue")}>
      <NumPick
        value={bonusValue}
        label={app.t("dw.petBonusValue")}
        lo={app.expert ? undefined : Math.min(petRange?.[0] ?? 0, bonusValue)}
        hi={app.expert ? undefined : Math.max(petRange?.[1] ?? 1000, bonusValue)}
        oncommit={(v) => setPet({ bonusValue: v })}
      />
    </Row>
  {/if}
</section>

<style>
  .col {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .warn {
    color: var(--warning);
  }
</style>
