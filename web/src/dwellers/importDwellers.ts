import { readDwellers } from "../lib/core"
import { MAX_FILE, pickFile } from "../lib/files"
import { app } from "../lib/state.svelte"

/** Reads another save (it is only read) and offers its dwellers. */
export async function importDwellers() {
  const f = await pickFile()
  if (!f) return
  if (f.size > MAX_FILE) {
    app.toastError(app.t("err.too_large"))
    return
  }
  let list
  try {
    list = readDwellers(new Uint8Array(await f.arrayBuffer()))
  } catch {
    app.toastError(app.t("dw.importBad"))
    return
  }
  if (!list.length) {
    app.toastError(app.t("dw.importEmpty"))
    return
  }
  app.push({ kind: "custom", id: "importDwellers", data: { file: f.name, list } })
}
