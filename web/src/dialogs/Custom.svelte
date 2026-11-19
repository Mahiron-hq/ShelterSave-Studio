<script lang="ts">
  import { untrack } from "svelte"
  import Modal from "../components/Modal.svelte"
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import Seg from "../components/Seg.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import ItemCard from "../items/ItemCard.svelte"
  import { catalog, item, journalDwellers, nameOf, rarityName, type ForeignDweller } from "../lib/core"
  import { app, type Modal as M } from "../lib/state.svelte"
  import { ui } from "../lib/ui.svelte"
  import { fmtNum } from "../lib/format"
  import { arr, DWELLERS, SPECIAL_LETTERS } from "../lib/vault"

  interface Props {
    modal: Extract<M, { kind: "custom" }>
  }
  let { modal }: Props = $props()

  let first = $state("")
  let last = $state("")
  let gender = $state<1 | 2>(1)
  let picked = $state(new Set<number>())
  let pets = $state<boolean>(untrack(() => modal.data?.pets ?? false))
  let query = $state("")
  let pick = $state<string | null>(null)
  let bonus = $state(0)

  function focusLast() {
    const list = arr(app.root, DWELLERS)
    const id = list[list.length - 1]?.serializeId
    if (typeof id === "number") {
      ui.dw.focus = id
      ui.dw.selected = new Set([id])
    }
  }

  const journalRows = $derived.by(() => {
    const q = query.trim().toLowerCase()
    const inVault = new Set(arr(app.root, DWELLERS).map((d) => d.uniqueData).filter(Boolean))
    let rows: { id: string; name: string; extra: string }[]
    if (pets) {
      rows = (catalog().pets as any[])
        .filter((p) => p.rarity === "Legendary")
        .map((p) => {
          const b = catalog().petBonuses?.[p.bonus]
          const bn = b ? nameOf(b, app.itemRu, p.bonus) : p.bonus
          const range = `${fmtNum(p.bonusMin)}–${fmtNum(p.bonusMax)}`
          return { id: p.id, name: nameOf(p.name, app.itemRu, p.id), extra: bn.includes("{0}") ? bn.replace("{0}", range) : `${bn}: ${range}` }
        })
    } else {
      rows = journalDwellers().map((u) => {
        const o = item("Outfit", u.outfit)
        let extra = o ? nameOf(o.name, app.itemRu, u.outfit) : u.outfit
        if (inVault.has(u.id)) extra += `  ·  ${app.t("dw.journalInVault")}`
        return { id: u.id, name: `${u.first} ${u.last}`.trim(), extra }
      })
    }
    return rows
      .filter((r) => !q || r.name.toLowerCase().includes(q) || r.extra.toLowerCase().includes(q))
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.extra.localeCompare(b.extra))
  })
  const unique = $derived(!pets && pick ? journalDwellers().find((u) => u.id === pick) : null)
  const pet = $derived(pets && pick ? item("Pet", pick) : null)
</script>

{#if modal.id === "newDweller"}
  <Modal title={app.t("dw.newTitle")}>
    <div class="form">
      <label>{app.t("dw.firstName")} <input type="text" bind:value={first} /></label>
      <label>{app.t("dw.lastName")} <input type="text" bind:value={last} /></label>
      <Seg
        label={app.t("dw.gender")}
        value={gender}
        options={[
          { value: 1 as const, label: app.t("dw.gender.1") },
          { value: 2 as const, label: app.t("dw.gender.2") },
        ]}
        onchange={(v) => (gender = v)}
      />
    </div>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="primary"
        onclick={() => {
          if (app.act({ action: "addDweller", first: first.trim(), last: last.trim(), gender })) {
            focusLast()
            app.pop()
          }
        }}>{app.t("c.add")}</button
      >
    </div>
  </Modal>
{:else if modal.id === "confirmDelete"}
  {@const ids = modal.data as number[]}
  <Modal title={app.t("c.confirmTitle")}>
    <p>{app.t("c.dangerDelete", { n: ids.length })}</p>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="danger"
        onclick={() => {
          app.act({ action: "deleteDwellers", ids })
          const s = new Set(ui.dw.selected)
          for (const id of ids) s.delete(id)
          ui.dw.selected = s
          if (ui.dw.focus !== null && ids.includes(ui.dw.focus)) ui.dw.focus = null
          app.pop()
        }}><Icon name="trash" /> {app.t("dw.delete")}</button
      >
    </div>
  </Modal>
{:else if modal.id === "importDwellers"}
  {@const list = modal.data.list as ForeignDweller[]}
  <Modal title={app.t("dw.importTitle", { file: modal.data.file })}>
    <p class="muted">{app.t("dw.importNote")}</p>
    <div class="row-wrap">
      <button type="button" class="small" onclick={() => (picked = new Set(list.map((_, i) => i)))}>{app.t("c.selectAll")}</button>
      <button type="button" class="small" onclick={() => (picked = new Set())}>{app.t("c.selectNone")}</button>
    </div>
    <ul class="pick-list">
      {#each list as d, i (i)}
        <li>
          <Check
            checked={picked.has(i)}
            label="{d.name}  ·  {app.t('dw.level')} {d.level}  ·  {rarityName(d.rarity, app.ru)}"
            onchange={(v) => {
              const s = new Set(picked)
              if (v) s.add(i)
              else s.delete(i)
              picked = s
            }}
          />
        </li>
      {/each}
    </ul>
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="primary"
        disabled={picked.size === 0}
        onclick={() => {
          let done = 0
          for (const i of picked) if (app.act({ action: "importDweller", json: list[i].json })) done++
          app.toast(app.t("dw.importDone", { n: done }))
          app.pop()
        }}>{app.t("dw.importGo", { n: picked.size })}</button
      >
    </div>
  </Modal>
{:else if modal.id === "fromJournal"}
  <Modal title={app.t("dw.journalTitle")}>
    <p class="muted">{app.t("dw.journalHelp")}</p>
    <Seg
      label={app.t("dw.journalTitle")}
      value={pets ? "pets" : "dwellers"}
      options={[
        { value: "dwellers", label: app.t("dw.journalDwellers") },
        { value: "pets", label: app.t("dw.journalPets") },
      ]}
      onchange={(v) => {
        pets = v === "pets"
        pick = null
      }}
    />
    <input type="search" class="search" aria-label={app.t("c.search")} placeholder={app.t("c.search")} bind:value={query} />
    <ul class="pick-list">
      {#if journalRows.length === 0}<li class="muted">{app.t("c.noResults")}</li>{/if}
      {#each journalRows as r (r.id)}
        <li>
          <button
            type="button"
            class="row-btn"
            class:on={pick === r.id}
            aria-pressed={pick === r.id}
            onclick={() => {
              pick = r.id
              const p = item("Pet", r.id)
              if (pets && p) bonus = Math.round(p.bonusMax)
            }}
          >
            <span>{r.name}</span><span class="muted small">  ·  {r.extra}</span>
          </button>
          {#if pets}<ItemCard kind="Pet" id={r.id} text="ⓘ" />{/if}
        </li>
      {/each}
    </ul>
    {#if pet}
      <p class="row-wrap">
        {app.t("dw.journalBonus")}
        <NumPick
          value={Math.min(Math.max(bonus, Math.floor(pet.bonusMin)), Math.ceil(pet.bonusMax))}
          label={app.t("dw.journalBonus")}
          lo={Math.floor(pet.bonusMin)}
          hi={Math.ceil(pet.bonusMax)}
          slider={[Math.floor(pet.bonusMin), Math.ceil(pet.bonusMax)]}
          oncommit={(v) => (bonus = v)}
        />
        <span class="muted">{app.t("dw.journalBonusRange", { min: fmtNum(Math.floor(pet.bonusMin)), max: fmtNum(Math.ceil(pet.bonusMax)) })}</span>
      </p>
    {:else if unique}
      <p class="mono">{SPECIAL_LETTERS.map((l, i) => `${l} ${unique.special[i] ?? 0}`).join("  ")}</p>
      <p class="muted small">
        {app.t("dw.journalWeapon", { name: item("Weapon", unique.weapon) ? nameOf(item("Weapon", unique.weapon).name, app.itemRu, unique.weapon) : app.t("dw.noWeapon") })}
      </p>
    {/if}
    <div class="dialog-actions">
      <button type="button" onclick={() => app.pop()}>{app.t("c.cancel")}</button>
      <button
        type="button"
        class="primary"
        disabled={!pick || (!pet && !unique)}
        onclick={() => {
          if (!pick) return
          const ok = pets ? app.act({ action: "addPet", id: pick, bonusValue: bonus }) : app.act({ action: "addUniqueDweller", id: pick })
          if (ok) {
            if (!pets) focusLast()
            app.pop()
          }
        }}>{app.t("c.add")}</button
      >
    </div>
  </Modal>
{/if}

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
  }
  .form label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pick-list {
    list-style: none;
    padding: 0;
    margin: var(--s-sm) 0;
    max-height: 320px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
  }
  .pick-list li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    border-bottom: 1px solid var(--border);
  }
  .row-btn {
    flex: 1;
    justify-content: flex-start;
    border: 0;
    background: transparent;
    text-align: start;
  }
  .row-btn.on {
    background: var(--accent-soft);
    font-weight: 600;
  }
  .search {
    width: 100%;
    margin-top: var(--s-sm);
  }
  .small {
    min-height: 26px;
    font-size: 12px;
  }
</style>
