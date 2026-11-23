// Addresses of editor sections: /ru/vault/rooms, /en/dwellers and so on.
// Only the place in the editor is in the address, never the save itself.
import { app, type Section } from "./state.svelte"

export const VAULT_TABS = ["resources", "storage", "rooms", "themes", "book"]
export const OTHER_TABS = ["quests", "wasteland", "objectives", "stats", "tutorial", "events", "service", "tree"]

interface Place {
  section: Section
  tab?: string
}

function parse(path: string): Place | null {
  const parts = path.split("/").filter(Boolean).slice(1)
  const [section, tab] = parts
  if (section === "save" || section === "dwellers") return parts.length === 1 ? { section } : null
  if (section === "vault" && parts.length <= 2) return { section, tab: VAULT_TABS.includes(tab) ? tab : "resources" }
  if (section === "other" && parts.length <= 2) return { section, tab: OTHER_TABS.includes(tab) ? tab : "quests" }
  return null
}

function current(): Place {
  const section = app.section
  if (section === "vault") return { section, tab: app.vaultTab }
  if (section === "other") return { section, tab: app.otherTab }
  return { section }
}

function pathOf(p: Place | null): string {
  const base = `/${app.lang}/`
  if (!p) return base
  return base + (p.tab ? `${p.section}/${p.tab}` : p.section)
}

function apply(p: Place) {
  app.section = p.section
  if (p.section === "vault" && p.tab) app.vaultTab = p.tab
  if (p.section === "other" && p.tab) app.otherTab = p.tab
}

function title(p: Place | null): string {
  const base = app.t("page.editorTitle")
  if (!p) return base
  const parts = [app.t(`nav.${p.section}`)]
  if (p.section === "vault" && p.tab) parts.push(app.t(`vault.tab.${p.tab}`))
  if (p.section === "other" && p.tab) parts.push(app.t(`o.${p.tab}`))
  return `${parts.join(" · ")} – ShelterSave-Studio`
}

/** Place asked for by the address the page was opened with, applied once a file is open. */
let wanted: Place | null = null
let applying = false

export function startRouting() {
  wanted = parse(location.pathname)
  addEventListener("popstate", () => {
    const p = parse(location.pathname)
    if (!p) return
    if (app.isOpen) {
      applying = true
      apply(p)
      applying = false
    } else wanted = p
  })
}

/** Called whenever the open file, section or tab changes. */
export function syncRoute() {
  if (!app.isOpen) {
    const keep = wanted ? pathOf(wanted) : `/${app.lang}/`
    if (location.pathname !== keep) history.replaceState(null, "", keep)
    document.title = title(null)
    return
  }
  if (wanted) {
    apply(wanted)
    wanted = null
  }
  const p = current()
  const path = pathOf(p)
  if (location.pathname !== path) {
    if (applying || location.pathname === `/${app.lang}/`) history.replaceState(null, "", path)
    else history.pushState(null, "", path)
  }
  document.title = title(p)
}
