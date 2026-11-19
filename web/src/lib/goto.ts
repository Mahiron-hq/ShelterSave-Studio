import type { Path } from "./core"
import { app } from "./state.svelte"
import { ui } from "./ui.svelte"
import { arr, DWELLERS, ROOMS } from "./vault"
import { pathStr } from "./core"

/** Opens the section that shows a path. */
export function goTo(path: Path) {
  const [a, b, i] = path
  if (a === "dwellers" && b === "dwellers") {
    app.section = "dwellers"
    if (typeof i === "number") {
      const id = arr(app.root, DWELLERS)[i]?.serializeId
      if (typeof id === "number") {
        ui.dw.focus = id
        ui.dw.selected = new Set([id])
      }
    }
  } else if (a === "dwellers") {
    app.section = "dwellers"
  } else if (a === "vault" && b === "rooms") {
    app.section = "vault"
    app.vaultTab = "rooms"
    if (typeof i === "number") ui.roomSel = arr(app.root, ROOMS)[i]?.deserializeID ?? null
  } else if (a === "vault") {
    app.section = "vault"
  } else {
    app.section = "other"
    if (app.expert) {
      app.otherTab = "tree"
      ui.treeSearch = pathStr(path)
    } else {
      const tabs: Record<string, string> = {
        objectiveMgr: "objectives",
        StatsWindow: "stats",
        tutorialManager: "tutorial",
        MysteriousStranger: "events",
        DeathclawManager: "events",
      }
      app.otherTab = tabs[String(a)] ?? "quests"
    }
  }
}
