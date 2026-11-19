import init, {
  Editor,
  catalog_json,
  consts as wasmConsts,
  look_choices,
  read_dwellers,
  journal_dwellers,
  room_shape,
  theme_picture,
  outfit_fits,
  search_items,
  describe as wasmDescribe,
  is_int_flag,
  strings as wasmStrings,
  ticks_format,
  ticks_parse,
} from "../wasm/shelter_wasm.js"
import wasmUrl from "../wasm/shelter_wasm_bg.wasm?url"

export type Seg = string | number
export type Path = Seg[]

export interface Names {
  en: string
  ru: string
  ruOfficial?: boolean
}

let editor: Editor | null = null
let catalogCache: any = null

export async function loadCore(): Promise<Editor> {
  if (editor) return editor
  await init({ module_or_path: wasmUrl })
  editor = new Editor()
  return editor
}

export function core(): Editor {
  if (!editor) throw new Error("core not loaded")
  return editor
}

export function catalog(): any {
  catalogCache ??= JSON.parse(catalog_json())
  return catalogCache
}

export function stringsFor(lang: string): Record<string, string> {
  return JSON.parse(wasmStrings(lang))
}

export function describe(lang: string, path: Path): { label: string; help: string; known: boolean } {
  return JSON.parse(wasmDescribe(lang, pathStr(path)))
}

export interface Consts {
  maxDwellers: number
  resourceMax: number
  lunchboxMax: number
  itemAddMax: number
  rarities: string[]
  resourceKeys: string[]
  playerResources: string[]
  themeParts: number
  roomIncidents: number[]
  noIncidentRooms: string[]
  fixedRooms: string[]
  notBuildable: string[]
  uniqueRooms: string[]
  questMemory: [string, [string, string, string]][]
  themeRooms: string[]
  holidays: [string, string][]
}
let constsCache: Consts | null = null
export function consts(): Consts {
  constsCache ??= JSON.parse(wasmConsts())
  return constsCache!
}

export type ItemKind = "Weapon" | "Outfit" | "Junk" | "Pet"
export const ITEM_KINDS: ItemKind[] = ["Weapon", "Outfit", "Junk", "Pet"]
export interface FoundItem {
  kind: ItemKind
  id: string
  rarity: string
  name: Names
}
export function searchItems(kinds: ItemKind[], q: string, gender = 0): FoundItem[] {
  return JSON.parse(search_items(kinds.join(","), q, gender))
}
export const outfitFits = (id: string, gender: number) => outfit_fits(id, gender)
export const lookChoices = (gender: number, field: string): string[] => JSON.parse(look_choices(gender, field))

export const roomShape = (kind: string): { merges: number[]; maxLevel: number[]; width: number[] } =>
  JSON.parse(room_shape(kind))
export const themePicture = (room: string, theme: string | null): [string, string] =>
  JSON.parse(theme_picture(room, theme ?? undefined))

export interface ForeignDweller {
  name: string
  level: number
  rarity: string
  json: string
}
export const readDwellers = (bytes: Uint8Array): ForeignDweller[] => JSON.parse(read_dwellers(bytes))
export const journalDwellers = (): { id: string; first: string; last: string; outfit: string; weapon: string; special: number[] }[] =>
  JSON.parse(journal_dwellers())

/** Helper of the core that reads the open save. */
export function query<T = any>(name: string, args: Record<string, unknown> = {}): T {
  return JSON.parse(core().query(name, JSON.stringify(args)))
}

const kindKey: Record<ItemKind, string> = { Weapon: "weapons", Outfit: "outfits", Junk: "junk", Pet: "pets" }
/** Catalog entry of an item, if known. */
export function item(kind: ItemKind, id: string): any {
  return (catalog()[kindKey[kind]] as any[] | undefined)?.find((x) => x.id === id)
}

export const ticksFormat = (t: string) => ticks_format(t) ?? null
export const ticksParse = (text: string, keep: string) => ticks_parse(text, keep) ?? null
export const isIntFlag = (key: string) => is_int_flag(key)

export function pathStr(path: Path): string {
  let s = ""
  for (const seg of path) {
    if (typeof seg === "number") s += `[${seg}]`
    else s += s ? `.${seg}` : seg
  }
  return s
}

export function at(root: any, path: Path): any {
  let v = root
  for (const seg of path) {
    if (v == null) return undefined
    v = v[seg]
  }
  return v
}

/** Name in the chosen language; Russian only when it is the official one. */
export function nameOf(n: Names | undefined, ru: boolean, fallback = ""): string {
  if (!n) return fallback
  return ru && n.ruOfficial && n.ru ? n.ru : n.en || fallback
}

export function rarityName(rarity: string, ru: boolean): string {
  const key = rarity === "Normal" ? "Common" : rarity
  const n = catalog().names?.rarity?.[key]
  if (!n) return rarity
  const s = nameOf(n, ru, rarity).toLowerCase()
  return s.charAt(0).toUpperCase() + s.slice(1)
}
