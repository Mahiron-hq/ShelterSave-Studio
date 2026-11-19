import { app } from "./state.svelte"

/** Number without grouping: whole values without decimals, others with up to two. */
export function plainNum(v: number): string {
  if (Number.isInteger(v) && Math.abs(v) < 1e15) return String(v)
  return v.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")
}

/** Whole numbers grouped by thousands with a narrow space, others with two decimals. */
export function fmtNum(v: number): string {
  if (Number.isInteger(v) && Math.abs(v) < 1e15) {
    const s = String(Math.abs(v))
    const sign = v < 0 ? "-" : ""
    if (s.length <= 4) return sign + s
    return sign + s.replace(/\B(?=(\d{3})+(?!\d))/g, " ")
  }
  return v.toFixed(2)
}

/** Short text for any JSON value. */
export function short(v: unknown): string {
  if (typeof v === "string") return v
  if (typeof v === "boolean") return String(v)
  if (Array.isArray(v)) return `[${v.length}]`
  if (v && typeof v === "object") return `{${Object.keys(v).length}}`
  if (v === null || v === undefined) return ""
  return String(v)
}

export function hexColor(n: number): string {
  return "#" + ((n >>> 0) & 0xffffff).toString(16).toUpperCase().padStart(6, "0")
}


/** "2 h 5 min" or "5 min". */
export function duration(secs: number): string {
  const total = Math.max(0, Math.floor(secs))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  return h > 0 ? app.t("o.hm", { h, m }) : app.t("o.m", { m })
}
