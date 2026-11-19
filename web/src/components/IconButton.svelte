<script lang="ts">
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
  <TooltipTrigger class="icon ghost" aria-label={label} {disabled} {onclick}>
    <Icon name={icon} />
  </TooltipTrigger>
  <TooltipContent>{label}</TooltipContent>
</Tooltip>
