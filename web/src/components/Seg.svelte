<script lang="ts" generics="T extends string | number">
  import { SegmentedControl, SegmentedControlItem } from "@ui-sdk/svelte/segmented-control"

  interface Props {
    label: string
    value: T | null
    options: { value: T; label: string; disabled?: boolean; title?: string }[]
    onchange: (v: T) => void
  }
  let { label, value, options, onchange }: Props = $props()
</script>

<SegmentedControl
  class="seg"
  aria-label={label}
  value={value === null ? null : String(value)}
  onValueChange={(v) => {
    const o = options.find((x) => String(x.value) === v)
    if (o && o.value !== value) onchange(o.value)
  }}
>
  {#each options as o (o.value)}
    <SegmentedControlItem value={String(o.value)} disabled={o.disabled} title={o.title}>{o.label}</SegmentedControlItem>
  {/each}
</SegmentedControl>

<style>
  :global([data-slot="segmented-control"].seg) {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-alt);
  }
  :global(.seg [data-slot="segmented-control-item"]) {
    border: 0;
    background: transparent;
    min-height: 28px;
    padding: 2px 10px;
  }
  :global(.seg [data-slot="segmented-control-item"][data-checked]) {
    background: var(--surface);
    color: var(--accent);
    font-weight: 600;
    box-shadow: inset 0 0 0 1px var(--accent);
  }
</style>
