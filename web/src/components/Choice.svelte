<script lang="ts" generics="T extends string">
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field"
  import { Radio, RadioGroup, RadioIndicator } from "@ui-sdk/svelte/radio"

  interface Props {
    label: string
    help?: string
    value: T
    options: { value: T; label: string }[]
    onchange: (v: T) => void
  }
  let { label, help = "", value, options, onchange }: Props = $props()
</script>

<Field class="setting">
  <div class="text">
    <FieldLabel>{label}</FieldLabel>
    {#if help}<FieldDescription>{help}</FieldDescription>{/if}
  </div>
  <RadioGroup class="radios" {value} onValueChange={(v) => v !== null && onchange(v as T)}>
    {#each options as o (o.value)}
      <label class="radio">
        <Radio value={o.value}><RadioIndicator /></Radio>
        {o.label}
      </label>
    {/each}
  </RadioGroup>
</Field>

<style>
  :global(.setting) {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--s-md);
    padding: var(--s-sm) 0;
    border-bottom: 1px solid var(--border);
  }
  :global(.setting [data-slot="field-label"]) {
    font-weight: 600;
  }
  :global(.setting [data-slot="field-description"]) {
    color: var(--muted);
    font-size: 12px;
    margin: 2px 0 0;
  }
  :global(.radios) {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    align-items: center;
  }
  .radio {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    cursor: pointer;
  }
  :global([data-slot="radio"]) {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--muted);
    background: var(--surface);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    min-height: 0;
  }
  :global([data-slot="radio"][data-checked]) {
    border-color: var(--accent);
  }
  :global([data-slot="radio-indicator"]) {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
  }
  @media (max-width: 600px) {
    :global(.setting) {
      grid-template-columns: minmax(0, 1fr);
      gap: 6px;
    }
  }
</style>
