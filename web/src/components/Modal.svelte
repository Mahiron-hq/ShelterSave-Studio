<script lang="ts">
  import type { Snippet } from "svelte"
  import { Dialog, DialogBackdrop, DialogContent, DialogTitle } from "@ui-sdk/svelte/dialog"
  import { app } from "../lib/state.svelte"

  interface Props {
    title: string
    wide?: boolean
    onclose?: () => void
    children: Snippet
  }
  let { title, wide = false, onclose, children }: Props = $props()

  function close() {
    if (onclose) onclose()
    else app.pop()
  }
</script>

<Dialog
  open={true}
  onOpenChange={(open) => {
    if (!open) close()
  }}
>
  <DialogBackdrop />
  <DialogContent class={wide ? "wide" : ""}>
    <DialogTitle>{title}</DialogTitle>
    {@render children()}
  </DialogContent>
</Dialog>

<style>
  :global([data-slot="dialog-content"].wide) {
    width: min(960px, calc(100vw - 32px));
  }
</style>
