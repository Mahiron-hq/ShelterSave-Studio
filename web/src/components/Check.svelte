<script lang="ts">
  import { Checkbox, CheckboxIndicator } from "@ui-sdk/svelte/checkbox"
  import Icon from "./Icon.svelte"

  interface Props {
    checked: boolean
    label: string
    /** Accessible name when the visible label is empty. */
    aria?: string
    disabled?: boolean
    onchange: (v: boolean) => void
  }
  let { checked, label, aria, disabled = false, onchange }: Props = $props()
</script>

<label class="check" class:disabled>
  <Checkbox {checked} {disabled} aria-label={label ? undefined : aria} onCheckedChange={(v) => onchange(v === true)}>
    <CheckboxIndicator aria-hidden="true"><Icon name="check" /></CheckboxIndicator>
  </Checkbox>
  {#if label}<span>{label}</span>{/if}
</label>

<style>
  .check {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
  }
  .check.disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .check :global([data-slot="checkbox-indicator"] .icon) {
    font-size: 14px;
  }
</style>
