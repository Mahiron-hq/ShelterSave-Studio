<script lang="ts">
  import ChoiceField from "../fields/ChoiceField.svelte"
  import Row from "../fields/Row.svelte"
  import { catalog, rarityName, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import Swatches from "../components/Swatches.svelte"

  interface Props {
    room: any
    base: Path
  }
  let { room, base }: Props = $props()
  const partners = $derived(((room.partners ?? []) as any[]).map((pr, k) => ({ pr, k })).filter(({ pr }) => pr?.s === "MakingBaby" || pr?.s === "RaisingBaby"))
  const female = $derived(catalog().looks?.female ?? {})
</script>

{#each partners as { pr, k } (k)}
  {@const at = (key: string): Path => [...base, "partners", k, key]}
  <h4 title={app.t("room.babyHelp")}>{app.t("room.baby")}</h4>
  <p class="muted small">{`${pr.maleName ?? ""} ${pr.maleLast ?? ""} + ${pr.femaleName ?? ""} ${pr.femaleLast ?? ""}`.trim()}</p>
  {#if pr.childRarity !== undefined}
    <ChoiceField
      path={at("childRarity")}
      label={app.t("room.babyRarity")}
      help={app.t("room.babyRarityHelp")}
      int
      choices={[
        { value: "1", label: rarityName("Normal", app.ru) },
        { value: "2", label: rarityName("Rare", app.ru) },
        { value: "3", label: rarityName("Legendary", app.ru) },
      ]}
    />
  {/if}
  {#if Array.isArray(pr.childStat) && pr.childStat.length}
    <ChoiceField
      path={[...at("childStat"), 0]}
      label={app.t("room.babyStat")}
      help={app.t("room.babyStatHelp")}
      int
      choices={[1, 2, 3, 4, 5, 6, 7].map((s) => ({ value: String(s), label: app.t(`dw.specialName.${s}`) }))}
    />
  {/if}
  {#if pr.pendingChildren !== undefined}
    <ChoiceField
      path={at("pendingChildren")}
      label={app.t("room.babyCount")}
      help={app.t("room.babyCountHelp")}
      int
      choices={[
        { value: "0", label: app.t("room.babyCountGame") },
        { value: "1", label: app.t("room.babyCount1") },
        { value: "2", label: app.t("room.babyCount2") },
        { value: "3", label: app.t("room.babyCount3") },
      ]}
    />
  {/if}
  {#each [["childColor", "room.babySkin", "skinColors"], ["childHairColor", "room.babyHair", "hairColors"]] as [key, label, list] (key)}
    {#if typeof pr[key] === "number"}
      <Row path={at(key)} label={app.t(label)}>
        <Swatches
          label={app.t(label)}
          value={(pr[key] >>> 0) & 0xffffff}
          colors={(female[list] ?? []) as number[]}
          onchange={(rgb) => app.act({ action: "setInt", path: at(key), value: (rgb | 0xff000000) >>> 0 })}
        />
      </Row>
    {/if}
  {/each}
{/each}

<style>
  h4 {
    margin: var(--s-sm) 0 0;
  }
  p {
    margin: 0;
  }
</style>
