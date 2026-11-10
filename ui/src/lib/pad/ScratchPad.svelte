<script lang="ts" module>
  export type Template = "draw" | "type" | "grid" | "craft" | "takeoff" | "landing";
  export const TEMPLATES: { id: Template; label: string; hint: string }[] = [
    { id: "draw", label: "Blank", hint: "Free writing" },
    { id: "type", label: "Typed notes", hint: "Keyboard" },
    { id: "grid", label: "Grid", hint: "Squared paper" },
    { id: "craft", label: "CRAFT", hint: "Clearance" },
    { id: "takeoff", label: "Takeoff", hint: "Departure briefing" },
    { id: "landing", label: "Landing", hint: "Arrival briefing" },
  ];
</script>

<script lang="ts">
  // The scratch pad: a sheet over the map (under the map's own buttons) to
  // write on with a finger, pen or mouse, or to type. A template gives the
  // sheet its lines: blank, grid, a CRAFT clearance, a takeoff or landing
  // briefing. Each template keeps its own sheet. Kept in this browser only
  // (each device has its own pad).
  import { onMount, untrack } from "svelte";
  import { padState } from "./store.svelte";
  import { pen } from "../draw/store.svelte";
  import DrawPalette from "../draw/DrawPalette.svelte";

  interface Props {
    /** Room to leave on the left (a panel beside the map). */
    insetLeft?: number;
  }
  let { insetLeft = 0 }: Props = $props();

  type Stroke = { c: string; w: number; e?: boolean; p: number[] };
  type Page = { strokes: Stroke[]; text?: string };
  const KEY = "fd.pad";

  function load(): { t: Template; pages: Partial<Record<Template, Page>> } {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? "null");
      if (v && typeof v === "object" && v.pages) return { t: TEMPLATES.some((x) => x.id === v.t) ? v.t : "draw", pages: v.pages };
    } catch {
      /* private window or blocked storage: start empty */
    }
    return { t: "draw", pages: {} };
  }
  const saved = load();
  let template = $state<Template>(saved.t);
  let pages = $state.raw<Partial<Record<Template, Page>>>(saved.pages);
  let redo = $state.raw<Stroke[]>([]);
  let picking = $state(false);
  let small = $state(false);
  const page = $derived(pages[template] ?? { strokes: [] });
  const current = $derived(TEMPLATES.find((x) => x.id === template)!);

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  function persist() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      try {
        localStorage.setItem(KEY, JSON.stringify({ t: template, pages }));
      } catch {
        /* storage full or blocked: the pad still works for now */
      }
    }, 300);
  }
  function setPage(p: Page) {
    pages = { ...pages, [template]: p };
    persist();
  }

  // --- Drawing ---------------------------------------------------------------
  let box = $state<HTMLDivElement>();
  let canvas = $state<HTMLCanvasElement>();
  let W = 1;
  let drawing: Stroke | null = null;

  /** Points are kept as fractions of the sheet's width, so they survive a resize. */
  function draw(ctx: CanvasRenderingContext2D, s: Stroke, from = 0) {
    const p = s.p;
    if (p.length < 2) return;
    ctx.save();
    ctx.globalCompositeOperation = s.e ? "destination-out" : "source-over";
    ctx.strokeStyle = s.c;
    ctx.fillStyle = s.c;
    ctx.lineWidth = s.e ? s.w * 4 : s.w;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const X = (i: number) => p[i] * W;
    const Y = (i: number) => p[i + 1] * W;
    ctx.beginPath();
    if (p.length === 2) {
      ctx.arc(X(0), Y(0), ctx.lineWidth / 2, 0, Math.PI * 2);
      ctx.fill();
    } else {
      const start = Math.max(0, from - 2);
      ctx.moveTo(X(start), Y(start));
      // Smooth: through the midpoints, with the points as control points.
      for (let i = start + 2; i < p.length - 2; i += 2) ctx.quadraticCurveTo(X(i), Y(i), (X(i) + X(i + 2)) / 2, (Y(i) + Y(i + 2)) / 2);
      ctx.lineTo(X(p.length - 2), Y(p.length - 2));
      ctx.stroke();
    }
    ctx.restore();
  }
  function redraw() {
    const c = canvas;
    if (!c) return;
    const ctx = c.getContext("2d")!;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, c.width, c.height);
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    for (const s of page.strokes) draw(ctx, s);
  }
  function fit() {
    const b = box;
    const c = canvas;
    if (!b || !c) return;
    const dpr = window.devicePixelRatio || 1;
    W = b.clientWidth;
    c.width = Math.round(W * dpr);
    c.height = Math.round(b.clientHeight * dpr);
    redraw();
  }
  onMount(() => {
    const ro = new ResizeObserver(fit);
    if (box) ro.observe(box);
    return () => {
      ro.disconnect();
      clearTimeout(saveTimer);
    };
  });
  $effect(() => {
    void page;
    void canvas;
    untrack(redraw);
  });

  function pt(e: PointerEvent): [number, number] {
    const r = canvas!.getBoundingClientRect();
    return [(e.clientX - r.left) / W, (e.clientY - r.top) / W];
  }
  function down(e: PointerEvent) {
    if (template === "type" || e.button > 0) return;
    canvas!.setPointerCapture(e.pointerId);
    picking = false;
    drawing = { c: pen.color, w: pen.size, e: pen.erasing || undefined, p: pt(e) };
    draw(canvas!.getContext("2d")!, drawing);
    e.preventDefault();
  }
  function move(e: PointerEvent) {
    if (!drawing) return;
    const from = drawing.p.length;
    for (const ev of e.getCoalescedEvents?.() ?? [e]) drawing.p.push(...pt(ev));
    // Erasing redraws (to cut through what is there); writing adds a piece.
    if (drawing.e) {
      redraw();
      draw(canvas!.getContext("2d")!, drawing);
    } else draw(canvas!.getContext("2d")!, drawing, from);
  }
  function up() {
    if (!drawing) return;
    const s = drawing;
    drawing = null;
    redo = [];
    setPage({ ...page, strokes: [...page.strokes, s] });
  }
  function undo() {
    const s = page.strokes;
    if (!s.length) return;
    redo = [...redo, s[s.length - 1]];
    setPage({ ...page, strokes: s.slice(0, -1) });
  }
  function again() {
    if (!redo.length) return;
    const last = redo[redo.length - 1];
    redo = redo.slice(0, -1);
    setPage({ ...page, strokes: [...page.strokes, last] });
  }
  function clear() {
    redo = [];
    setPage({ strokes: [], text: "" });
  }
  function choose(t: Template) {
    template = t;
    redo = [];
    picking = false;
    persist();
  }

  // The template list closes when clicking anywhere else.
  let picker = $state<HTMLDivElement>();
  function outside(e: PointerEvent) {
    if (picking && picker && !picker.contains(e.target as Node)) picking = false;
  }
  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (picking) picking = false;
      else padState.open = false;
      return;
    }
    if (template !== "type" && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
      e.preventDefault();
      if (e.shiftKey) again();
      else undo();
    }
  }
</script>

<svelte:window onkeydown={onkey} onpointerdown={outside} />

{#snippet icon(t: Template)}
  <svg viewBox="0 0 24 24" aria-hidden="true">
    {#if t === "draw"}
      <path d="M3.5 17.5c2.2-3.6 4.1-4.6 5-2.8.9 1.8 2.2 2.2 3.9-.6" /><path d="M13.6 13.4l6-6a1.6 1.6 0 0 0-2.3-2.3l-6 6-.6 2.9z" />
    {:else if t === "type"}
      <path d="M5 7.5V5.5h9v2" /><path d="M9.5 5.5v12" /><path d="M7.5 17.5h4" /><path d="M15.5 12h4M15.5 15.5h4M15.5 19h2.5" />
    {:else if t === "grid"}
      <rect x="4" y="4" width="16" height="16" rx="2.5" /><path d="M9.3 4v16M14.7 4v16M4 9.3h16M4 14.7h16" />
    {:else if t === "craft"}
      <rect x="5" y="4" width="14" height="17" rx="2.2" /><path d="M9 4V3h6v1" /><path d="M8.5 9h1M11.5 9h4M8.5 12.5h1M11.5 12.5h4M8.5 16h1M11.5 16h3" />
    {:else}
      <path d="M3 20.5h18" opacity="0.5" />
      <g transform={t === "takeoff" ? "translate(12.5 10.5) rotate(50) scale(0.42)" : "translate(12 10.5) rotate(130) scale(0.42)"}>
        <path class="solid" d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" />
      </g>
    {/if}
  </svg>
{/snippet}

<section class="pad" class:small style:left={small ? null : `${16 + insetLeft}px`} aria-label="Scratch pad">
  <header>
    <div class="pickwrap" bind:this={picker}>
      <button class="pick" onclick={() => (picking = !picking)} aria-expanded={picking} aria-haspopup="listbox">
        <span class="ti">{@render icon(template)}</span>
        <span class="tl"><b>{current.label}</b><small>{current.hint}</small></span>
        <svg class="caret" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 4.5L6 8l3.5-3.5" /></svg>
      </button>
      {#if picking}
        <ul class="menu" role="listbox" aria-label="Sheet">
          {#each TEMPLATES as t (t.id)}
            <li>
              <button role="option" aria-selected={template === t.id} class:on={template === t.id} onclick={() => choose(t.id)}>
                <span class="mi">{@render icon(t.id)}</span>
                <span class="ml">{t.label}</span>
                <span class="mh">{t.hint}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <span class="title">Scratch pad</span>
    <button class="hb" onclick={() => (small = !small)} title={small ? "Larger" : "Smaller"} aria-label={small ? "Larger" : "Smaller"}>
      {#if small}
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></svg>
      {:else}
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" /></svg>
      {/if}
    </button>
    <button class="hb" onclick={() => (padState.open = false)} title="Close" aria-label="Close scratch pad">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </header>

  <div class="sheet" bind:this={box}>
    <div class="lines {template}" aria-hidden="true">
      {#if template === "craft"}
        {#each [["C", "Clearance limit"], ["R", "Route"], ["A", "Altitude"], ["F", "Frequency"], ["T", "Transponder"]] as [l, name] (l)}
          <div class="crow"><b>{l}</b><span>{name}</span></div>
        {/each}
      {:else if template === "takeoff" || template === "landing"}
        {#each template === "takeoff" ? ["ATIS", "Wind", "Altimeter", "Runway", "Taxi route", "Clearance", "Squawk", "Departure freq", "Notes"] : ["ATIS", "Wind", "Altimeter", "Approach", "Runway", "Minimums", "Missed approach", "Tower / Ground", "Taxi to"] as f (f)}
          <div class="cell"><span>{f}</span></div>
        {/each}
      {/if}
    </div>
    {#if template === "type"}
      <textarea class="typed" placeholder="Type here…" value={page.text ?? ""} oninput={(e) => setPage({ ...page, text: (e.currentTarget as HTMLTextAreaElement).value })}></textarea>
    {/if}
    <canvas bind:this={canvas} class:off={template === "type"} class:erase={pen.erasing} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}></canvas>
    {#if template !== "type"}
      <div class="dock">
        <DrawPalette canUndo={page.strokes.length > 0} canRedo={redo.length > 0} onundo={undo} onredo={again} onclear={clear} />
      </div>
    {:else}
      <div class="dock">
        <button class="clr" onclick={clear}>Clear</button>
      </div>
    {/if}
  </div>
</section>

<style>
  /* Fills the map under its buttons (top row stays clear); "smaller" docks
     it on the right. */
  .pad {
    position: absolute;
    top: 64px;
    right: 16px;
    bottom: 16px;
    z-index: 6;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--rule);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 18px 40px rgba(0, 0, 0, 0.5);
    color: var(--ink);
  }
  .pad.small {
    width: min(440px, calc(100% - 32px));
  }
  @media (max-width: 640px) {
    .pad {
      top: 58px;
      left: 8px !important;
      right: 8px;
      bottom: 8px;
      width: auto !important;
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 8px 8px 10px;
    border-bottom: 1px solid var(--rule);
  }
  .title {
    flex: 1;
    text-align: right;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 13px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-faint);
    padding-right: 4px;
  }
  header svg,
  .menu svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .pad :global(svg .solid) {
    fill: currentColor;
    stroke: none;
  }
  .pickwrap {
    position: relative;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 5px 10px 5px 7px;
    border-radius: 9px;
    border: 1px solid var(--rule);
    background: rgba(255, 255, 255, 0.03);
    color: var(--ink);
    cursor: pointer;
    font: inherit;
  }
  .pick:hover,
  .pick[aria-expanded="true"] {
    border-color: var(--amber);
  }
  .ti {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    background: rgba(240, 178, 58, 0.14);
    color: var(--amber);
  }
  .ti svg {
    width: 19px;
    height: 19px;
  }
  .tl {
    display: grid;
    text-align: left;
    line-height: 1.15;
  }
  .tl b {
    font-size: 13.5px;
  }
  .tl small {
    font-size: 11px;
    color: var(--ink-dim);
  }
  .caret {
    width: 12px;
    height: 12px;
    color: var(--ink-dim);
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 4;
    margin: 0;
    padding: 5px;
    list-style: none;
    min-width: 250px;
    background: var(--panel);
    border: 1px solid var(--rule);
    border-radius: 10px;
    box-shadow: 0 14px 32px rgba(0, 0, 0, 0.5);
  }
  .menu button {
    width: 100%;
    display: grid;
    grid-template-columns: 18px auto 1fr;
    align-items: center;
    gap: 10px;
    padding: 7px 9px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .menu button:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .menu button.on {
    background: rgba(240, 178, 58, 0.14);
  }
  .menu button.on .mi {
    color: var(--amber);
  }
  .mi {
    display: grid;
    color: var(--ink-dim);
  }
  .mi svg {
    width: 16px;
    height: 16px;
  }
  .ml {
    font-weight: 600;
  }
  .mh {
    justify-self: end;
    font-size: 11.5px;
    color: var(--ink-faint);
  }
  .hb {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    border: 0;
    background: transparent;
    color: var(--ink-dim);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .hb:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.05);
  }
  .hb svg {
    width: 18px;
    height: 18px;
  }
  .sheet {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: #141517;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    touch-action: none;
    cursor: crosshair;
  }
  canvas.erase {
    cursor: cell;
  }
  canvas.off {
    pointer-events: none;
  }
  .lines {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .lines.grid {
    background-image:
      linear-gradient(rgba(255, 255, 255, 0.06) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.06) 1px, transparent 1px);
    background-size: 26px 26px;
  }
  .lines.craft {
    display: grid;
    grid-template-rows: repeat(5, 1fr);
    padding-bottom: 64px;
  }
  .crow {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px dashed rgba(255, 255, 255, 0.08);
  }
  .crow b {
    font-family: var(--cond);
    font-size: 30px;
    line-height: 1;
    color: rgba(240, 178, 58, 0.35);
    width: 24px;
  }
  .crow span,
  .cell span {
    font-family: var(--cond);
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-faint);
    padding-top: 5px;
  }
  .lines.takeoff,
  .lines.landing {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(3, 1fr);
    gap: 8px;
    padding: 10px 10px 70px;
  }
  .cell {
    padding: 7px 10px;
    border-radius: 9px;
    border: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(255, 255, 255, 0.015);
  }
  .typed {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    resize: none;
    border: 0;
    outline: none;
    padding: 18px 20px 70px;
    background: transparent;
    color: var(--ink);
    font: 15px/1.6 "IBM Plex Mono", monospace;
  }
  .dock {
    position: absolute;
    left: 50%;
    bottom: 12px;
    transform: translateX(-50%);
    max-width: calc(100% - 20px);
    z-index: 2;
  }
  .clr {
    padding: 8px 16px;
    border-radius: 10px;
    border: 1px solid var(--rule);
    background: rgba(28, 30, 33, 0.96);
    color: var(--ink);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
</style>
