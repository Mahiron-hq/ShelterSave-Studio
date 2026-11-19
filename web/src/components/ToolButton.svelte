<script lang="ts">
  import { ToolbarButton } from "@ui-sdk/svelte/toolbar"
  import { Tooltip, TooltipContent, TooltipTrigger } from "@ui-sdk/svelte/tooltip"
  import Icon from "./Icon.svelte"
  import type { IconName } from "../lib/icons"

  interface Props {
    icon: IconName
    label: string
    onclick: () => void
    disabled?: boolean
  }
  let { icon, label, onclick, disabled = false }: Props = $props()
</script>

<Tooltip>
  <TooltipTrigger>
    {#snippet render({ props })}
      <ToolbarButton {...props} class="icon ghost" aria-label={label} {disabled} {onclick}>
        <Icon name={icon} />
      </ToolbarButton>
    {/snippet}
  </TooltipTrigger>
  <TooltipContent>{label}</TooltipContent>
</Tooltip>
