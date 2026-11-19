<script lang="ts">
  import { art, picture } from "../lib/art.svelte"

  interface Props {
    id: string
    max: number
    alt?: string
  }
  let { id, max, alt = "" }: Props = $props()
  let canvas = $state<HTMLCanvasElement>()
  let shown = $state(false)

  $effect(() => {
    const c = canvas
    const i = id
    void art.rev
    shown = false
    let alive = true
    void picture(i).then((bmp) => {
      if (!alive || !bmp || !c) return
      const s = Math.min(1, max / Math.max(bmp.width, bmp.height))
      c.width = Math.round(bmp.width * s)
      c.height = Math.round(bmp.height * s)
      const ctx = c.getContext("2d")
      ctx?.drawImage(bmp, 0, 0, c.width, c.height)
      shown = true
    })
    return () => {
      alive = false
    }
  })
</script>

<canvas bind:this={canvas} class:shown role={alt ? "img" : undefined} aria-label={alt || undefined} aria-hidden={alt ? undefined : "true"}></canvas>

<style>
  canvas {
    display: none;
    flex: none;
  }
  canvas.shown {
    display: block;
  }
</style>
