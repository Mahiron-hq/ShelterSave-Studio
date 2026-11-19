import { defineConfig } from "vite"
import { svelte } from "@sveltejs/vite-plugin-svelte"

export default defineConfig({
  plugins: [svelte()],
  build: {
    target: "es2022",
    assetsInlineLimit: 0,
    modulePreload: { polyfill: false },
  },
  server: { fs: { allow: [".."] } },
})
