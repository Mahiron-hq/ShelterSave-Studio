<script lang="ts">
  import ArtNotice from "../art/ArtNotice.svelte"
  import { art, dwellerPicture, previewNote } from "../lib/art.svelte"
  import { app } from "../lib/state.svelte"

  interface Props {
    d: any
  }
  let { d }: Props = $props()
  let canvas = $state<HTMLCanvasElement>()
  let shown = $state(false)
  const json = $derived(JSON.stringify(d))
  const note = $derived(art.status === "ready" ? previewNote(json) : null)

  $effect(() => {
    const c = canvas
    const j = json
    if (art.status !== "ready" || !c) return
    let alive = true
    void dwellerPicture(j).then((bmp) => {
      if (!alive || !bmp) return
      const h = Math.min(260, bmp.height)
      const w = (bmp.width * h) / bmp.height
      const dpr = window.devicePixelRatio || 1
      c.width = Math.round(w * dpr)
      c.height = Math.round(h * dpr)
      c.style.width = `${Math.round(w)}px`
      c.style.height = `${Math.round(h)}px`
      const ctx = c.getContext("2d")
      ctx?.clearRect(0, 0, c.width, c.height)
      ctx?.drawImage(bmp, 0, 0, c.width, c.height)
      shown = true
    })
    return () => {
      alive = false
    }
  })
</script>

<div class="preview">
  {#if art.status === "ready"}
    <div class="frame" title={app.t("dw.previewHelp")} role="img" aria-label={app.t("dw.previewHelp")}>
      <canvas bind:this={canvas} class:shown aria-hidden="true"></canvas>
      {#if !shown}<p class="muted small">{app.t("dw.previewLoading")}</p>{/if}
    </div>
    {#if note}<p class="muted small">{app.t(note)}</p>{/if}
  {:else}
    <ArtNotice compact />
  {/if}
</div>

<style>
  .preview {
    max-width: 260px;
  }
  .frame {
    background: var(--surface-alt);
    border-radius: var(--r-lg);
    padding: var(--s-sm);
    min-height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  canvas {
    display: none;
  }
  canvas.shown {
    display: block;
  }
  p {
    margin: 4px 0 0;
  }
</style>
