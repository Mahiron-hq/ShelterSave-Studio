<script lang="ts">
  import { art, picture } from "../lib/art.svelte"
  import { app } from "../lib/state.svelte"

  interface Props {
    kind: string
    id: string
    alt?: string | null
    label?: string
    maxW?: number
    /** Says so when the game file has no such picture. */
    noteMissing?: boolean
  }
  let { kind, id, alt = null, label = "", maxW = 420, noteMissing = false }: Props = $props()
  let missing = $state(false)
  let canvas = $state<HTMLCanvasElement>()
  let shown = $state(false)

  $effect(() => {
    const c = canvas
    const key = `${kind}:${id}`
    const fallback = alt ? `${kind}:${alt}` : null
    void art.rev
    shown = false
    missing = false
    let alive = true
    void (async () => {
      const bmp = (await picture(key)) ?? (fallback ? await picture(fallback) : null)
      if (alive && !bmp && art.status === "ready") missing = true
      if (!alive || !bmp || !c) return
      let w = Math.min(maxW, c.parentElement?.clientWidth ?? maxW)
      let h = (bmp.height * w) / Math.max(1, bmp.width)
      // Single rooms are nearly square; keep them as tall as a full-width room picture.
      if (h > 110) {
        w *= 110 / h
        h = 110
      }
      const dpr = window.devicePixelRatio || 1
      c.width = Math.round(w * dpr)
      c.height = Math.round(h * dpr)
      c.style.width = `${Math.round(w)}px`
      c.style.height = `${Math.round(h)}px`
      c.getContext("2d")?.drawImage(bmp, 0, 0, c.width, c.height)
      shown = true
    })()
    return () => {
      alive = false
    }
  })
</script>

{#if missing && noteMissing}<p class="muted small">{app.t("theme.noPicture")}</p>{/if}
<canvas bind:this={canvas} class:shown role={label ? "img" : undefined} aria-label={label || undefined} aria-hidden={label ? undefined : "true"}></canvas>

<style>
  canvas {
    display: none;
    border-radius: var(--r-md);
    margin: var(--s-xs) 0;
  }
  canvas.shown {
    display: block;
  }
</style>
