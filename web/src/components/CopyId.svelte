<script lang="ts">
  import { app } from "../lib/state.svelte"

  interface Props {
    id: string
  }
  let { id }: Props = $props()

  async function copy() {
    try {
      await navigator.clipboard.writeText(id)
      app.toast(app.t("c.copied"))
    } catch {
      app.toastError(id)
    }
  }
</script>

<button type="button" class="copy-id mono small muted" title={app.t("c.copy")} onclick={copy}>{id}</button>

<style>
  .copy-id {
    border: 0;
    background: transparent;
    padding: 0;
    min-height: 0;
    color: var(--muted);
    cursor: copy;
  }
  .copy-id:hover {
    text-decoration: underline dotted;
    background: transparent;
  }
</style>
