import { core, stringsFor, type Path } from "./core"
import { applyTheme, loadSettings, storeSettings, systemLang, type Settings } from "./settings"
import { clearSession, writeSession } from "./session"

export type Section = "save" | "vault" | "dwellers" | "other"

export interface Toast {
  id: number
  text: string
  undo: boolean
  error: boolean
}

export type Modal =
  | { kind: "settings" }
  | { kind: "about" }
  | { kind: "journal" }
  | { kind: "gameFile" }
  | { kind: "review" }
  | { kind: "afterDownload"; name: string }
  | { kind: "expertConfirm" }
  | { kind: "closeConfirm" }
  | { kind: "confirmMass"; action: any; count: number }
  | { kind: "restoreSession"; fileName: string; savedAt: number; session: string }
  | { kind: "custom"; id: string; data?: any }

let toastId = 0

class AppState {
  settings = $state<Settings>(loadSettings())
  pageLang: "en" | "ru" = document.documentElement.lang === "ru" ? "ru" : "en"
  strings = $state.raw<Record<string, string>>({})
  ready = $state(false)
  loadError = $state<string | null>(null)
  /** Kind of the last opening error, for the anonymous counter. */
  loadErrorCode = ""

  /** Bumped on every change of the open document. */
  rev = $state(0)
  root = $state.raw<any>(null)
  /** Length of the data as compact JSON. */
  size = $state(0)
  fileName = $state("")
  dirty = $state(false)
  canUndo = $state(false)
  canRedo = $state(false)

  section = $state<Section>("save")
  vaultTab = $state("resources")
  otherTab = $state("quests")
  toasts = $state<Toast[]>([])
  modals = $state<Modal[]>([])
  /** Path focused by search or review, shown by the section. */
  focus = $state<Path | null>(null)

  private sessionTimer = 0

  get lang(): "en" | "ru" {
    return this.pageLang
  }

  get ru(): boolean {
    return this.lang === "ru"
  }

  get itemRu(): boolean {
    return this.settings.russianItemNames
  }

  get expert(): boolean {
    return this.settings.expert
  }

  get isOpen(): boolean {
    return this.root !== null
  }

  init() {
    this.strings = stringsFor(this.lang)
    applyTheme(this.settings.theme)
    this.syncPrefs()
  }

  /** Language from settings, or from the browser. */
  wantedLang(): "en" | "ru" {
    return this.settings.lang === "system" ? systemLang() : this.settings.lang
  }

  t(key: string, params?: Record<string, string | number>): string {
    let s = this.strings[key] ?? key
    if (params) for (const [k, v] of Object.entries(params)) s = s.replaceAll(`{${k}}`, String(v))
    return s
  }

  saveSettings() {
    storeSettings($state.snapshot(this.settings))
    applyTheme(this.settings.theme)
    this.syncPrefs()
    this.refresh(false)
  }

  syncPrefs() {
    core().prefs(this.lang, this.settings.russianItemNames, this.settings.expert)
  }

  /** Re-reads the document after a change. */
  refresh(changed = true) {
    const ed = core()
    if (!ed.isOpen()) {
      this.root = null
      this.fileName = ""
      this.dirty = false
      this.canUndo = this.canRedo = false
      this.rev++
      return
    }
    const text = ed.root()
    this.size = text.length
    this.root = JSON.parse(text)
    this.fileName = ed.fileName()
    this.dirty = ed.dirty()
    const h = JSON.parse(ed.history())
    this.canUndo = !!h.canUndo
    this.canRedo = !!h.canRedo
    this.rev++
    if (changed) this.keepSession()
  }

  /** Writes the editing session to the browser a moment after a change. */
  keepSession() {
    clearTimeout(this.sessionTimer)
    if (!this.settings.keepSession) return
    this.sessionTimer = window.setTimeout(() => {
      const ed = core()
      const session = ed.isOpen() ? ed.session() : undefined
      if (!session) return
      void writeSession({ session, fileName: ed.fileName(), savedAt: Date.now() })
    }, 400)
  }

  open(bytes: Uint8Array, name: string): boolean {
    try {
      core().open(bytes, name)
    } catch (e) {
      const err = parseErr(e)
      this.loadError = err.text
      this.loadErrorCode = err.code
      return false
    }
    this.loadError = null
    this.section = "save"
    this.focus = null
    this.refresh()
    return true
  }

  close() {
    core().close()
    clearTimeout(this.sessionTimer)
    void clearSession()
    this.refresh(false)
  }

  /** Runs an editor action; returns false when it changed nothing or failed. */
  act(action: any, raw?: string): boolean {
    let json = JSON.stringify(action)
    if (raw !== undefined) json = json.replace('"__RAW__"', raw)
    let res: { label: string; affected: number }
    try {
      res = JSON.parse(core().apply(json))
    } catch (e) {
      const err = parseErr(e)
      if (err.text) this.toastError(err.text)
      return false
    }
    this.refresh()
    if (res.affected > 1) this.toast(this.t("c.undoToast", { label: res.label, n: res.affected }), true)
    return true
  }

  /** Bulk action: asks first when the setting says so. */
  actMass(action: any, count: number) {
    if (this.settings.confirmMass && count > 1) this.modals.push({ kind: "confirmMass", action, count })
    else this.act(action)
  }

  undo() {
    let done: string | undefined
    try {
      done = core().undo()
    } catch (e) {
      this.toastError(this.t("aerr.edit", { detail: parseErr(e).text }))
      return
    }
    if (done !== undefined) this.refresh()
  }

  redo() {
    let done: string | undefined
    try {
      done = core().redo()
    } catch (e) {
      this.toastError(this.t("aerr.edit", { detail: parseErr(e).text }))
      return
    }
    if (done !== undefined) this.refresh()
  }

  toast(text: string, undo = false) {
    const id = ++toastId
    this.toasts.push({ id, text, undo, error: false })
    setTimeout(() => this.dismiss(id), 6000)
  }

  toastError(text: string) {
    const id = ++toastId
    this.toasts.push({ id, text, undo: false, error: true })
    setTimeout(() => this.dismiss(id), 8000)
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id)
  }

  push(m: Modal) {
    this.modals.push(m)
  }

  pop() {
    this.modals.pop()
  }
}

export function parseErr(e: unknown): { code: string; text: string } {
  const msg = e instanceof Error ? e.message : String(e)
  try {
    const v = JSON.parse(msg)
    return { code: String(v.code ?? ""), text: String(v.text ?? "") }
  } catch {
    return { code: "", text: msg }
  }
}

export const app = new AppState()
