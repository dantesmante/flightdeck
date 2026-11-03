<script lang="ts" module>
  /** What part of the map is in view: the top-left corner in map units and
   * the zoom (1 = the whole world). Both pages share it, so the map stays
   * continuous over the spine. */
  export interface View {
    x: number;
    y: number;
    k: number;
  }
  export interface MapAirport {
    icao: string;
    lon: number;
    lat: number;
  }

</script>

<script lang="ts">
  // The passport's endpaper: a world map printed across both inside pages,
  // with every country the pilot has flown in shaded in passport ink. Each
  // page draws its own half (the map runs over the spine).
  interface Country {
    c: string;
    n: string;
    d: string;
    dot?: [number, number];
  }
  interface World {
    w: number;
    h: number;
    countries: Country[];
  }

  interface Props {
    world: World;
    visited: Set<string>;
    half: "left" | "right";
    width: number;
    view?: View;
    onview?: (v: View) => void;
    /** Airports you have flown from or to, shown as dots once zoomed in. */
    airports?: MapAirport[];
  }
  let { world, visited, half, width, view = { x: 0, y: 0, k: 1 }, onview, airports = [] }: Props = $props();

  const MAX_ZOOM = 16;
  // The visible part of the whole map, and this page's half of it.
  const vw = $derived(world.w / view.k);
  const vh = $derived(world.h / view.k);
  const vx = $derived(half === "left" ? view.x : view.x + vw / 2);
  const viewBox = $derived(`${vx} ${view.y} ${vw / 2} ${vh}`);
  const height = $derived((width * world.h) / (world.w / 2));
  /** Screen pixels per map unit, to keep dots and text the same size at any zoom. */
  const ppu = $derived(width / (vw / 2));

  function clamp(v: View): View {
    const k = Math.min(MAX_ZOOM, Math.max(1, v.k));
    const w = world.w / k;
    const h = world.h / k;
    return { k, x: Math.min(world.w - w, Math.max(0, v.x)), y: Math.min(world.h - h, Math.max(0, v.y)) };
  }
  /** The map point under a pointer event. */
  function at(e: { clientX: number; clientY: number }, el: Element): [number, number] {
    const r = el.getBoundingClientRect();
    return [vx + ((e.clientX - r.left) / r.width) * (vw / 2), view.y + ((e.clientY - r.top) / r.height) * vh];
  }
  function wheel(e: WheelEvent) {
    if (!onview) return;
    e.preventDefault();
    const [mx, my] = at(e, e.currentTarget as Element);
    const k = Math.min(MAX_ZOOM, Math.max(1, view.k * Math.exp(-e.deltaY * 0.0015)));
    const f = view.k / k;
    // Keep the point under the pointer where it is.
    onview(clamp({ k, x: mx - (mx - view.x) * f, y: my - (my - view.y) * f }));
  }
  let drag = $state<{ x: number; y: number; vx: number; vy: number } | null>(null);
  function down(e: PointerEvent) {
    if (!onview || view.k <= 1) return;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, y: e.clientY, vx: view.x, vy: view.y };
  }
  function move(e: PointerEvent) {
    if (!drag || !onview) return;
    const r = (e.currentTarget as Element).getBoundingClientRect();
    const s = vw / 2 / r.width;
    onview(clamp({ k: view.k, x: drag.vx - (e.clientX - drag.x) * s, y: drag.vy - (e.clientY - drag.y) * s }));
  }
  function up() {
    drag = null;
  }
  function dblclick(e: MouseEvent) {
    if (!onview) return;
    const [mx, my] = at(e, e.currentTarget as Element);
    const k = Math.min(MAX_ZOOM, view.k * 2);
    const f = view.k / k;
    onview(clamp({ k, x: mx - (mx - view.x) * f, y: my - (my - view.y) * f }));
  }

  // Meridians and parallels every 30 degrees, in the same Equal Earth frame.
  const A1 = 1.340264, A2 = -0.081106, A3 = 0.000893, A4 = 0.003796, M = Math.sqrt(3) / 2;
  const XM = 2.7064737, YM = 1.3173627;
  function project(lon: number, lat: number): [number, number] {
    const s = world.w / (2 * XM);
    const l = (lon * Math.PI) / 180;
    const t = Math.asin(M * Math.sin((lat * Math.PI) / 180));
    const t2 = t * t, t6 = t2 * t2 * t2;
    const x = (2 * Math.sqrt(3) * l * Math.cos(t)) / (3 * (9 * A4 * t6 * t2 + 7 * A3 * t6 + 3 * A2 * t2 + A1));
    const y = t * (A4 * t6 * t2 + A3 * t6 + A2 * t2 + A1);
    return [(x + XM) * s, (YM - y) * s];
  }
  const line = (pts: [number, number][]) => "M" + pts.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join("L");
  const graticule = $derived.by(() => {
    const out: string[] = [];
    for (let lon = -180; lon <= 180; lon += 30) {
      out.push(line(Array.from({ length: 37 }, (_, i) => project(lon, -90 + i * 5))));
    }
    for (let lat = -60; lat <= 60; lat += 30) {
      out.push(line(Array.from({ length: 73 }, (_, i) => project(-180 + i * 5, lat))));
    }
    return out;
  });
  const outline = $derived(
    line([
      ...Array.from({ length: 37 }, (_, i) => project(-180, -90 + i * 5)),
      ...Array.from({ length: 37 }, (_, i) => project(180, 90 - i * 5)),
    ]) + "Z",
  );
</script>

<svg
  {viewBox}
  {width}
  {height}
  class="world"
  class:zoomable={!!onview}
  class:zoomed={view.k > 1}
  class:dragging={!!drag}
  role="img"
  aria-label="World map{half === 'left' ? '' : ', continued'}"
  onwheel={wheel}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  ondblclick={dblclick}
>
  <defs>
    <pattern id="ink-hatch-{half}" width="3" height="3" patternUnits="userSpaceOnUse" patternTransform="rotate(45) scale({1 / Math.sqrt(view.k)})">
      <rect width="3" height="3" fill="#5f8f88" />
      <line x1="0" y1="0" x2="0" y2="3" stroke="#3f6f69" stroke-width="1.1" />
    </pattern>
  </defs>
  <path d={outline} class="sea" />
  {#each graticule as g, i (i)}<path d={g} class="grid" />{/each}
  {#each world.countries as c (c.c)}
    <path d={c.d} class:visited={visited.has(c.c)} style:fill={visited.has(c.c) ? `url(#ink-hatch-${half})` : null}>
      <title>{c.n}{visited.has(c.c) ? ", visited" : ""}</title>
    </path>
  {/each}
  {#if view.k < 2.5}
    {#each world.countries.filter((c) => c.dot && visited.has(c.c)) as c (c.c)}
      <circle cx={c.dot![0]} cy={c.dot![1]} r={3.2 / ppu * (width / (world.w / 2))} class="dot"><title>{c.n}, visited</title></circle>
    {/each}
  {:else}
    <!-- Zoomed in: a dot at every airport you have flown from or to. -->
    {#each airports as a (a.icao)}
      {@const [x, y] = project(a.lon, a.lat)}
      <circle cx={x} cy={y} r={3 / ppu} class="apt" style:stroke-width={1 / ppu}><title>{a.icao}</title></circle>
      {#if view.k >= 5}
        <text x={x + 5 / ppu} y={y + 3.2 / ppu} class="apt-code" style:font-size="{9 / ppu}px">{a.icao}</text>
      {/if}
    {/each}
  {/if}
</svg>

<style>
  .world {
    display: block;
    touch-action: none;
  }
  .world.zoomable {
    cursor: zoom-in;
  }
  .world.zoomed {
    cursor: grab;
  }
  .world.dragging {
    cursor: grabbing;
  }
  .world :global(path) {
    vector-effect: non-scaling-stroke;
  }
  .apt {
    fill: #7a2e3a;
    stroke: #f1e4c9;
  }
  .apt-code {
    fill: #4a2630;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-weight: 600;
    paint-order: stroke;
    stroke: rgba(233, 220, 196, 0.85);
    stroke-width: 0.6px;
  }
  .sea {
    fill: rgba(159, 193, 189, 0.16);
    stroke: #9fb7b3;
    stroke-width: 0.6;
  }
  .grid {
    fill: none;
    stroke: #9fb7b3;
    stroke-width: 0.3;
    opacity: 0.7;
  }
  path:not(.sea):not(.grid) {
    fill: #d8c7aa;
    stroke: #a99d81;
    stroke-width: 0.35;
    stroke-linejoin: round;
  }
  path.visited {
    stroke: #2f5a55;
    stroke-width: 0.45;
  }
  .dot {
    fill: #3f6f69;
    stroke: #e2d3b8;
    stroke-width: 0.8;
  }
</style>
