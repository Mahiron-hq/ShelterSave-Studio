import { app } from "../lib/state.svelte"
import { arr, DWELLERS, dwellerName } from "../lib/vault"

/** Name of a dweller in the vault or away on a quest. */
export function nameAny(id: number): string {
  for (const list of [["questDwellers", "dwellers"], DWELLERS]) {
    const d = arr(app.root, list).find((x) => x?.serializeId === id)
    if (d) return dwellerName(d)
  }
  return `#${id}`
}
