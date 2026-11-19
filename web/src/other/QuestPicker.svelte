<script lang="ts">
  import { Combobox, ComboboxContent, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList } from "@ui-sdk/svelte/combobox"
  import { catalog, core } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props {
    prefix: string
    current: string
    onpick: (id: string) => void
  }
  let { prefix, current, onpick }: Props = $props()
  let query = $state("")

  const list = $derived.by(() => {
    const s = query.trim().toLowerCase()
    return (catalog().quests as any[])
      .filter((x) => x.id.startsWith(prefix))
      .map((x) => {
        let name = core().questLine(x.id)
        // Ids like Daily_04_Diff_10_B differ only by the last letter.
        const v = x.id.split("_").at(-1)
        if (v && /^[A-Z]$/.test(v)) name += `  ·  ${app.t("o.questVariant", { v })}`
        return { value: x.id as string, label: name }
      })
      .filter((x) => x.value !== current)
      .filter((x) => !s || x.label.toLowerCase().includes(s) || x.value.toLowerCase().includes(s))
  })
</script>

<Combobox
  filter={false}
  bind:query
  value={null}
  onValueChange={(picked) => {
    const v = picked as unknown
    if (typeof v === "string" && v !== current) {
      query = ""
      onpick(v)
    }
  }}
>
  <ComboboxInput aria-label={app.t("o.replaceQuest")} placeholder={app.t("o.replaceQuest")} title={app.t("o.replaceQuestHelp")} class="picker-input" />
  <ComboboxContent>
    <ComboboxEmpty>{app.t("c.nothingFound")}</ComboboxEmpty>
    <ComboboxList>
      {#each list as x (x.value)}
        <ComboboxItem value={x.value}>{x.label}</ComboboxItem>
      {/each}
    </ComboboxList>
  </ComboboxContent>
</Combobox>
