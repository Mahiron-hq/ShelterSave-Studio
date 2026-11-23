// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
// SPDX-License-Identifier: GPL-3.0-or-later
// Writes THIRD_PARTY.md from cargo metadata and the bundled assets.
import { execFileSync } from "node:child_process";
import { writeFileSync, readFileSync, existsSync } from "node:fs";

const meta = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], { maxBuffer: 1 << 28 }).toString());
const own = new Set(meta.workspace_members);
const lines = ["# Third-party components", "", "ShelterSave-Studio uses these components. Each keeps its own license.", "", "## Fonts and icons", "",
  "- Inter 4.1 — SIL Open Font License 1.1 — https://rsms.me/inter/",
  "- JetBrains Mono 2.304 — SIL Open Font License 1.1 — https://www.jetbrains.com/lp/mono/",
  "- Lucide icons 0.468.0 — ISC License — https://lucide.dev", ""];
const web = "web/package.json";
if (existsSync(web)) {
  const lock = JSON.parse(readFileSync("web/package-lock.json", "utf8"));
  const shipped = ["@ui-sdk/core", "@ui-sdk/svelte", "svelte"];
  lines.push("## Website (JavaScript)", "", "Included in the website:", "");
  for (const n of shipped) {
    const p = lock.packages["node_modules/" + n] || {};
    lines.push(`- ${n} ${p.version || ""} — ${p.license || "see package"}`);
  }
  lines.push("", "Used only to build and check it:", "");
  const names = Object.keys(JSON.parse(readFileSync(web, "utf8")).devDependencies || {}).filter(n => !shipped.includes(n) && n !== "lucide-static").sort();
  for (const n of names) {
    const p = lock.packages["node_modules/" + n] || {};
    lines.push(`- ${n} ${p.version || ""} — ${p.license || "see package"}`);
  }
  lines.push("");
}
lines.push("## Rust crates", "");
const rows = meta.packages.filter(p => !own.has(p.id)).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
for (const p of rows) lines.push(`- ${p.name} ${p.version} — ${p.license || "see crate"}`);
writeFileSync("THIRD_PARTY.md", lines.join("\n") + "\n");
console.log(rows.length, "crates");
