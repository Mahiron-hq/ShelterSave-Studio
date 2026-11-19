import { app } from "./state.svelte"

/** Events the server accepts; anything else is dropped there too. */
export type StatEvent = "view" | "open" | "save" | "error" | "desktop"

function allowed(): boolean {
  const nav = navigator as Navigator & { globalPrivacyControl?: boolean }
  if (nav.globalPrivacyControl === true) return false
  if (nav.doNotTrack === "1") return false
  return app.settings.stats
}

/** Sends an anonymous counter event. Failures are ignored on purpose. */
export function hit(event: StatEvent, detail = "") {
  if (!allowed()) return
  let ref = ""
  if (event === "view" && document.referrer) {
    try {
      const host = new URL(document.referrer).hostname
      if (host !== location.hostname) ref = host
    } catch {
      ref = ""
    }
  }
  const body = JSON.stringify({ e: event, p: location.pathname, r: ref, d: detail.slice(0, 32), l: app.lang })
  try {
    void fetch("/api/hit", {
      method: "POST",
      body,
      headers: { "content-type": "application/json" },
      keepalive: true,
      credentials: "omit",
      cache: "no-store",
    }).catch(() => {})
  } catch {
    // Counting never affects the editor.
  }
}
