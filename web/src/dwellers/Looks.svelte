<script lang="ts">
  import Row from "../fields/Row.svelte"
  import Pick from "../components/Pick.svelte"
  import Swatches from "../components/Swatches.svelte"
  import ColorField from "./ColorField.svelte"
  import Preview from "./Preview.svelte"
  import { catalog, core, query, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props {
    d: any
    id: number
    index: number
    base: Path
  }
  let { d, id, index, base }: Props = $props()

  const gender = $derived<number>(d.gender === 2 ? 2 : 1)
  const looks = $derived(catalog().looks?.[gender === 2 ? "male" : "female"] ?? {})
  const outfit = $derived(
    (void app.rev,
    query<{ worn: string | null; known: boolean; fits: boolean; colors: number[]; used: number } | null>("outfitLook", { index })),
  )

  const parts = $derived(
    (["hair", "faceMask", "helmet"] as const).map((field) => {
      const none = field === "hair" ? "dw.bald" : "c.none"
      const label = field === "hair" ? "dw.hairStyle" : field === "faceMask" ? "dw.faceMask" : "dw.helmet"
      const cur: string = typeof d[field] === "string" ? d[field] : ""
      const opts: { value: string; label: string }[] = [{ value: "", label: app.t(none) }, ...JSON.parse(core().lookOptions(gender, field))]
      if (cur && !opts.some((o) => o.value === cur)) opts.push({ value: cur, label: cur })
      return { field, label, cur, opts }
    }),
  )
</script>

<section class="panel">
  <h3>{app.t("dw.looks")}</h3>
  <div class="looks">
    <Preview {d} />
    <div class="controls">
      {#each [["skinColor", "dw.skin", "skinColors"], ["hairColor", "dw.hair", "hairColors"]] as [field, label, presets] (field)}
        {#if typeof d[field] === "number"}
          <ColorField path={[...base, field]} label={app.t(label)} value={(d[field] >>> 0) & 0xffffff} presets={looks[presets] ?? []} onset={(rgb) => app.act({ action: "setColor", ids: [id], field, rgb })} />
        {/if}
      {/each}
      {#if typeof d.outfitColor === "number"}
        {#if outfit && (outfit.known || !outfit.fits)}
          <Row path={[...base, "outfitColor"]} label={app.t("dw.outfitColor")} help={app.t("dw.outfitColorHelp")}>
            {#if !outfit.fits && !outfit.known}
              <span class="muted">{app.t("dw.outfitColorWrong")}</span>
            {:else if outfit.worn === "__pregnant"}
              <span class="muted">{app.t("dw.outfitColorPregnant")}</span>
            {:else if outfit.colors.length === 0}
              <span class="muted">{app.t("dw.outfitColorFixed")}</span>
            {:else}
              <Swatches
                label={app.t("dw.outfitColor")}
                value={(outfit.used >>> 0) & 0xffffff}
                colors={outfit.colors}
                onchange={(rgb) => app.act({ action: "setColor", ids: [id], field: "outfitColor", rgb })}
              />
            {/if}
          </Row>
        {:else}
          <ColorField
            path={[...base, "outfitColor"]}
            label={app.t("dw.outfitColor")}
            value={(d.outfitColor >>> 0) & 0xffffff}
            presets={[]}
            onset={(rgb) => app.act({ action: "setColor", ids: [id], field: "outfitColor", rgb })}
          />
        {/if}
      {/if}
      {#each parts as p (p.field)}
        <Row path={[...base, p.field]} label={app.t(p.label)} help={app.t(`${p.label}Help`)} choice>
          <Pick label={app.t(p.label)} value={p.cur} items={p.opts} onchange={(v) => v !== p.cur && app.act({ action: "setLook", id, field: p.field, value: v })} />
        </Row>
      {/each}
    </div>
  </div>
</section>

<style>
  .looks {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--s-md);
    align-items: start;
  }
  .controls {
    min-width: 0;
  }
  @media (max-width: 700px) {
    .looks {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
