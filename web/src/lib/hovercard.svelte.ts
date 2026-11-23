// One floating card for items and themes. Moving the pointer from one name to
// another switches the card at once; the card never takes the pointer, so it
// cannot hide the next name in a list.
import type { ItemKind } from "./core"

export type CardSubject = { kind: ItemKind; id: string } | { theme: string }

const OPEN_DELAY = 300
const CLOSE_DELAY = 150

class HoverCard {
  subject = $state<CardSubject | null>(null)
  anchor = $state<DOMRect | null>(null)
  private openTimer = 0
  private closeTimer = 0

  enter(subject: CardSubject, el: HTMLElement) {
    clearTimeout(this.closeTimer)
    clearTimeout(this.openTimer)
    const show = () => {
      this.subject = subject
      this.anchor = el.getBoundingClientRect()
    }
    if (this.subject) show()
    else this.openTimer = window.setTimeout(show, OPEN_DELAY)
  }

  leave() {
    clearTimeout(this.openTimer)
    clearTimeout(this.closeTimer)
    this.closeTimer = window.setTimeout(() => this.hide(), CLOSE_DELAY)
  }

  toggle(subject: CardSubject, el: HTMLElement) {
    if (this.subject && JSON.stringify(this.subject) === JSON.stringify(subject)) this.hide()
    else {
      clearTimeout(this.openTimer)
      this.subject = subject
      this.anchor = el.getBoundingClientRect()
    }
  }

  hide() {
    clearTimeout(this.openTimer)
    clearTimeout(this.closeTimer)
    this.subject = null
    this.anchor = null
  }
}

export const hoverCard = new HoverCard()
