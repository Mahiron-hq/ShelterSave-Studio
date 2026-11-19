/** New room form on the map: the clicked cell and the chosen room. */
export interface MapAdd {
  row: number
  at: number
  kind: string
  merge: number
  level: number
  /** Place rocks instead of a room. */
  rock: boolean
  /** Opened as a copy of an existing room. */
  copy: boolean
}

class RoomsUi {
  add = $state<MapAdd | null>(null)
  /** Type, size and level of the last thing built, offered again on the next empty cell. */
  last = $state<MapAdd | null>(null)
}

export const rooms = new RoomsUi()
