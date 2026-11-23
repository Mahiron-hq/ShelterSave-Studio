<script lang="ts">
  import { hoverCard } from "../lib/hovercard.svelte"
  import ItemInfo from "./ItemInfo.svelte"
  import ThemeInfo from "../rooms/ThemeInfo.svelte"

  let card = $state<HTMLDivElement>()
  let pos = $state({ left: 0, top: 0 })
  const GAP = 12

  // Beside the name, never over it: right of the list when there is room, else left.
  $effect(() => {
    const a = hoverCard.anchor
    if (!a || !card) return
    const w = card.offsetWidth
    const h = card.offsetHeight
    const vw = document.documentElement.clientWidth
    const vh = document.documentElement.clientHeight
    let left = a.right + GAP
    if (left + w > vw - 8) left = a.left - GAP - w
    if (left < 8) {
      left = Math.max(8, Math.min(a.left, vw - w - 8))
      const below = a.bottom + GAP
      pos = { left, top: below + h <= vh - 8 ? below : Math.max(8, a.top - GAP - h) }
      return
    }
    pos = { left, top: Math.max(8, Math.min(a.top - 8, vh - h - 8)) }
  })
</script>

<svelte:window onscroll={() => hoverCard.hide()} onresize={() => hoverCard.hide()} />

{#if hoverCard.subject}
  {@const s = hoverCard.subject}
  <div class="card hover-card" role="tooltip" bind:this={card} style:left="{pos.left}px" style:top="{pos.top}px">
    {#if "theme" in s}
      <ThemeInfo id={s.theme} />
    {:else}
      <ItemInfo kind={s.kind} id={s.id} />
    {/if}
  </div>
{/if}

<style>
  .hover-card {
    position: fixed;
    z-index: 1000;
    width: max-content;
    max-width: 340px;
    pointer-events: none;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 4px;
  }
</style>
