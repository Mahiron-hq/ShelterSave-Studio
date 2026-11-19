// The wheel over a value changes it only after the pointer was moved onto it,
// so a page scroll that brings a field under a resting pointer keeps scrolling.
let lastMove = 0
let lastPageScroll = 0
let started = false

function start() {
  if (started) return
  started = true
  window.addEventListener(
    "pointermove",
    (e) => {
      if (e.movementX !== 0 || e.movementY !== 0) lastMove = performance.now()
    },
    { passive: true },
  )
  window.addEventListener(
    "wheel",
    (e) => {
      if (!e.defaultPrevented) lastPageScroll = performance.now()
    },
    { passive: true },
  )
}

const acc = new WeakMap<EventTarget, number>()

/** Whole wheel notches for a value widget; 0 leaves the event to the page. */
export function wheelSteps(e: WheelEvent): number {
  start()
  if (lastMove <= lastPageScroll || !e.currentTarget) return 0
  e.preventDefault()
  // Shift may turn vertical wheel motion into horizontal.
  const d = e.deltaY !== 0 ? e.deltaY : e.deltaX
  const notch = e.deltaMode === 1 ? d / 3 : e.deltaMode === 2 ? d : d / 100
  const sum = (acc.get(e.currentTarget) ?? 0) - notch
  const n = Math.trunc(sum)
  acc.set(e.currentTarget, sum - n)
  return n
}

export function trackPointer() {
  start()
}

const MAX_STEP = 60
const STEP_PX = 6
const FAST_SPEED = 700

/** Fast step: 60, but at most a fiftieth of the range and at least one. */
export function fastStep(span: number): number {
  return Number.isFinite(span) ? Math.min(MAX_STEP, Math.max(1, Math.floor(span / 50))) : MAX_STEP
}

/** Units per pixel of travel: slow moves step by one, fast moves (or Shift) by fastStep. */
export function dragRate(speed: number, fast: boolean, span: number): number {
  return (fast || speed > FAST_SPEED ? fastStep(span) : 1) / STEP_PX
}
