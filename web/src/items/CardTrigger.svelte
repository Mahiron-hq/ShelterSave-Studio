<script lang="ts">
  import { hoverCard, type CardSubject } from "../lib/hovercard.svelte"

  interface Props {
    subject: CardSubject
    text: string
  }
  let { subject, text }: Props = $props()
  let el: HTMLButtonElement

  function pointer(e: PointerEvent) {
    if (e.pointerType !== "touch") hoverCard.enter(subject, el)
  }
  function click(e: MouseEvent) {
    // Touch screens have no hover: a tap opens and closes the card.
    if ((e as PointerEvent).pointerType === "touch" || e.detail === 0) hoverCard.toggle(subject, el)
  }
</script>

<button
  bind:this={el}
  type="button"
  class="item-trigger"
  aria-haspopup="dialog"
  onpointerenter={pointer}
  onpointerleave={() => hoverCard.leave()}
  onfocus={() => hoverCard.enter(subject, el)}
  onblur={() => hoverCard.leave()}
  onclick={click}
  onkeydown={(e) => e.key === "Escape" && hoverCard.hide()}>{text}</button
>

<style>
  .item-trigger {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    font-weight: 600;
    text-align: start;
    cursor: help;
    color: var(--text);
  }
  .item-trigger:hover {
    text-decoration: underline dotted;
    background: transparent;
  }
</style>
