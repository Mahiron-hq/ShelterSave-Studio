import { execFileSync } from "node:child_process"
import { existsSync, mkdirSync, renameSync } from "node:fs"
import { join, resolve } from "node:path"

const root = resolve(import.meta.dirname, "../..")
const out = resolve(import.meta.dirname, "../src/wasm")
const run = (cmd, args) => execFileSync(cmd, args, { stdio: "inherit", cwd: root })

run("cargo", ["build", "-p", "shelter-wasm", "--target", "wasm32-unknown-unknown", "--profile", "wasm"])
mkdirSync(out, { recursive: true })
run("wasm-bindgen", [
  "--target", "web", "--out-dir", out, "--no-typescript",
  join(root, "target/wasm32-unknown-unknown/wasm/shelter_wasm.wasm"),
])

const local = join(process.env.LOCALAPPDATA ?? "", "binaryen/binaryen-version_133/bin/wasm-opt.exe")
const wasmOpt = process.env.WASM_OPT ?? (existsSync(local) ? local : "wasm-opt")
const file = join(out, "shelter_wasm_bg.wasm")
run(wasmOpt, [
  "-Oz", "--strip-debug", "--strip-producers",
  "--enable-bulk-memory", "--enable-nontrapping-float-to-int", "--enable-sign-ext",
  "--enable-mutable-globals", "--enable-reference-types", "--enable-multivalue",
  file, "-o", file + ".opt",
])
renameSync(file + ".opt", file)
