<script lang="ts" generics="T extends string">
  import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@ui-sdk/svelte/select"
  import Icon from "./Icon.svelte"
  import type { Snippet } from "svelte"

  interface Props {
    value: T | null
    items: { value: T; label: string }[]
    label: string
    placeholder?: string
    disabled?: boolean
    onchange: (v: T) => void
    /** Card for the highlighted item, shown beside the list. */
    preview?: Snippet<[T]>
  }
  let { value, items, label, placeholder = "", disabled = false, onchange, preview }: Props = $props()
  let lit = $state<T | null>(null)
</script>

<Select
  {items}
  {value}
  {placeholder}
  {disabled}
  onValueChange={(v) => v !== null && onchange(v as T)}
  onHighlight={(v) => (lit = (v as T | null) ?? null)}
  onOpenChange={(open) => {
    if (!open) lit = null
  }}
>
  <SelectTrigger aria-label={label} class="pick-trigger" title={items.find((i) => i.value === value)?.label}>
    <SelectValue />
    <Icon name="chevronDown" />
  </SelectTrigger>
  {#if preview}
    <SelectContent class="with-info">
      <div class="list">
        {#each items as it (it.value)}
          <SelectItem value={it.value}>{it.label}</SelectItem>
        {/each}
      </div>
      {#if lit !== null}
        <aside class="info card" aria-live="polite">{@render preview(lit)}</aside>
      {/if}
    </SelectContent>
  {:else}
    <SelectContent>
      {#each items as it (it.value)}
        <SelectItem value={it.value}>{it.label}</SelectItem>
      {/each}
    </SelectContent>
  {/if}
</Select>

<style>
  :global([data-slot="select-content"].with-info) {
    display: flex;
    gap: var(--s-sm);
    align-items: flex-start;
    overflow: hidden;
  }
  .list {
    overflow: auto;
    max-height: min(350px, var(--available-height, 350px));
    min-width: min(220px, 80vw);
  }
  .info {
    width: 300px;
    max-height: min(350px, var(--available-height, 350px));
    overflow: auto;
    padding: 4px var(--s-sm);
    border-inline-start: 1px solid var(--border);
  }
  @media (max-width: 640px) {
    :global([data-slot="select-content"].with-info) {
      flex-direction: column;
    }
    .list {
      max-height: 180px;
    }
    .info {
      width: auto;
      max-height: 170px;
      border-inline-start: 0;
      border-top: 1px solid var(--border);
    }
  }
  :global(.pick-trigger) {
    max-width: min(320px, 100%);
  }
  :global(.pick-trigger [data-slot="select-value"]) {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
