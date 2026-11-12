<script lang="ts" module>
  import type maplibregl from "maplibre-gl";
  import type { GeoJSONSource } from "maplibre-gl";

  export type Mark = { c: string; w: number; p: [number, number][] };
  const KEY = "fd.markup";

  /** The marks drawn on the map, kept in this browser (each device its own). */
  export function loadMarks(): Mark[] {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? "[]");
      return Array.isArray(v) ? v : [];
    } catch {
      return [];
    }
  }
  export function saveMarks(m: Mark[]) {
    try {
      localStorage.setItem(KEY, JSON.stringify(m));
    } catch {
      /* storage full or blocked: the marks stay until the page closes */
    }
  }

  /** The marks as a map layer (shown whether the tool is open or not). */
  export function showMarks(map: maplibregl.Map, marks: Mark[]) {
    const data: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: marks.map((m) => ({ type: "Feature", properties: { c: m.c, w: m.w }, geometry: m.p.length > 1 ? { type: "LineString", coordinates: m.p } : { type: "Point", coordinates: m.p[0] } })),
    };
    const src = map.getSource("fd-markup") as GeoJSONSource | undefined;
    if (src) {
      src.setData(data);
      return;
    }
    map.addSource("fd-markup", { type: "geojson", data });
    map.addLayer({
      id: "fd-markup",
      type: "line",
      source: "fd-markup",
      filter: ["==", ["geometry-type"], "LineString"],
      layout: { "line-cap": "round", "line-join": "round" },
      paint: { "line-color": ["get", "c"], "line-width": ["get", "w"] },
    });
    map.addLayer({
      id: "fd-markup-dot",
      type: "circle",
      source: "fd-markup",
      filter: ["==", ["geometry-type"], "Point"],
      paint: { "circle-color": ["get", "c"], "circle-radius": ["/", ["get", "w"], 2] },
    });
  }
</script>

<script lang="ts">
  // Markup (Tools → Markup): draw on the map with the pen. The marks stick
  // to the map (they move and zoom with it) and stay after the tool is
  // closed, until cleared. "Move map" lets the map be panned meanwhile.
  import { onDestroy, onMount } from "svelte";
  import { eraserR, pen, markupState } from "./store.svelte";
  import DrawPalette from "./DrawPalette.svelte";

  interface Props {
    getMap: () => maplibregl.Map;
    marks: Mark[];
    onchange: (m: Mark[]) => void;
    insetLeft?: number;
    insetBottom?: number;
  }
  let { getMap, marks, onchange, insetLeft = 0, insetBottom = 0 }: Props = $props();

  let moving = $state(false);
  /** Undo and redo: the marks before and after each change. */
  let past = $state.raw<Mark[][]>([]);
  let future = $state.raw<Mark[][]>([]);
  /** The marks when the current stroke or rub began. */
  let before: Mark[] | null = null;
  /** The eraser's ring, where the pointer is. */
  let ring = $state<[number, number] | null>(null);
  let canvas = $state<HTMLCanvasElement>();
  let live: [number, number][] = [];
  let dockEl = $state<HTMLDivElement>();
  let drawingNow = false;

  function fit() {
    const c = canvas;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(c.clientWidth * dpr);
    c.height = Math.round(c.clientHeight * dpr);
  }
  onMount(() => {
    fit();
    const ro = new ResizeObserver(fit);
    if (canvas) ro.observe(canvas);
    // Not passive: the page itself must not scroll.
    canvas?.addEventListener("wheel", wheel, { passive: false });
    return () => {
      ro.disconnect();
      canvas?.removeEventListener("wheel", wheel);
    };
  });
  onDestroy(() => {
    pen.erasing = false;
  });

  function xy(e: PointerEvent): [number, number] {
    const r = canvas!.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }
  function paint() {
    const c = canvas!;
    const g = c.getContext("2d")!;
    const dpr = window.devicePixelRatio || 1;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, c.width, c.height);
    if (!live.length) return;
    g.strokeStyle = pen.color;
    g.fillStyle = pen.color;
    g.lineWidth = pen.size;
    g.lineCap = "round";
    g.lineJoin = "round";
    g.beginPath();
    g.moveTo(live[0][0], live[0][1]);
    for (const p of live.slice(1)) g.lineTo(p[0], p[1]);
    if (live.length === 1) g.arc(live[0][0], live[0][1], pen.size / 2, 0, Math.PI * 2), g.fill();
    else g.stroke();
  }
  /** The eraser: whole marks it touches ("line"), or only the parts under it ("area"). */
  function eraseAt(p: [number, number]) {
    const map = getMap();
    const r = eraserR();
    const near = (x: number, y: number, w: number) => Math.hypot(x - p[0], y - p[1]) < r + w / 2;
    if (pen.eraseMode === "line") {
      const keep = marks.filter(
        (m) =>
          !m.p.some((ll, i) => {
            const q = map.project(ll);
            if (near(q.x, q.y, m.w)) return true;
            // Between two points too (a long straight piece).
            if (i === 0) return false;
            const a = map.project(m.p[i - 1]);
            const dx = q.x - a.x;
            const dy = q.y - a.y;
            const t = Math.max(0, Math.min(1, ((p[0] - a.x) * dx + (p[1] - a.y) * dy) / (dx * dx + dy * dy || 1)));
            return near(a.x + t * dx, a.y + t * dy, m.w);
          }),
      );
      if (keep.length !== marks.length) onchange(keep);
      return;
    }
    let changed = false;
    const out: Mark[] = [];
    for (const m of marks) {
      // Points close enough together to cut cleanly (a few px apart).
      const pts: { x: number; y: number; ll: [number, number] }[] = [];
      for (let i = 0; i < m.p.length; i++) {
        const q = map.project(m.p[i]);
        if (i > 0) {
          const a = pts[pts.length - 1];
          const n = Math.floor(Math.hypot(q.x - a.x, q.y - a.y) / Math.max(2, r / 2));
          for (let k = 1; k < n; k++) {
            const t = k / n;
            const ll: [number, number] = [a.ll[0] + (m.p[i][0] - a.ll[0]) * t, a.ll[1] + (m.p[i][1] - a.ll[1]) * t];
            pts.push({ x: a.x + (q.x - a.x) * t, y: a.y + (q.y - a.y) * t, ll });
          }
        }
        pts.push({ x: q.x, y: q.y, ll: m.p[i] });
      }
      if (!pts.some((q) => near(q.x, q.y, m.w))) {
        out.push(m);
        continue;
      }
      changed = true;
      // What is left: the runs of points outside the eraser.
      let run: [number, number][] = [];
      for (const q of pts) {
        if (near(q.x, q.y, m.w)) {
          if (run.length > 1) out.push({ ...m, p: run });
          run = [];
        } else run.push(q.ll);
      }
      if (run.length > 1) out.push({ ...m, p: run });
    }
    if (changed) onchange(out);
  }
  function down(e: PointerEvent) {
    if (e.button > 0) return;
    canvas!.setPointerCapture(e.pointerId);
    drawingNow = true;
    before = marks;
    const p = xy(e);
    if (pen.erasing) return eraseAt(p);
    live = [p];
    paint();
  }
  function move(e: PointerEvent) {
    ring = pen.erasing ? xy(e) : null;
    if (!drawingNow) return;
    const p = xy(e);
    if (pen.erasing) return eraseAt(p);
    for (const ev of e.getCoalescedEvents?.() ?? [e]) live.push(xy(ev));
    paint();
  }
  /** One stroke or rub is one step to undo. */
  function remember() {
    if (before && before !== marks) {
      past = [...past.slice(-49), before];
      future = [];
    }
    before = null;
  }
  function up() {
    if (!drawingNow) return;
    drawingNow = false;
    if (pen.erasing || !live.length) {
      queueMicrotask(remember);
      return;
    }
    const map = getMap();
    // Thin the points a little (every 2 px or more), then onto the map.
    const pts: [number, number][] = [];
    for (const p of live) if (!pts.length || Math.hypot(p[0] - pts[pts.length - 1][0], p[1] - pts[pts.length - 1][1]) >= 2) pts.push(p);
    const ll = pts.map((p) => {
      const l = map.unproject(p);
      return [Math.round(l.lng * 1e6) / 1e6, Math.round(l.lat * 1e6) / 1e6] as [number, number];
    });
    live = [];
    paint();
    onchange([...marks, { c: pen.color, w: pen.size, p: ll }]);
    queueMicrotask(remember);
  }
  /** The mouse wheel zooms the map (and so moves around it) whenever the
   * pen is not pressed on the map; while drawing, the map stays put. */
  function wheel(e: WheelEvent) {
    e.preventDefault();
    if (drawingNow) return;
    const target = getMap().getCanvas();
    target.dispatchEvent(
      new WheelEvent("wheel", {
        bubbles: true,
        cancelable: true,
        deltaX: e.deltaX,
        deltaY: e.deltaY,
        deltaZ: e.deltaZ,
        deltaMode: e.deltaMode,
        clientX: e.clientX,
        clientY: e.clientY,
        screenX: e.screenX,
        screenY: e.screenY,
        ctrlKey: e.ctrlKey,
        shiftKey: e.shiftKey,
        altKey: e.altKey,
        metaKey: e.metaKey,
      }),
    );
  }
  function undo() {
    if (!past.length) return;
    future = [...future, marks];
    const prev = past[past.length - 1];
    past = past.slice(0, -1);
    onchange(prev);
  }
  function again() {
    if (!future.length) return;
    past = [...past, marks];
    const next = future[future.length - 1];
    future = future.slice(0, -1);
    onchange(next);
  }
  function clearAll() {
    if (!marks.length) return;
    past = [...past.slice(-49), marks];
    future = [];
    onchange([]);
  }
</script>

<canvas bind:this={canvas} class="markup" class:moving class:erase={pen.erasing} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onpointerleave={() => (ring = null)}></canvas>
{#if ring && pen.erasing && !moving}
  <!-- The eraser: an open circle its own size. -->
  <div class="ring" style:left="{ring[0]}px" style:top="{ring[1]}px" style:width="{eraserR() * 2}px" style:height="{eraserR() * 2}px"></div>
{/if}
<div class="mdock" bind:this={dockEl} style:left="calc({insetLeft}px + (100% - {insetLeft}px) / 2)" style:bottom="{16 + insetBottom}px">
  <DrawPalette compact={(canvas?.clientWidth ?? 1200) - insetLeft < 860} canUndo={past.length > 0} canRedo={future.length > 0} onundo={undo} onredo={again} onclear={clearAll}>
    {#snippet lead()}
      <button class:on={moving} aria-pressed={moving} title={moving ? "Back to drawing" : "Move the map"} onclick={() => (moving = !moving)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v18M3 12h18" /><path d="M9 6l3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3" /></svg>
      </button>
    {/snippet}
    {#snippet tail()}
      <button class="done" onclick={() => (markupState.open = false)}>Done</button>
    {/snippet}
  </DrawPalette>
</div>

<style>
  .markup {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 2;
    touch-action: none;
    cursor: crosshair;
  }
  .markup.erase {
    cursor: none;
  }
  .ring {
    position: absolute;
    z-index: 3;
    transform: translate(-50%, -50%);
    border: 1.5px solid #fff;
    border-radius: 50%;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.6), inset 0 0 0 1px rgba(0, 0, 0, 0.35);
    pointer-events: none;
  }
  /* Moving the map: the pen steps aside. */
  .markup.moving {
    pointer-events: none;
  }
  .mdock {
    position: absolute;
    transform: translateX(-50%);
    z-index: 5;
    max-width: calc(100% - 24px);
  }
  .done {
    padding: 0 14px !important;
    font-weight: 600 !important;
    color: #1b1406 !important;
    background: var(--amber) !important;
  }
</style>
