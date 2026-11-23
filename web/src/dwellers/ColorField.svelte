<script lang="ts">
  import Row from "../fields/Row.svelte"
  import Swatches from "../components/Swatches.svelte"
  import Icon from "../components/Icon.svelte"
  import { app } from "../lib/state.svelte"
  import { hexColor } from "../lib/format"
  import type { Path } from "../lib/core"

  interface Props {
    path: Path
    label: string
    /** 0xRRGGBB */
    value: number
    presets: number[]
    onset: (rgb: number) => void
  }
  let { path, label, value, presets, onset }: Props = $props()
  let text = $state<string | null>(null)
  let open = $state(false)
  // Colour while the palette is dragged; written on release.
  let pending = $state<number | null>(null)
  // The hue is kept while dragging, so grey and black do not reset it.
  let keptHue = $state<number | null>(null)

  const cur = $derived(pending ?? value)
  const hsv = $derived.by(() => {
    const v = toHsv(cur)
    if (keptHue !== null && (v.s < 0.01 || v.v < 0.01)) v.h = keptHue
    return v
  })

  function toHsv(rgb: number) {
    const r = ((rgb >> 16) & 255) / 255
    const g = ((rgb >> 8) & 255) / 255
    const b = (rgb & 255) / 255
    const max = Math.max(r, g, b)
    const d = max - Math.min(r, g, b)
    let h = 0
    if (d > 0) {
      if (max === r) h = ((g - b) / d + 6) % 6
      else if (max === g) h = (b - r) / d + 2
      else h = (r - g) / d + 4
      h /= 6
    }
    return { h, s: max === 0 ? 0 : d / max, v: max }
  }
  function fromHsv(h: number, s: number, v: number): number {
    const f = (n: number) => {
      const k = (n + h * 6) % 6
      return Math.round(255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))))
    }
    return (f(5) << 16) | (f(3) << 8) | f(1)
  }

  function commitText(v: string) {
    text = null
    const m = /^#?([0-9a-f]{6})$/i.exec(v.trim())
    if (!m) return
    const rgb = parseInt(m[1], 16)
    if (rgb !== value) onset(rgb)
  }
  function finish() {
    if (pending !== null && pending !== value) onset(pending)
    pending = null
  }

  function drag(kind: "sv" | "hue") {
    return (e: PointerEvent) => {
      const el = e.currentTarget as HTMLElement
      el.setPointerCapture(e.pointerId)
      const move = (ev: PointerEvent) => {
        const r = el.getBoundingClientRect()
        const x = Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width))
        const y = Math.min(1, Math.max(0, (ev.clientY - r.top) / r.height))
        const c = hsv
        if (kind === "sv") pending = fromHsv(c.h, x, 1 - y)
        else {
          keptHue = Math.min(x, 0.9999)
          pending = fromHsv(keptHue, c.s, c.v)
        }
      }
      const up = () => {
        el.removeEventListener("pointermove", move)
        el.removeEventListener("pointerup", up)
        el.removeEventListener("pointercancel", up)
        finish()
      }
      el.addEventListener("pointermove", move)
      el.addEventListener("pointerup", up)
      el.addEventListener("pointercancel", up)
      move(e)
    }
  }
  function keys(kind: "sv" | "hue") {
    return (e: KeyboardEvent) => {
      const step = e.shiftKey ? 0.1 : 0.02
      const c = { ...hsv }
      if (kind === "hue") {
        if (e.key === "ArrowLeft") c.h = Math.max(0, c.h - step)
        else if (e.key === "ArrowRight") c.h = Math.min(0.9999, c.h + step)
        else return
        keptHue = c.h
      } else if (e.key === "ArrowLeft") c.s = Math.max(0, c.s - step)
      else if (e.key === "ArrowRight") c.s = Math.min(1, c.s + step)
      else if (e.key === "ArrowUp") c.v = Math.min(1, c.v + step)
      else if (e.key === "ArrowDown") c.v = Math.max(0, c.v - step)
      else return
      e.preventDefault()
      pending = fromHsv(c.h, c.s, c.v)
    }
  }
</script>

<Row {path} {label}>
  <div class="color">
    <div class="line">
      <button
        type="button"
        class="pick-swatch"
        class:open
        style:background={hexColor(cur)}
        aria-expanded={open}
        aria-label="{label}: {app.t('dw.colorPick')}"
        title={app.t("dw.colorPick")}
        onclick={() => (open = !open)}
      ></button>
      <input
        class="mono hex"
        aria-label={app.t("dw.colorCode")}
        title={app.t("dw.colorCode")}
        value={text ?? hexColor(cur)}
        oninput={(e) => (text = e.currentTarget.value)}
        onblur={(e) => commitText(e.currentTarget.value)}
        onkeydown={(e) => e.key === "Enter" && commitText(e.currentTarget.value)}
      />
    </div>
    {#if presets.length}
      <Swatches label="{label}: {app.t('dw.gameColor')}" {value} colors={presets} onchange={onset} />
    {/if}
    {#if open}
      <div class="palette">
        <div
          class="sv"
          role="slider"
          tabindex="0"
          aria-label={app.t("dw.colorPick")}
          aria-valuenow={Math.round(hsv.s * 100)}
          aria-valuetext={hexColor(cur)}
          style:--hue={`hsl(${hsv.h * 360} 100% 50%)`}
          onpointerdown={drag("sv")}
          onkeydown={keys("sv")}
          onkeyup={finish}
        >
          <span class="mark" style:left="{hsv.s * 100}%" style:top="{(1 - hsv.v) * 100}%"></span>
        </div>
        <div
          class="hue"
          role="slider"
          tabindex="0"
          aria-label={app.t("dw.colorPick")}
          aria-valuemin={0}
          aria-valuemax={360}
          aria-valuenow={Math.round(hsv.h * 360)}
          onpointerdown={drag("hue")}
          onkeydown={keys("hue")}
          onkeyup={finish}
        >
          <span class="bar" style:left="{hsv.h * 100}%"></span>
        </div>
        <button type="button" class="small-btn" onclick={() => (open = false)}><Icon name="check" /> {app.t("dw.paletteClose")}</button>
      </div>
    {/if}
  </div>
</Row>

<style>
  .color {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .line {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .pick-swatch {
    width: 40px;
    height: 24px;
    min-height: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    cursor: pointer;
    transition:
      box-shadow var(--motion),
      transform var(--motion);
  }
  .pick-swatch:hover {
    transform: scale(1.06);
  }
  .pick-swatch.open {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .hex {
    width: 96px;
  }
  .palette {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 240px;
    max-width: 100%;
    animation: unfold var(--motion) ease-out;
  }
  .sv {
    position: relative;
    height: 144px;
    border-radius: var(--r-sm);
    background:
      linear-gradient(to top, #000, transparent),
      linear-gradient(to right, #fff, var(--hue));
    touch-action: none;
    cursor: crosshair;
  }
  .hue {
    position: relative;
    height: 18px;
    border-radius: var(--r-sm);
    background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
    touch-action: none;
    cursor: ew-resize;
  }
  .mark {
    position: absolute;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px #000;
    pointer-events: none;
  }
  .bar {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 4px;
    margin-left: -2px;
    border: 2px solid #fff;
    border-radius: 1px;
    box-shadow: 0 0 0 1px #000;
    pointer-events: none;
  }
  .small-btn {
    align-self: flex-start;
  }
  @keyframes unfold {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
