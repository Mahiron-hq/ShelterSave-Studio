<script lang="ts">
  import ArtImage from "../art/ArtImage.svelte"
  import RoomPicture from "./RoomPicture.svelte"
  import Price from "../items/Price.svelte"
  import { catalog, core, nameOf, query, themePicture } from "../lib/core"
  import { app } from "../lib/state.svelte"
  import type { Snippet } from "svelte"

  interface Props {
    id: string
    title?: Snippet<[string]>
  }
  let { id, title }: Props = $props()
  const t = $derived((catalog().themes as any[]).find((x) => x.id === id))
  const name = $derived(t ? nameOf(t.name, app.ru, id) : id)
  const state = $derived.by(() => {
    void app.rev
    return query<[number, boolean]>("themeState", { id })
  })
  const pic = $derived(t ? themePicture(t.room, t.theme) : null)
</script>

<div class="head">
  <ArtImage id="Theme:{id}" max={120} />
  <div>
    {#if title}{@render title(name)}{:else}<p class="title">{name}</p>{/if}
    {#if t}
      <p class="muted small">
        {app.t("card.themeFor", { room: t.room === "FakeWasteland" ? app.t("theme.exterior") : core().kindName(t.room, 1) })}
      </p>
    {/if}
    {#if state}
      <p>{state[1] ? app.t("card.themeCrafted") : app.t("card.themeParts", { n: state[0] })}</p>
    {/if}
  </div>
</div>
{#if pic}<RoomPicture kind={pic[0]} id={pic[1]} maxW={300} />{/if}
<Price craft={t?.craft} theme />
{#if app.expert}<p class="mono muted small">{id}</p>{/if}

<style>
  .head {
    display: flex;
    gap: var(--s-sm);
    align-items: flex-start;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }
  p {
    margin: 2px 0;
  }
</style>
