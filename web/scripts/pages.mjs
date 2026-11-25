import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs"
import { join, resolve } from "node:path"

const web = resolve(import.meta.dirname, "..")
const root = resolve(web, "..")
const dist = join(web, "dist")
const SITE = process.env.SITE_URL ?? "https://shelter.mahiron.online"
const VERSION = JSON.parse(readFileSync(join(web, "package.json"), "utf8")).version
const REPO = "https://github.com/Mahiron-hq/ShelterSave-Studio"
const LANGS = ["en", "ru"]
const INDEXNOW_KEY = "094bd7d7560ecc07b80a67aad57d698f"
const AUTHOR = {
  "@type": "Person",
  "@id": `${SITE}/#author`,
  name: "Lev Burmistrov",
  alternateName: "Mahiron-hq",
  url: "https://github.com/Mahiron-hq",
}
const CRAWLERS = [
  "Googlebot",
  "Google-Extended",
  "Bingbot",
  "YandexBot",
  "DuckDuckBot",
  "Applebot",
  "Applebot-Extended",
  "GPTBot",
  "OAI-SearchBot",
  "ChatGPT-User",
  "ClaudeBot",
  "Claude-SearchBot",
  "Claude-User",
  "PerplexityBot",
  "Perplexity-User",
  "CCBot",
  "Amazonbot",
  "Meta-ExternalAgent",
  "MistralAI-User",
  "*",
]

const strings = Object.fromEntries(LANGS.map((l) => [l, JSON.parse(readFileSync(join(root, "i18n", `${l}.json`), "utf8"))]))
const t = (lang, key) => strings[lang][key] ?? key

const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;")

function inline(s) {
  let out = esc(s)
  out = out.replace(/`([^`]+)`/g, "<code>$1</code>")
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
  out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, text, href) => {
    const ext = /^https?:/.test(href)
    return `<a href="${href}"${ext ? ' rel="noopener"' : ""}>${text}</a>`
  })
  out = out.replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, '$1<a href="$2" rel="noopener">$2</a>')
  out = out.replace(/(^|[\s(])([\w.+-]+@[\w-]+\.[\w.]+)/g, '$1<a href="mailto:$2">$2</a>')
  return out
}

/** Small Markdown subset of the bundled documents: headings, paragraphs, lists, bold, code, links. */
function markdown(text) {
  const lines = text.replace(/\r/g, "").split("\n")
  const out = []
  let para = []
  let list = null
  const flush = () => {
    if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`)
    para = []
    if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i)}</li>`).join("")}</${list.tag}>`)
    list = null
  }
  for (const raw of lines) {
    const line = raw.trimEnd()
    const h = /^(#{1,3})\s+(.*)$/.exec(line)
    const ul = /^[-*]\s+(.*)$/.exec(line)
    const ol = /^\d+\.\s+(.*)$/.exec(line)
    if (h) {
      flush()
      const n = h[1].length
      const id = h[2]
        .toLowerCase()
        .replace(/[^\p{L}\p{N}]+/gu, "-")
        .replace(/^-|-$/g, "")
      out.push(`<h${n} id="${id}">${inline(h[2])}</h${n}>`)
    } else if (ul || ol) {
      if (para.length) {
        out.push(`<p>${inline(para.join(" "))}</p>`)
        para = []
      }
      const tag = ul ? "ul" : "ol"
      if (!list || list.tag !== tag) {
        if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i)}</li>`).join("")}</${list.tag}>`)
        list = { tag, items: [] }
      }
      list.items.push((ul ?? ol)[1])
    } else if (!line.trim()) {
      flush()
    } else if (list && /^\s+/.test(raw)) {
      list.items[list.items.length - 1] += " " + line.trim()
    } else {
      if (list) flush()
      para.push(line.trim())
    }
  }
  flush()
  return out.join("\n")
}

const PAGES = [
  { slug: "help", key: "page.help" },
  { slug: "desktop", key: "page.desktop" },
  { slug: "privacy", key: "page.privacy", src: (l) => join(root, "legal", `privacy.${l}.md`) },
  { slug: "terms", key: "page.terms", src: (l) => join(root, "legal", `terms.${l}.md`) },
  { slug: "disclaimer", key: "page.disclaimer", src: (l) => join(root, "legal", `disclaimer.${l}.md`) },
  { slug: "storage", key: "page.storage", src: (l) => join(root, "legal", `storage.${l}.md`) },
  { slug: "license", key: "page.license" },
  { slug: "contacts", key: "page.contacts" },
]
const LEGAL = ["privacy", "terms", "disclaimer", "storage", "license", "contacts"]

const assets = readdirSync(join(dist, "assets"))
const appCss = assets.find((f) => /^index-.*\.css$/.test(f))
if (!appCss) throw new Error("vite build output not found")

function alternates(path) {
  return [
    ...LANGS.map((l) => `<link rel="alternate" hreflang="${l}" href="${SITE}/${l}/${path}">`),
    `<link rel="alternate" hreflang="x-default" href="${SITE}/${path ? "en/" + path : ""}">`,
  ].join("\n    ")
}

function social(lang, path, title, description) {
  const other = lang === "ru" ? "en" : "ru"
  const locale = (l) => (l === "ru" ? "ru_RU" : "en_US")
  return [
    `<meta property="og:type" content="website">`,
    `<meta property="og:site_name" content="ShelterSave-Studio">`,
    `<meta property="og:title" content="${esc(title)}">`,
    `<meta property="og:description" content="${esc(description)}">`,
    `<meta property="og:url" content="${SITE}/${lang}/${path}">`,
    `<meta property="og:locale" content="${locale(lang)}">`,
    `<meta property="og:locale:alternate" content="${locale(other)}">`,
    `<meta property="og:image" content="${SITE}/og-image.png">`,
    `<meta property="og:image:type" content="image/png">`,
    `<meta property="og:image:width" content="1200">`,
    `<meta property="og:image:height" content="630">`,
    `<meta property="og:image:alt" content="${esc(t(lang, "page.editorTitle"))}">`,
    `<meta name="twitter:card" content="summary_large_image">`,
  ].join("\n    ")
}

function jsonLd(data) {
  const json = JSON.stringify({ "@context": "https://schema.org", ...data }).replace(/</g, "\\u003c")
  return `<script type="application/ld+json">${json}</script>`
}

function appData(lang, type, extra) {
  return {
    "@type": type,
    name: "ShelterSave-Studio",
    description: t(lang, type === "WebApplication" ? "page.editorDesc" : "page.desktopDesc"),
    inLanguage: lang,
    applicationCategory: "UtilitiesApplication",
    softwareVersion: VERSION,
    isAccessibleForFree: true,
    offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
    license: "https://www.gnu.org/licenses/gpl-3.0.html",
    author: { "@id": `${SITE}/#author` },
    sameAs: [REPO],
    image: `${SITE}/og-image.png`,
    ...extra,
  }
}

function header(lang, current) {
  const other = lang === "ru" ? "en" : "ru"
  const nav = [
    ["", "page.editor"],
    ["help/", "page.help"],
    ["desktop/", "page.desktop"],
  ]
    .map(([p, k]) => `<a href="/${lang}/${p}"${current === p ? ' aria-current="page"' : ""}>${esc(t(lang, k))}</a>`)
    .join("")
  return `<header class="site-head">
  <a class="brand" href="/${lang}/">ShelterSave-Studio</a>
  <nav aria-label="${esc(t(lang, "page.nav"))}">${nav}</nav>
  <a class="lang" href="/${other}/${current}" hreflang="${other}" lang="${other}">${other === "ru" ? "Русский" : "English"}</a>
</header>`
}

function footer(lang) {
  const links = LEGAL.map((s) => `<a href="/${lang}/${s}/">${esc(t(lang, `page.${s}`))}</a>`).join(" · ")
  return `<footer class="site-foot">
  <p>Copyright (C) 2026 Lev Burmistrov (Mahiron-hq) · ${esc(t(lang, "about.version").replace("{v}", VERSION))} · <a href="${REPO}" rel="noopener">GitHub</a> · <a href="${REPO}/blob/main/LICENSE" rel="noopener">GPL-3.0</a></p>
  <p>${links}</p>
  <p class="muted">${esc(t(lang, "app.unaffiliated"))}</p>
</footer>`
}

function page(lang, path, title, description, body, { current = path, noindex = false, data = null } = {}) {
  return `<!doctype html>
<html lang="${lang}">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>${esc(title)}</title>
    <meta name="description" content="${esc(description)}">
    ${noindex ? '<meta name="robots" content="noindex">' : `<link rel="canonical" href="${SITE}/${lang}/${path}">\n    ${alternates(path)}\n    ${social(lang, path, title, description)}`}${data ? `\n    ${jsonLd(data)}` : ""}
    <link rel="icon" href="/favicon.svg" type="image/svg+xml">
    <link rel="stylesheet" href="/assets/${appCss}">
    <link rel="stylesheet" href="/assets/site.css">
    <script src="/assets/page.js" defer></script>
  </head>
  <body class="doc">
    <a class="skip" href="#main">${esc(t(lang, "web.skip"))}</a>
    ${header(lang, current)}
    <main id="main" class="doc-main">
${body}
    </main>
    ${footer(lang)}
  </body>
</html>
`
}

function write(rel, html) {
  const file = join(dist, rel)
  mkdirSync(resolve(file, ".."), { recursive: true })
  writeFileSync(file, html)
}

// Editor pages: the Vite shell with the language, texts and a description the app replaces on start.
const shell = readFileSync(join(dist, "index.html"), "utf8")
for (const lang of LANGS) {
  const title = t(lang, "page.editorTitle")
  const description = t(lang, "page.editorDesc")
  const data = {
    "@graph": [
      {
        "@type": "WebSite",
        "@id": `${SITE}/#website`,
        url: `${SITE}/`,
        name: "ShelterSave-Studio",
        inLanguage: LANGS,
        publisher: { "@id": `${SITE}/#author` },
      },
      appData(lang, "WebApplication", {
        url: `${SITE}/${lang}/`,
        operatingSystem: "Any",
        browserRequirements: "Requires JavaScript and WebAssembly",
      }),
      AUTHOR,
    ],
  }
  const head = [
    `<title>${esc(title)}</title>`,
    `<meta name="description" content="${esc(description)}">`,
    `<link rel="canonical" href="${SITE}/${lang}/">`,
    alternates(""),
    social(lang, "", title, description),
    `<link rel="stylesheet" href="/assets/site.css">`,
    jsonLd(data),
  ].join("\n    ")
  const intro = markdown(readFileSync(join(web, "pages", `intro.${lang}.md`), "utf8"))
  const noscript = `<noscript><p>${esc(t(lang, "page.noscript"))}</p></noscript>`
  const html = shell
    .replace('<html lang="en">', `<html lang="${lang}">`)
    .replace(/<title>[^<]*<\/title>/, head)
    .replace('<div id="app"></div>', `<div id="app"><main class="doc-main">\n${intro}\n${noscript}\n</main></div>`)
  write(`${lang}/index.html`, html)
}
rmSync(join(dist, "index.html"))

for (const lang of LANGS) {
  for (const p of PAGES) {
    const src = p.src ? p.src(lang) : join(web, "pages", `${p.slug}.${lang}.md`)
    if (!existsSync(src)) throw new Error(`missing ${src}`)
    const md = readFileSync(src, "utf8")
    const title = `${t(lang, p.key)} – ShelterSave-Studio`
    const data =
      p.slug === "desktop"
        ? {
            "@graph": [
              appData(lang, "SoftwareApplication", {
                url: `${SITE}/${lang}/desktop/`,
                operatingSystem: "Windows 10, Windows 11, Linux, macOS",
                downloadUrl: `${REPO}/releases/latest`,
              }),
              AUTHOR,
            ],
          }
        : null
    write(`${lang}/${p.slug}/index.html`, page(lang, `${p.slug}/`, title, t(lang, `${p.key}Desc`), markdown(md), { data }))
  }
  const nf = markdown(readFileSync(join(web, "pages", `404.${lang}.md`), "utf8"))
  write(`${lang}/404.html`, page(lang, "", `${t(lang, "page.404")} – ShelterSave-Studio`, t(lang, "page.404"), nf, { current: "", noindex: true }))
}

const urls = ["", ...PAGES.map((p) => `${p.slug}/`)]
const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${urls
  .flatMap((u) =>
    LANGS.map(
      (l) =>
        `  <url><loc>${SITE}/${l}/${u}</loc>${LANGS.map((a) => `<xhtml:link rel="alternate" hreflang="${a}" href="${SITE}/${a}/${u}"/>`).join("")}</url>`,
    ),
  )
  .join("\n")}
</urlset>
`
write("sitemap.xml", sitemap)
write(
  "robots.txt",
  `${CRAWLERS.map((c) => `User-agent: ${c}`).join("\n")}\nAllow: /\nDisallow: /api/\n\nSitemap: ${SITE}/sitemap.xml\n`,
)
write(`${INDEXNOW_KEY}.txt`, INDEXNOW_KEY)

const llmsLinks = (lang) =>
  [["", "page.editor", "page.editorDesc"], ...PAGES.map((p) => [`${p.slug}/`, p.key, `${p.key}Desc`])]
    .map(([u, k, d]) => `- [${t(lang, k)}](${SITE}/${lang}/${u}): ${t(lang, d)}`)
    .join("\n")
write(
  "llms.txt",
  `# ShelterSave-Studio

> ${t("en", "page.editorDesc")} Also available as a desktop program for Windows, Linux and macOS. GPL-3.0, author Lev Burmistrov (Mahiron-hq).

ShelterSave-Studio edits Fallout Shelter save files (Vault1.sav, Vault2.sav, Vault3.sav): resources, storage, rooms and the vault map, dwellers, pets, quests, objectives, achievements, wasteland teams, incidents and timers. Its rules come from the game's own data and code (Fallout Shelter 2.7), and an integrity check finds problems that make the game crash or hang. The website decrypts and edits the file in the browser and never uploads it. It is an independent project, not affiliated with Bethesda Softworks.

## English

${llmsLinks("en")}

## Русский

${llmsLinks("ru")}

## Source

- [GitHub repository](${REPO}): source code, README, changelog
- [Releases](${REPO}/releases/latest): desktop builds with SHA-256 checksums
`,
)
const expires = new Date(Date.now() + 365 * 24 * 3600 * 1000).toISOString().replace(/\.\d+Z$/, ".000Z")
write(
  ".well-known/security.txt",
  `Contact: mailto:Mahiron.HQ@protonmail.com\nExpires: ${expires}\nPreferred-Languages: en, ru\nCanonical: ${SITE}/.well-known/security.txt\nPolicy: ${REPO}/blob/main/SECURITY.md\n`,
)
console.log(`pages: ${LANGS.length * (PAGES.length + 2)} html files`)
