import { SvelteSet } from "svelte/reactivity"

/** Open branches of the JSON tree, by path text; kept while switching tabs. */
export const treeOpen = new SvelteSet<string>()

/** Fields of the "add key / element" row, shared like in the desktop app. */
export const treeAdd = $state({ key: "", value: "" })
