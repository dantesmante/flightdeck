<script lang="ts" module>
  export type Template = "draw" | "type" | "grid" | "craft" | "takeoff" | "landing";
  export const TEMPLATES: { id: Template; label: string }[] = [
    { id: "draw", label: "Draw" },
    { id: "type", label: "Type" },
    { id: "grid", label: "Grid" },
    { id: "craft", label: "CRAFT" },
    { id: "takeoff", label: "Takeoff" },
    { id: "landing", label: "Landing" },
  ];
</script>

<script lang="ts">
  // The scratch pad: write with a finger, pen or mouse on a blank page, a
  // grid, a CRAFT clearance or a takeoff or landing briefing, or type. Each
  // template keeps its own page. Kept in this browser only (each device has
  // its own pad).
  import { onMount, untrack } from "svelte";
  import { padState } from "./store.svelte";

  type Stroke = { c: string; w: number; e?: boolean; p: number[] };
  type Page = { strokes: Stroke[]; text?: string };

  const COLORS = ["#f2f4f7", "#f0b23a", "#5cc8e8", "#ff5a5f", "#5fd38d"];
  const SIZES = [2, 4, 8];
  const KEY = "fd.pad";

  function load(): { t: Template; pages: Partial<Record<Template, Page>> } {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? "null");
      if (v && typeof v === "object" && v.pages) return v;
    } catch {
      /* private window or blocked storage: start empty */
    }
    return { t: "draw", pages: {} };
  }
  const saved = load();
  let template = $state<Template>(saved.t);
  let pages = $state.raw<Partial<Record<Template, Page>>>(saved.pages);
  let redo = $state.raw<Stroke[]>([]);
  let color = $state(COLORS[0]);
  let size = $state(SIZES[0]);
  let erasing = $state(false);
  let picking = $state(false);
  let styleOpen = $state(false);
  let full = $state(false);

  const page = $derived(pages[template] ?? { strokes: [] });

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
  let H = 1;
  let current: Stroke | null = null;

  /** Points are kept as fractions of the page width, so they survive a resize. */
  function draw(ctx: CanvasRenderingContext2D, s: Stroke, from = 0) {
    const p = s.p;
    if (p.length < 2) return;
    ctx.save();
    ctx.globalCompositeOperation = s.e ? "destination-out" : "source-over";
    ctx.strokeStyle = s.c;
    ctx.fillStyle = s.c;
    ctx.lineWidth = s.e ? s.w * 5 : s.w;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const X = (i: number) => p[i] * W;
    const Y = (i: number) => p[i + 1] * W;
    if (p.length === 2) {
      ctx.beginPath();
      ctx.arc(X(0), Y(0), ctx.lineWidth / 2, 0, Math.PI * 2);
      ctx.fill();
    } else {
      ctx.beginPath();
      const start = Math.max(0, from - 2);
      ctx.moveTo(X(start), Y(start));
      // Smooth: through the midpoints, with the points as control points.
      for (let i = start + 2; i < p.length - 2; i += 2) {
        const mx = (X(i) + X(i + 2)) / 2;
        const my = (Y(i) + Y(i + 2)) / 2;
        ctx.quadraticCurveTo(X(i), Y(i), mx, my);
      }
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
    H = b.clientHeight;
    c.width = Math.round(W * dpr);
    c.height = Math.round(H * dpr);
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
    styleOpen = false;
    current = { c: color, w: size, e: erasing || undefined, p: pt(e) };
    const ctx = canvas!.getContext("2d")!;
    draw(ctx, current);
    e.preventDefault();
  }
  function move(e: PointerEvent) {
    if (!current) return;
    const events = e.getCoalescedEvents?.() ?? [e];
    const from = current.p.length;
    for (const ev of events) current.p.push(...pt(ev));
    // Erasing redraws (to cut through what is there); writing adds a piece.
    if (current.e) {
      redraw();
      draw(canvas!.getContext("2d")!, current);
    } else draw(canvas!.getContext("2d")!, current, from);
  }
  function up() {
    if (!current) return;
    const s = current;
    current = null;
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
    if (!page.strokes.length && !page.text) return;
    redo = [];
    setPage({ strokes: [], text: "" });
  }
  function choose(t: Template) {
    template = t;
    redo = [];
    picking = false;
    persist();
  }
  function close() {
    padState.open = false;
  }
  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (picking || styleOpen) picking = styleOpen = false;
      else close();
      return;
    }
    if (template !== "type" && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
      e.preventDefault();
      if (e.shiftKey) again();
      else undo();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

{#snippet icon(t: Template)}
  {#if t === "draw"}
    <svg viewBox="0 0 48 32" aria-hidden="true"><path d="M3 24c4-10 9-12 9-6s-6 10-3 10 10-14 14-12-2 10 2 10 6-4 8-6" /><path d="M30 18l12-12 4 4-12 12-5 1z" /></svg>
  {:else if t === "type"}
    <svg viewBox="0 0 48 32" aria-hidden="true"><path d="M10 4h22M21 4v26M15 30h12" /><path d="M38 3v28M35 3h6M35 31h6" /></svg>
  {:else if t === "grid"}
    <svg viewBox="0 0 48 32" aria-hidden="true"><path d="M8 2v28M18 2v28M28 2v28M38 2v28M2 8h44M2 16h44M2 24h44" /></svg>
  {:else if t === "craft"}
    <svg viewBox="0 0 48 32" aria-hidden="true"><rect x="12" y="1" width="24" height="30" /><text x="16" y="7.5" font-size="6">C</text><text x="16" y="13.3" font-size="6">R</text><text x="16" y="19.1" font-size="6">A</text><text x="16" y="24.9" font-size="6">F</text><text x="16" y="30.5" font-size="6">T</text></svg>
  {:else if t === "takeoff"}
    <svg viewBox="0 0 48 32" aria-hidden="true"><path class="solid" d="M6 26l30-12c4-2 8-1 8 1s-4 3-8 4L14 26z M18 20l-6-8h4l10 5z" /></svg>
  {:else}
    <svg viewBox="0 0 48 32" aria-hidden="true"><path class="solid" d="M8 10l30 10c4 1 6 4 4 5s-5 0-9-1L10 18z M20 14l2-10h3l4 12z" /></svg>
  {/if}
{/snippet}

<div class="pad" class:full role="dialog" aria-label="Scratch pad">
  <div class="bar">
    <button class="tpl" onclick={() => ((picking = !picking), (styleOpen = false))} aria-expanded={picking} title="Choose a template">
      <span class="ti">{@render icon(template)}</span>
      {TEMPLATES.find((x) => x.id === template)?.label}
      <svg class="caret" viewBox="0 0 12 12" aria-hidden="true"><path d="M2 4l4 4 4-4" /></svg>
    </button>
    {#if template !== "type"}
      <button class="tool" class:on={!erasing} onclick={() => (erasing = false)} title="Pen" aria-pressed={!erasing}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20l2-6L16 4l4 4-10 10z" /><path d="M14 6l4 4" /></svg>
      </button>
      <div class="style">
        <button class="sw" onclick={() => ((styleOpen = !styleOpen), (picking = false))} aria-expanded={styleOpen} title="Colour and size">
          <i style:background={color}></i>{size} px
          <svg class="caret" viewBox="0 0 12 12" aria-hidden="true"><path d="M2 4l4 4 4-4" /></svg>
        </button>
        {#if styleOpen}
          <div class="pop">
            <div class="row">
              {#each COLORS as c (c)}
                <button class="dot" class:on={color === c} style:background={c} aria-label="Colour {c}" onclick={() => ((color = c), (erasing = false))}></button>
              {/each}
            </div>
            <div class="row">
              {#each SIZES as s (s)}
                <button class="size" class:on={size === s} onclick={() => (size = s)}><i style:width="{s + 2}px" style:height="{s + 2}px"></i>{s} px</button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      <button class="tool" class:on={erasing} onclick={() => (erasing = true)} title="Eraser" aria-pressed={erasing}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 20h12" /><path d="M4 15l9-9 6 6-8 8H8z" /><path d="M9 10l6 6" /></svg>
      </button>
    {/if}
    <span class="sp"></span>
    <button class="txt" onclick={clear}>Clear</button>
    {#if template !== "type"}
      <button class="tool" onclick={undo} disabled={!page.strokes.length} title="Undo" aria-label="Undo">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 7H5V3" /><path d="M5 7a8 8 0 1 1-1 7" /></svg>
      </button>
      <button class="tool" onclick={again} disabled={!redo.length} title="Redo" aria-label="Redo">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 7h4V3" /><path d="M19 7a8 8 0 1 0 1 7" /></svg>
      </button>
    {/if}
    <button class="tool" onclick={() => (full = !full)} title={full ? "Smaller" : "Full window"} aria-label={full ? "Smaller" : "Full window"}>
      {#if full}
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" /></svg>
      {:else}
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></svg>
      {/if}
    </button>
    <button class="tool" onclick={close} title="Close" aria-label="Close scratch pad">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </div>

  <div class="page" bind:this={box}>
    <div class="tmpl {template}" aria-hidden="true">
      {#if template === "craft"}
        {#each [["C", "Clearance limit"], ["R", "Route"], ["A", "Altitude"], ["F", "Frequency"], ["T", "Transponder"]] as [l, name] (l)}
          <div class="crow"><b>{l}</b><span>{name}</span></div>
        {/each}
      {:else if template === "takeoff" || template === "landing"}
        {#each template === "takeoff" ? ["ATIS", "Wind", "Visibility", "Sky", "Temp / Dew", "Altimeter", "Runway", "Taxi route", "Clearance", "Squawk", "Departure", "Notes"] : ["ATIS", "Wind", "Visibility", "Sky", "Temp / Dew", "Altimeter", "Approach", "Runway", "Minimums", "Missed approach", "Tower / Ground", "Taxi to"] as f (f)}
          <div class="cell"><span>{f}</span></div>
        {/each}
      {/if}
    </div>
    {#if template === "type"}
      <!-- svelte-ignore a11y_autofocus -->
      <textarea class="typed" placeholder="Type notes here…" value={page.text ?? ""} oninput={(e) => setPage({ ...page, text: (e.currentTarget as HTMLTextAreaElement).value })} autofocus></textarea>
    {/if}
    <canvas
      bind:this={canvas}
      class:off={template === "type"}
      class:erase={erasing}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
    ></canvas>
  </div>

  {#if picking}
    <div class="pick" role="dialog" aria-label="Choose a template">
      <div class="ph">Choose a template</div>
      <div class="grid">
        {#each TEMPLATES as t (t.id)}
          <button class:on={template === t.id} onclick={() => choose(t.id)}>
            <span class="big">{@render icon(t.id)}</span>
            {t.label}
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .pad {
    position: fixed;
    right: 16px;
    top: 64px;
    width: min(620px, calc(100vw - 32px));
    height: min(720px, calc(100vh - 88px));
    z-index: 150;
    display: flex;
    flex-direction: column;
    background: #141a24;
    border: 1px solid var(--rule);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 18px 40px rgba(0, 0, 0, 0.55);
    color: var(--ink);
  }
  .pad.full {
    inset: 0;
    width: auto;
    height: auto;
    border-radius: 0;
    z-index: 210;
  }
  @media (max-width: 640px) {
    .pad {
      inset: 0;
      width: auto;
      height: auto;
      border-radius: 0;
      z-index: 210;
    }
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 7px 8px;
    background: #1d2738;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    flex-wrap: wrap;
  }
  .bar button {
    height: 34px;
    border-radius: 7px;
    border: 0;
    background: #121925;
    color: var(--ink);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
  }
  .bar button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .bar svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .bar .caret {
    width: 11px;
    height: 11px;
  }
  .tpl {
    padding: 0 10px 0 6px;
  }
  .tpl .ti :global(svg) {
    width: 26px;
    height: 18px;
  }
  .tool {
    width: 38px;
  }
  .tool.on {
    background: #3a86e0;
  }
  .txt {
    padding: 0 12px;
  }
  .sp {
    flex: 1;
  }
  .style {
    position: relative;
  }
  .sw {
    padding: 0 9px;
  }
  .sw i {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 1px solid rgba(255, 255, 255, 0.35);
  }
  .pop {
    position: absolute;
    top: 40px;
    left: 0;
    z-index: 3;
    display: grid;
    gap: 8px;
    padding: 10px;
    background: #1d2738;
    border: 1px solid var(--rule);
    border-radius: 9px;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.5);
  }
  .pop .row {
    display: flex;
    gap: 6px;
  }
  .pop .dot {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid transparent;
  }
  .pop .dot.on {
    border-color: #fff;
    box-shadow: 0 0 0 2px #3a86e0;
  }
  .pop .size {
    padding: 0 9px;
    gap: 6px;
  }
  .pop .size.on {
    background: #3a86e0;
  }
  .pop .size i {
    border-radius: 50%;
    background: currentColor;
  }
  .page {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: #111722;
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
  .tmpl {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .tmpl.grid {
    background-image:
      linear-gradient(rgba(255, 255, 255, 0.08) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.08) 1px, transparent 1px);
    background-size: 28px 28px;
  }
  .tmpl.craft {
    display: grid;
    grid-template-rows: repeat(5, 1fr);
  }
  .crow {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 8px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }
  .crow b {
    font-size: 30px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.22);
    width: 26px;
  }
  .crow span,
  .cell span {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.35);
    padding-top: 4px;
  }
  .tmpl.takeoff,
  .tmpl.landing {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(4, 1fr);
  }
  .cell {
    padding: 6px 10px;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }
  .cell:nth-child(3n) {
    border-right: 0;
  }
  .typed {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    resize: none;
    border: 0;
    outline: none;
    padding: 16px 18px;
    background: transparent;
    color: var(--ink);
    font: 15px/1.55 "IBM Plex Mono", monospace;
  }
  .pick {
    position: absolute;
    left: 8px;
    top: 50px;
    z-index: 4;
    width: min(420px, calc(100% - 16px));
    background: #1d2738;
    border: 1px solid var(--rule);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.55);
  }
  .ph {
    padding: 11px;
    text-align: center;
    font-weight: 600;
    background: #26334a;
  }
  .pick .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }
  .pick .grid button {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 16px 6px 14px;
    border: 0;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .pick .grid button:hover,
  .pick .grid button.on {
    background: rgba(58, 134, 224, 0.22);
  }
  .big :global(svg) {
    width: 56px;
    height: 38px;
  }
  .pad :global(svg text) {
    fill: currentColor;
    stroke: none;
    font-weight: 700;
  }
  .pad :global(svg .solid) {
    fill: currentColor;
    stroke: none;
  }
  .ti :global(svg),
  .big :global(svg) {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
