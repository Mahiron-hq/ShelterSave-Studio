import { app } from "./state.svelte"
import { hit } from "./stats"

export const MAX_FILE = 32 * 1024 * 1024

export async function openFile(file: File): Promise<boolean> {
  if (file.size > MAX_FILE) {
    app.loadError = app.t("err.too_large")
    hit("error", "too_large")
    return false
  }
  let bytes: Uint8Array
  try {
    bytes = new Uint8Array(await file.arrayBuffer())
  } catch (e) {
    app.loadError = app.t("err.io", { detail: e instanceof Error ? e.message : String(e) })
    hit("error", "io")
    return false
  }
  const ok = app.open(bytes, file.name || "Vault1.sav")
  if (ok) hit("open")
  else hit("error", app.loadErrorCode || "other")
  return ok
}

/** Asks the browser for a file. */
export function pickFile(accept = ".sav,.json"): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input")
    input.type = "file"
    input.accept = accept
    input.addEventListener("change", () => resolve(input.files?.[0] ?? null), { once: true })
    input.addEventListener("cancel", () => resolve(null), { once: true })
    input.click()
  })
}

export function download(name: string, text: string, type = "application/octet-stream") {
  const url = URL.createObjectURL(new Blob([text], { type }))
  const a = document.createElement("a")
  a.href = url
  a.download = name
  document.body.append(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 10_000)
}
