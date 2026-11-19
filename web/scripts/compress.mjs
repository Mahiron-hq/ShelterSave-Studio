import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs"
import { join, resolve } from "node:path"
import { brotliCompressSync, constants, gzipSync } from "node:zlib"

const dist = resolve(import.meta.dirname, "../dist")
const TEXT = /\.(html|js|mjs|css|wasm|json|svg|xml|txt|woff2)$/

let raw = 0
let br = 0
function walk(dir) {
  for (const name of readdirSync(dir)) {
    const file = join(dir, name)
    if (statSync(file).isDirectory()) {
      walk(file)
      continue
    }
    if (!TEXT.test(name) || name.endsWith(".woff2")) continue
    const data = readFileSync(file)
    const b = brotliCompressSync(data, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: data.length },
    })
    const g = gzipSync(data, { level: 9 })
    if (b.length < data.length) writeFileSync(file + ".br", b)
    if (g.length < data.length) writeFileSync(file + ".gz", g)
    raw += data.length
    br += Math.min(b.length, data.length)
  }
}
walk(dist)
console.log(`compressed: ${(raw / 1024).toFixed(0)} KB -> ${(br / 1024).toFixed(0)} KB brotli`)
