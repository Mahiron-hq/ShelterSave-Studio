import { consts } from "../lib/core"
import { arr, DWELLERS, dwellerName, isDead } from "../lib/vault"

export interface DwellerRow {
  id: number
  name: string
  level: number
  happiness: number
  health: number
  special: number
  rarity: string
  dead: boolean
  gender: number
}

export function dwellerRows(root: any): DwellerRow[] {
  return arr(root, DWELLERS)
    .filter((d) => typeof d?.serializeId === "number")
    .map((d) => ({
      id: d.serializeId,
      name: dwellerName(d),
      level: d.experience?.currentLevel ?? 1,
      happiness: d.happiness?.happinessValue ?? 0,
      health: d.health?.healthValue ?? 0,
      special: [1, 2, 3, 4, 5, 6, 7].reduce((a, s) => a + (d.stats?.stats?.[s]?.value ?? 0), 0),
      rarity: typeof d.rarity === "string" ? d.rarity : "",
      dead: isDead(d),
      gender: d.gender ?? 0,
    }))
}

export function rarityRank(r: string): number {
  const i = consts().rarities.indexOf(r)
  return i < 0 ? 9 : i
}

type Cmp = (a: DwellerRow, b: DwellerRow) => number

export const SORTS: [string, Cmp][] = [
  ["dw.sort.name", (a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase())],
  ["dw.sort.level", (a, b) => a.level - b.level],
  ["dw.sort.happiness", (a, b) => a.happiness - b.happiness],
  ["dw.sort.health", (a, b) => a.health - b.health],
  ["dw.sort.special", (a, b) => a.special - b.special],
  ["dw.sort.rarity", (a, b) => rarityRank(a.rarity) - rarityRank(b.rarity)],
  ["dw.sort.id", (a, b) => a.id - b.id],
]

export const FILTERS: [string, (r: DwellerRow) => boolean][] = [
  ["dw.filter.all", () => true],
  ["dw.filter.alive", (r) => !r.dead],
  ["dw.filter.dead", (r) => r.dead],
  ["dw.filter.male", (r) => r.gender === 2],
  ["dw.filter.female", (r) => r.gender === 1],
]

export function label(r: DwellerRow): string {
  return r.name || "#" + r.id
}
