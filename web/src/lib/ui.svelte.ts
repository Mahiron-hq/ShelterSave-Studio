/** View state of the sections that is kept while switching between them. */
class Ui {
  dw = $state({
    search: "",
    filter: 0,
    sort: 0,
    desc: false,
    selected: new Set<number>(),
    focus: null as number | null,
  })
  st = $state({
    search: "",
    kind: null as string | null,
    rarity: null as string | null,
    selected: new Set<string>(),
  })
  roomSel = $state<number | null>(null)
  /** Expert mode: built or moved rooms do not merge with equal neighbours. */
  keepApart = $state(false)
  rockSel = $state<[number, number] | null>(null)
  treeSearch = $state("")

  reset() {
    this.dw = { search: "", filter: 0, sort: 0, desc: false, selected: new Set(), focus: null }
    this.st = { search: "", kind: null, rarity: null, selected: new Set() }
    this.roomSel = null
    this.rockSel = null
    this.treeSearch = ""
  }
}

export const ui = new Ui()
