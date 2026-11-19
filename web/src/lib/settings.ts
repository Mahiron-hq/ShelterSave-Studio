export type LangPref = "system" | "en" | "ru"
export type ThemePref = "system" | "light" | "dark"

export interface Settings {
  version: number
  lang: LangPref
  theme: ThemePref
  russianItemNames: boolean
  expert: boolean
  confirmMass: boolean
  keepSession: boolean
  stats: boolean
  firstNoticeSeen: boolean
  dwellerScroll: "shared" | "top"
}

const KEY = "sss.settings"
const VERSION = 1

export const defaults: Settings = {
  version: VERSION,
  lang: "system",
  theme: "system",
  russianItemNames: false,
  expert: false,
  confirmMass: true,
  keepSession: true,
  stats: true,
  firstNoticeSeen: false,
  dwellerScroll: "shared",
}

/** Reads stored settings; unknown or broken values fall back to defaults. */
export function loadSettings(): Settings {
  let raw: any = null
  try {
    raw = JSON.parse(localStorage.getItem(KEY) ?? "null")
  } catch {
    raw = null
  }
  const s: Settings = { ...defaults }
  if (raw && typeof raw === "object") {
    for (const k of Object.keys(defaults) as (keyof Settings)[]) {
      if (k === "version") continue
      if (typeof raw[k] === typeof defaults[k]) (s as any)[k] = raw[k]
    }
  }
  if (!["system", "en", "ru"].includes(s.lang)) s.lang = "system"
  if (!["system", "light", "dark"].includes(s.theme)) s.theme = "system"
  if (!["shared", "top"].includes(s.dwellerScroll)) s.dwellerScroll = "shared"
  return s
}

export function storeSettings(s: Settings) {
  try {
    localStorage.setItem(KEY, JSON.stringify({ ...s, version: VERSION }))
  } catch {
    // Storage may be blocked; settings then last for this visit only.
  }
}

export function clearSettings() {
  try {
    localStorage.removeItem(KEY)
  } catch {
    // Nothing stored.
  }
}

export function systemLang(): "en" | "ru" {
  const l = (navigator.language || "en").toLowerCase()
  return l === "ru" || l.startsWith("ru-") ? "ru" : "en"
}

export function applyTheme(t: ThemePref) {
  const el = document.documentElement
  if (t === "system") delete el.dataset.theme
  else el.dataset.theme = t
}
