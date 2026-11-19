<script lang="ts">
  import type { Snippet } from "svelte"
  import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "@ui-sdk/svelte/collapsible"
  import Icon from "./Icon.svelte"

  interface Props {
    title: string
    help?: string
    open?: boolean
    children: Snippet
  }
  let { title, help = "", open = false, children }: Props = $props()
</script>

<Collapsible defaultOpen={open} class="fold">
  <CollapsibleTrigger class="fold-trigger" title={help || undefined}>
    <Icon name="chevronRight" />
    <span>{title}</span>
  </CollapsibleTrigger>
  <CollapsiblePanel class="fold-panel">
    {@render children()}
  </CollapsiblePanel>
</Collapsible>

<style>
  :global(.fold-trigger) {
    border: 0;
    background: transparent;
    padding: 4px 2px;
    font-weight: 600;
    width: 100%;
    justify-content: flex-start;
  }
  :global(.fold-trigger[data-open] .icon) {
    transform: rotate(90deg);
  }
  :global(.fold-panel) {
    padding-inline-start: var(--s-md);
  }
</style>
