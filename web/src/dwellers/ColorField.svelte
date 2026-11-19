<script lang="ts">
  import {
    ColorPicker,
    ColorPickerArea,
    ColorPickerAreaThumb,
    ColorPickerInput,
    ColorPickerSlider,
    ColorPickerSliderThumb,
  } from "@ui-sdk/svelte/color-picker"
  import { Popover, PopoverContent, PopoverTrigger } from "@ui-sdk/svelte/popover"
  import Row from "../fields/Row.svelte"
  import Swatches from "../components/Swatches.svelte"
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
  let draft = $state<string | null>(null)
  const shown = $derived(draft ?? hexColor(value))

  function commit(v: string) {
    draft = null
    const m = /^#?([0-9a-f]{6})$/i.exec(v.trim())
    if (!m) return
    const rgb = parseInt(m[1], 16)
    if (rgb !== value) onset(rgb)
  }
</script>

<Row {path} {label}>
  <div class="color">
    <Popover>
      <PopoverTrigger class="swatch-btn" aria-label="{label}: {app.t('dw.colorPick')}" title={app.t("dw.colorPick")} style="--sw: {shown}">
        <span class="mono">{shown}</span>
      </PopoverTrigger>
      <PopoverContent class="picker">
        <ColorPicker value={shown} format="hex" onValueChange={(v) => (draft = v)} onValueCommit={commit}>
          <ColorPickerArea class="area"><ColorPickerAreaThumb class="thumb" /></ColorPickerArea>
          <ColorPickerSlider channel="hue" class="hue"><ColorPickerSliderThumb class="thumb" /></ColorPickerSlider>
          <ColorPickerInput class="mono hex" aria-label={app.t("dw.colorCode")} />
        </ColorPicker>
      </PopoverContent>
    </Popover>
    {#if presets.length}
      <Swatches label="{label}: {app.t('dw.gameColor')}" {value} colors={presets} onchange={onset} />
    {/if}
  </div>
</Row>

<style>
  .color {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  :global(.swatch-btn) {
    min-width: 120px;
    justify-content: flex-start;
    gap: 8px;
  }
  :global(.swatch-btn)::before {
    content: "";
    width: 28px;
    height: 18px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--sw);
  }
  :global(.picker) {
    width: 240px;
  }
  :global(.picker .area) {
    position: relative;
    height: 140px;
    border-radius: var(--r-sm);
    background: var(--color-area-background);
    margin-bottom: 8px;
    touch-action: none;
  }
  :global(.picker .hue) {
    position: relative;
    height: 16px;
    border-radius: var(--r-sm);
    background: var(--color-slider-background);
    margin-bottom: 8px;
    touch-action: none;
  }
  :global(.picker .thumb) {
    position: absolute;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px #000;
    background: var(--color-picker-color);
  }
  :global(.picker .hex) {
    width: 100%;
  }
</style>
