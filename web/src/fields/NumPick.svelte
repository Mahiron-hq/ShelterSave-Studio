<script lang="ts">
  import { Slider, SliderControl, SliderIndicator, SliderThumb, SliderTrack } from "@ui-sdk/svelte/slider"
  import { tick } from "svelte"
  import Icon from "../components/Icon.svelte"
  import { fmtNum, plainNum } from "../lib/format"
  import { dragRate, fastStep, trackPointer, wheelSteps } from "../lib/wheel"

  interface Props {
    value: number
    label: string
    /** Allowed range of the box (undefined = open). */
    lo?: number
    hi?: number
    /** Range of the slider; no slider without it. */
    slider?: [number, number] | null
    int?: boolean
    oncommit: (v: number) => void
  }
  let { value, label, lo, hi, slider = null, int = true, oncommit }: Props = $props()

  const min = $derived(lo ?? -Infinity)
  const max = $derived(hi ?? Infinity)
  const span = $derived(lo !== undefined && hi !== undefined ? hi - lo : Infinity)
  const cubic = $derived(slider ? slider[1] - slider[0] > 1000 : false)

  let draft = $state<number | null>(null)
  let typing = $state<string | null>(null)
  let input = $state<HTMLInputElement>()
  let wheelTimer: ReturnType<typeof setTimeout> | undefined
  let drag: { id: number; start: number; acc: number; x: number; t: number; speed: number; moved: number } | null =
    null

  const shown = $derived(draft ?? value)
  const width = $derived(Math.min(18, Math.max(7, fmtNum(shown).length + 3)))

  function clamp(v: number): number {
    return Math.min(max, Math.max(min, v))
  }

  function commit(v: number | null) {
    draft = null
    if (v === null || !Number.isFinite(v)) return
    const n = int ? Math.round(v) : v
    if (n !== value) oncommit(n)
  }

  // A wheel gesture is one history step: it is committed after a short pause.
  function onWheel(e: WheelEvent) {
    const n = wheelSteps(e)
    if (n === 0) return
    draft = clamp((draft ?? value) + n)
    clearTimeout(wheelTimer)
    wheelTimer = setTimeout(() => commit(draft), 500)
  }

  function wheel(node: HTMLElement) {
    trackPointer()
    node.addEventListener("wheel", onWheel, { passive: false })
    return { destroy: () => node.removeEventListener("wheel", onWheel) }
  }

  function down(e: PointerEvent) {
    if (e.button !== 0) return
    e.preventDefault()
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    drag = { id: e.pointerId, start: draft ?? value, acc: 0, x: e.clientX, t: e.timeStamp, speed: 0, moved: 0 }
  }

  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return
    const dx = e.clientX - drag.x
    const dt = Math.max(1, e.timeStamp - drag.t)
    drag.speed = drag.speed * 0.6 + (Math.abs(dx) / dt) * 1000 * 0.4
    drag.x = e.clientX
    drag.t = e.timeStamp
    drag.moved += Math.abs(dx)
    if (drag.moved < 3) return
    // The travel is clamped too, so moving back from past an end responds at once.
    drag.acc = Math.min(max - drag.start, Math.max(min - drag.start, drag.acc + dx * dragRate(drag.speed, e.shiftKey, span)))
    draft = clamp(drag.start + Math.trunc(drag.acc))
  }

  async function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return
    const moved = drag.moved >= 3
    drag = null
    if (moved) commit(draft)
    else await startTyping()
  }

  async function startTyping() {
    draft = null
    typing = int ? String(Math.round(value)) : plainNum(value)
    await tick()
    input?.focus()
    input?.select()
  }

  function parse(t: string): number | null {
    const n = Number(t.replace(/[\s  ]/g, "").replace(",", "."))
    return t.trim() !== "" && Number.isFinite(n) ? n : null
  }

  function endTyping(save: boolean) {
    if (typing === null) return
    const n = parse(typing)
    typing = null
    if (save && n !== null) commit(clamp(int ? Math.round(n) : Math.round(n * 100) / 100))
  }

  function typingKey(e: KeyboardEvent) {
    if (e.key === "Enter") endTyping(true)
    else if (e.key === "Escape") endTyping(false)
    else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault()
      const base = parse(typing ?? "") ?? value
      const n = clamp(base + (e.key === "ArrowUp" ? 1 : -1))
      typing = int ? String(Math.round(n)) : plainNum(n)
    }
  }

  function boxKey(e: KeyboardEvent) {
    const fast = fastStep(span)
    const steps: Record<string, number> = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1, PageUp: fast, PageDown: -fast }
    if (e.key === "Enter" || e.key === " " || e.key === "F2") {
      e.preventDefault()
      void startTyping()
    } else if (e.key in steps) {
      e.preventDefault()
      commit(clamp(value + steps[e.key] * (e.shiftKey ? fast : 1)))
    } else if (e.key === "Home" && lo !== undefined) {
      e.preventDefault()
      commit(lo)
    } else if (e.key === "End" && hi !== undefined) {
      e.preventDefault()
      commit(hi)
    }
  }

  function first(v: number | readonly number[]): number {
    return typeof v === "number" ? v : (v[0] ?? 0)
  }
  // Big ranges use a cubic scale over 0..1000 so small values stay easy to pick; steps stay whole from the current value.
  const sMin = $derived(cubic || !slider ? 0 : slider[0])
  const sMax = $derived(cubic || !slider ? 1000 : slider[1])
  function toT(v: number): number {
    if (!slider) return 0
    const c = Math.min(Math.max(v, slider[0]), slider[1])
    if (!cubic) return c
    return Math.round(Math.cbrt((c - slider[0]) / (slider[1] - slider[0] || 1)) * 1000)
  }
  function fromT(t: number): number {
    if (!slider) return value
    const x = t / 1000
    const v = cubic ? slider[0] + (slider[1] - slider[0]) * x * x * x : t
    return clamp(value + Math.round(v - value))
  }
</script>

{#if slider && slider[1] > slider[0]}
  <div class="slider" use:wheel>
    <Slider
      value={[toT(shown)]}
      min={sMin}
      max={sMax}
      onValueChange={(v) => (draft = fromT(first(v)))}
      onValueCommit={(v) => commit(fromT(first(v)))}
    >
      <SliderControl>
        <SliderTrack>
          <SliderIndicator />
          <SliderThumb aria-label={label} aria-valuetext={fmtNum(shown)} />
        </SliderTrack>
      </SliderControl>
    </Slider>
  </div>
{/if}
{#if typing !== null}
  <input
    bind:this={input}
    class="num-box mono typing"
    style:width="{width + 2}ch"
    aria-label={label}
    inputmode={int ? "numeric" : "decimal"}
    class:invalid={parse(typing) === null}
    bind:value={typing}
    onkeydown={typingKey}
    onblur={() => endTyping(true)}
  />
{:else}
  <span
    class="num-box mono"
    style:width="{width}ch"
    role="spinbutton"
    tabindex="0"
    aria-label={label}
    aria-valuenow={shown}
    aria-valuemin={lo}
    aria-valuemax={hi}
    aria-valuetext={fmtNum(shown)}
    use:wheel
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => {
      drag = null
      draft = null
    }}
    onkeydown={boxKey}
  >
    <Icon name="sort" />
    <span class="v">{fmtNum(shown)}</span>
  </span>
{/if}

<style>
  .slider {
    display: inline-block;
    width: clamp(80px, 30vw, 200px);
    vertical-align: middle;
  }
  .num-box {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    box-sizing: content-box;
    min-height: 30px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--text);
    cursor: ew-resize;
    user-select: none;
    touch-action: none;
    vertical-align: middle;
  }
  .num-box :global(svg) {
    transform: rotate(90deg);
    color: var(--muted);
    flex: none;
  }
  .num-box:focus-visible {
    outline: 2px solid var(--focus, var(--accent));
    outline-offset: 1px;
  }
  .typing {
    cursor: text;
    text-align: center;
    user-select: text;
    font: inherit;
  }
  .invalid {
    border-color: var(--danger);
  }
</style>
