import type { Path } from "./core"

export const DWELLERS: Path = ["dwellers", "dwellers"]
export const ACTORS: Path = ["dwellers", "actors"]
export const ROOMS: Path = ["vault", "rooms"]
export const ITEMS: Path = ["vault", "inventory", "items"]
export const RESOURCES: Path = ["vault", "storage", "resources"]
export const SPECIAL_LETTERS = ["S", "P", "E", "C", "I", "A", "L"]

export function arr(root: any, path: Path): any[] {
  let v = root
  for (const s of path) v = v?.[s]
  return Array.isArray(v) ? v : []
}

export function dwellerName(d: any): string {
  return `${typeof d?.name === "string" ? d.name : ""} ${typeof d?.lastName === "string" ? d.lastName : ""}`.trim()
}

export function isDead(d: any): boolean {
  return (typeof d?.deathTime === "number" && d.deathTime >= 0) || d?.health?.permaDeath === true
}
