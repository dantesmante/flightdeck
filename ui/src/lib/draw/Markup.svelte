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
  import { pen, markupState } from "./store.svelte";
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
  let redo = $state.raw<Mark[]>([]);
  let canvas = $state<HTMLCanvasElement>();
  let live: [number, number][] = [];
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
    return () => ro.disconnect();
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
  /** The eraser takes away whole marks it passes over. */
  function eraseAt(p: [number, number]) {
    const map = getMap();
    const hit = (m: Mark) =>
      m.p.some((ll) => {
        const q = map.project(ll);
        return Math.hypot(q.x - p[0], q.y - p[1]) < 12 + m.w;
      });
    const keep = marks.filter((m) => !hit(m));
    if (keep.length !== marks.length) {
      redo = [];
      onchange(keep);
    }
  }
  function down(e: PointerEvent) {
    if (e.button > 0) return;
    canvas!.setPointerCapture(e.pointerId);
    drawingNow = true;
    const p = xy(e);
    if (pen.erasing) return eraseAt(p);
    live = [p];
    paint();
  }
  function move(e: PointerEvent) {
    if (!drawingNow) return;
    const p = xy(e);
    if (pen.erasing) return eraseAt(p);
    for (const ev of e.getCoalescedEvents?.() ?? [e]) live.push(xy(ev));
    paint();
  }
  function up() {
    if (!drawingNow) return;
    drawingNow = false;
    if (pen.erasing || !live.length) return;
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
    redo = [];
    onchange([...marks, { c: pen.color, w: pen.size, p: ll }]);
  }
  function undo() {
    if (!marks.length) return;
    redo = [...redo, marks[marks.length - 1]];
    onchange(marks.slice(0, -1));
  }
  function again() {
    if (!redo.length) return;
    const m = redo[redo.length - 1];
    redo = redo.slice(0, -1);
    onchange([...marks, m]);
  }
</script>

<canvas bind:this={canvas} class="markup" class:moving class:erase={pen.erasing} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}></canvas>
<div class="mdock" style:left="calc({insetLeft}px + (100% - {insetLeft}px) / 2)" style:bottom="{16 + insetBottom}px">
  <DrawPalette canUndo={marks.length > 0} canRedo={redo.length > 0} onundo={undo} onredo={again} onclear={() => ((redo = []), onchange([]))}>
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
    cursor: cell;
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
