import { Art } from "../wasm/shelter_wasm.js"
import { app } from "./state.svelte"

/** Pictures come only from the game file the user picks; nothing is sent anywhere. */
class ArtState {
  status = $state<"none" | "loading" | "ready" | "error">("none")
  progress = $state(0)
  error = $state("")
  /** Wasteland journal phrases were read from the game file. */
  texts = $state(false)
  /** Bumped when new pictures can be drawn. */
  rev = $state(0)
}

export const art = new ArtState()

let store: Art | null = null
let file: File | null = null
const bitmaps = new Map<string, Promise<ImageBitmap | null>>()

async function slice(at: number, len: number): Promise<Uint8Array> {
  if (!file) throw new Error("no file")
  return new Uint8Array(await file.slice(at, at + len).arrayBuffer())
}

function pick(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input")
    input.type = "file"
    input.accept = ".unity3d"
    input.addEventListener("change", () => resolve(input.files?.[0] ?? null), { once: true })
    input.addEventListener("cancel", () => resolve(null), { once: true })
    input.click()
  })
}

async function loadTexture(s: Art, name: string): Promise<boolean> {
  const r = s.range(name)
  if (!r) return false
  return s.texture(name, await slice(r[0], r[1]))
}

/** Asks for the game file and reads the pictures from it, on this device only. */
export async function loadGameFile(onPicked?: () => void) {
  const f = await pick()
  if (!f) return
  onPicked?.()
  file = f
  bitmaps.clear()
  art.status = "loading"
  art.progress = 0
  art.error = ""
  const s = new Art()
  try {
    const head = s.head(await slice(0, Art.headSize()))
    if (!head || !s.info(await slice(head[0], head[1]))) throw new Error(app.t("art.web.badFile", { name: Art.bundleName() }))
    for (const g of ["male", "female"]) {
      const m = s.meshRanges(g)
      if (!m || !s.mesh(g, await slice(m[0], m[1]), await slice(m[2], m[3]))) throw new Error(app.t("art.mismatch"))
    }
    const tr = s.textsRange()
    if (tr && s.texts(await slice(tr[0], tr[1]))) {
      art.texts = true
      art.rev++
    }
    const plan = s.plan()
    let failed = 0
    for (let i = 0; i < plan.length; i++) {
      if (!(await loadTexture(s, plan[i]))) failed++
      art.progress = Math.round(((i + 1) / plan.length) * 100)
    }
    if (failed === plan.length) throw new Error(app.t("art.mismatch"))
    store = s
    art.status = "ready"
    art.rev++
  } catch (e) {
    s.free()
    file = null
    art.status = "error"
    art.error = e instanceof Error ? e.message : String(e)
  }
}

function toBitmap(rgba: Uint8Array, w: number, h: number): Promise<ImageBitmap | null> {
  if (!w || !h || rgba.length !== w * h * 4) return Promise.resolve(null)
  return createImageBitmap(new ImageData(new Uint8ClampedArray(rgba), w, h)).catch(() => null)
}

/** Item, room or exterior picture by key, like `Weapon:Shotgun` or `Room:Cafeteria:3:3`. */
export function picture(key: string): Promise<ImageBitmap | null> {
  const s = store
  if (!s) return Promise.resolve(null)
  let p = bitmaps.get(key)
  if (!p) {
    const rgba = s.icon(key)
    const [w, h] = s.lastSize()
    p = rgba ? toBitmap(rgba, w, h) : Promise.resolve(null)
    bitmaps.set(key, p)
  }
  return p
}

let drawing: Promise<unknown> = Promise.resolve()

/** The dweller as the game draws it, from the dweller's JSON. */
export function dwellerPicture(json: string): Promise<ImageBitmap | null> {
  const s = store
  if (!s) return Promise.resolve(null)
  const key = "dweller|" + Art.lookKey(json)
  let p = bitmaps.get(key)
  if (!p) {
    p = (drawing = drawing.then(async () => {
      for (const name of s.dwellerMissing(json)) await loadTexture(s, name)
      const rgba = s.dweller(json)
      const [w, h] = s.lastSize()
      return rgba ? toBitmap(rgba, w, h) : null
    })) as Promise<ImageBitmap | null>
    bitmaps.set(key, p)
  }
  return p
}

export function previewNote(json: string): string | null {
  return Art.previewNote(json) ?? null
}
