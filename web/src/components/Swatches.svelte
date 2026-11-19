<script lang="ts">
  import { Radio, RadioGroup } from "@ui-sdk/svelte/radio"
  import { hexColor } from "../lib/format"

  interface Props {
    label: string
    /** Current colour, 0xRRGGBB. */
    value: number
    colors: number[]
    onchange: (rgb: number) => void
  }
  let { label, value, colors, onchange }: Props = $props()
  const list = $derived([value, ...colors.map((c) => (c >>> 0) & 0xffffff).filter((c) => c !== value)])
</script>

<RadioGroup class="swatches" aria-label={label} value={hexColor(value)} onValueChange={(v) => v && v !== hexColor(value) && onchange(parseInt(v.slice(1), 16))}>
  {#each list as col (col)}
    <Radio value={hexColor(col)} class="swatch" aria-label={hexColor(col)} title={hexColor(col)} style="--sw: {hexColor(col)}"></Radio>
  {/each}
</RadioGroup>

<style>
  :global(.swatches) {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  :global([data-slot="radio"].swatch) {
    width: 22px;
    height: 18px;
    min-height: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--sw);
  }
  :global([data-slot="radio"].swatch[data-checked]) {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
