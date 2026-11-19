<script lang="ts">
  import Pick from "../components/Pick.svelte"
  import ThemeInfo from "./ThemeInfo.svelte"
  import RoomPicture from "./RoomPicture.svelte"
  import { catalog, consts, nameOf, query, themePicture } from "../lib/core"
  import { app } from "../lib/state.svelte"

  interface Props {
    room: string
  }
  let { room }: Props = $props()

  const list = $derived((catalog().themes as any[]).filter((t) => t.room === room))
  const cur = $derived<string | null>(app.root?.specialTheme?.themeByRoomType?.[room] ?? null)
  const curId = $derived(cur ? (list.find((t) => t.theme === cur)?.id ?? null) : null)
  const items = $derived.by(() => {
    void app.rev
    const out = [{ value: "", label: app.t("theme.standard") }]
    for (const t of list) {
      const crafted = query<[number, boolean]>("themeState", { id: t.id })?.[1]
      const name = nameOf(t.name, app.ru, t.id)
      out.push({ value: t.id, label: crafted ? name : `${name}  ·  ${app.t("theme.willCraft")}` })
    }
    if (cur && !curId) {
      const h = consts().holidays.find(([k]) => k === cur)
      out.push({ value: `@${cur}`, label: h ? app.t(`theme.holiday.${h[0]}`) : cur })
    }
    return out
  })
  const value = $derived(cur === null ? "" : (curId ?? `@${cur}`))
</script>

<span class="pick" title={app.t("theme.pickHelp")}>
<Pick
  label={app.t("room.theme")}
  {preview}
  {value}
  {items}
  onchange={(v) => {
    if (v.startsWith("@") || v === value) return
    app.act({ action: "setRoomTheme", room, theme: v || null })
  }}
/>
</span>
{#if cur && !curId}<p class="small warn">{app.t("theme.holidaySet")}</p>{/if}

{#snippet preview(v: string)}
  {#if v && !v.startsWith("@")}<ThemeInfo id={v} />{:else}
    {@const pic = themePicture(room, null)}
    <p class="title">{app.t("theme.standard")}</p>
    <RoomPicture kind={pic[0]} id={pic[1]} maxW={300} noteMissing />
  {/if}
{/snippet}

<style>
  .pick {
    display: inline-block;
  }
  .warn {
    color: var(--warning);
    margin: 4px 0;
  }
  .title {
    font-weight: 600;
    margin: 0 0 4px;
  }
</style>
