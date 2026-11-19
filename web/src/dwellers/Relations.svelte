<script lang="ts">
  import Row from "../fields/Row.svelte"
  import NumField from "../fields/NumField.svelte"
  import ObjectForm from "../fields/ObjectForm.svelte"
  import Pick from "../components/Pick.svelte"
  import Fold from "../components/Fold.svelte"
  import { describe, type Path } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import { arr, DWELLERS, dwellerName } from "../lib/vault"

  interface Props {
    d: any
    id: number
    base: Path
  }
  let { d, id, base }: Props = $props()
  const all = $derived(arr(app.root, DWELLERS))
  const nameOf = (pid: number) => {
    const x = all.find((y) => y.serializeId === pid)
    return x ? dwellerName(x) : `#${pid}`
  }
  const partner = $derived<number>(d.relations?.partner ?? -1)
  const last = $derived<number>(d.relations?.lastPartner ?? -1)
  const asc = $derived<number[]>((d.relations?.ascendants ?? []).filter((x: unknown) => typeof x === "number" && x >= 0))
  const ascPath = $derived<Path>([...base, "relations", "ascendants"])
</script>

<section class="panel">
  <h3>{app.t("dw.relations")}</h3>
  <Row path={[...base, "relations", "partner"]} label={app.t("dw.partner")} choice>
    <Pick
      label={app.t("dw.partner")}
      value={String(partner)}
      items={[
        { value: "-1", label: app.t("c.none") },
        ...all.filter((o) => typeof o.serializeId === "number" && o.serializeId !== id).map((o) => ({ value: String(o.serializeId), label: dwellerName(o) })),
      ]}
      onchange={(v) => Number(v) !== partner && app.act({ action: "setPartner", id, partner: Number(v) })}
    />
  </Row>
  <Row path={[...base, "relations", "lastPartner"]} label={app.t("dw.lastPartner")}>
    <span>{last === -1 ? app.t("c.none") : nameOf(last)}</span>
  </Row>
  {#if app.expert}
    {#if d.lastChildBorn !== undefined}<NumField path={[...base, "lastChildBorn"]} />{/if}
    <Fold title={describe(app.lang, ascPath).label}>
      <ObjectForm path={ascPath} />
    </Fold>
  {:else}
    <Row path={ascPath}>
      <span>{asc.length ? asc.map(nameOf).join(", ") : app.t("dw.noAscendants")}</span>
    </Row>
  {/if}
</section>
