<script lang="ts">
  // Altitude and ground speed over the flight. Altitude is a filled profile
  // shaded with the same red-to-green scale as the map line; ground speed is
  // a single yellow line on its own scale at the right.
  import { ALT_STOPS, SPEED_COLOR, altRange, type Pt } from "./track";

  interface Props {
    pts: Pt[];
    height?: number;
    title?: boolean;
  }
  let { pts, height = 190, title = true }: Props = $props();

  let width = $state(0);
  let hover = $state<number | null>(null);
  const uid = `g${Math.random().toString(36).slice(2, 8)}`;

  const PAD = { l: 48, r: 44, t: 10, b: 24 };

  /** At most ~one point per pixel column. */
  const data = $derived.by(() => {
    const n = pts.length;
    const max = Math.max(60, Math.round(width || 600));
    if (n <= max) return pts;
    const out: Pt[] = [];
    const step = n / max;
    for (let i = 0; i < max; i++) out.push(pts[Math.floor(i * step)]);
    out.push(pts[n - 1]);
    return out;
  });

  function nice(max: number, ticks = 4): { top: number; step: number } {
    const raw = max / ticks;
    const mag = 10 ** Math.floor(Math.log10(raw || 1));
    const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw;
    return { top: Math.ceil(max / step) * step || step, step };
  }

  const g = $derived.by(() => {
    const w = Math.max(10, width - PAD.l - PAD.r);
    const h = Math.max(10, height - PAD.t - PAD.b);
    const t0 = data[0]?.[4] ?? 0;
    const t1 = data[data.length - 1]?.[4] ?? 1;
    const span = Math.max(1, t1 - t0);
    const { lo, hi } = altRange(data);
    const base = Math.max(0, Math.floor(lo / 1000) * 1000);
    const a = nice(hi - base);
    const altTop = base + a.top;
    const maxGs = Math.max(60, ...data.map((p) => p[3]));
    const s = nice(maxGs);
    const x = (t: number) => PAD.l + ((t - t0) / span) * w;
    const ya = (ft: number) => PAD.t + h - ((ft - base) / (altTop - base)) * h;
    const ys = (kt: number) => PAD.t + h - (kt / s.top) * h;
    let alt = "";
    let spd = "";
    data.forEach((p, i) => {
      alt += `${i ? "L" : "M"}${x(p[4]).toFixed(1)} ${ya(p[2]).toFixed(1)}`;
      spd += `${i ? "L" : "M"}${x(p[4]).toFixed(1)} ${ys(p[3]).toFixed(1)}`;
    });
    const floor = PAD.t + h;
    const area = data.length ? `${alt}L${x(t1).toFixed(1)} ${floor}L${x(t0).toFixed(1)} ${floor}Z` : "";
    const altTicks: number[] = [];
    for (let v = base; v <= altTop + 1; v += a.step) altTicks.push(v);
    const spdTicks: number[] = [];
    for (let v = 0; v <= s.top + 0.1; v += s.step) spdTicks.push(v);
    // Time ticks on round minutes.
    const mins = span / 60000;
    const room = Math.max(2, Math.floor(w / 72));
    const tStep = [5, 10, 15, 30, 60, 120, 180, 240].find((m) => mins / m <= room) ?? 360;
    const timeTicks: number[] = [];
    const first = Math.ceil(t0 / (tStep * 60000)) * tStep * 60000;
    for (let t = first; t <= t1; t += tStep * 60000) timeTicks.push(t);
    return {
      w, h, x, ya, ys, alt, spd, area, floor, altTicks, spdTicks, timeTicks, t0, span,
      // The fill's colour scale runs over the flight's own range, like the line.
      gy0: ya(lo), gy1: ya(hi),
    };
  });

  const zulu = (t: number) => {
    const d = new Date(t);
    return `${String(d.getUTCHours()).padStart(2, "0")}:${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
  const kfmt = (v: number) => (v >= 1000 ? `${(v / 1000).toLocaleString("en-US", { maximumFractionDigits: 1 })}k` : `${v}`);

  function move(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    const px = e.clientX - r.left;
    if (px < PAD.l || px > PAD.l + g.w || !data.length) return (hover = null);
    const t = g.t0 + ((px - PAD.l) / g.w) * g.span;
    let lo = 0;
    let hi = data.length - 1;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (data[mid][4] < t) lo = mid;
      else hi = mid;
    }
    hover = Math.abs(data[lo][4] - t) < Math.abs(data[hi][4] - t) ? lo : hi;
  }

  const hp = $derived(hover != null ? data[hover] : null);
  const elapsed = (t: number) => {
    const m = Math.round((t - g.t0) / 60000);
    return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;
  };
</script>

<figure class="graph">
  {#if title}
    <figcaption>
      <span class="name">Altitude &amp; speed</span>
      <span class="key"><i class="sw alt"></i>Altitude, ft</span>
      <span class="key"><i class="sw spd" style:background={SPEED_COLOR}></i>Ground speed, kt</span>
    </figcaption>
  {/if}
  <div class="plot" bind:clientWidth={width} style:height="{height}px">
    {#if pts.length < 2}
      <p class="empty">No altitude or speed was recorded for this flight.</p>
    {:else if width > 0}
      <svg
        {width}
        {height}
        role="img"
        aria-label="Altitude and ground speed over time"
        onpointermove={move}
        onpointerleave={() => (hover = null)}
      >
        <defs>
          <linearGradient id="{uid}-a" gradientUnits="userSpaceOnUse" x1="0" x2="0" y1={g.gy0} y2={g.gy1}>
            {#each ALT_STOPS as [f, c] (f)}<stop offset={f} stop-color={c} />{/each}
          </linearGradient>
          <linearGradient id="{uid}-f" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0" stop-color="#fff" stop-opacity="0.5" />
            <stop offset="1" stop-color="#fff" stop-opacity="0.06" />
          </linearGradient>
          <mask id="{uid}-m"><rect x="0" y="0" {width} {height} fill="url(#{uid}-f)" /></mask>
        </defs>

        {#each g.altTicks as v (v)}
          <line class="grid" x1={PAD.l} x2={PAD.l + g.w} y1={g.ya(v)} y2={g.ya(v)} />
          <text class="tick" x={PAD.l - 7} y={g.ya(v) + 4} text-anchor="end">{kfmt(v)}</text>
        {/each}
        {#each g.spdTicks as v (v)}
          <text class="tick" x={PAD.l + g.w + 7} y={g.ys(v) + 4}>{v}</text>
        {/each}
        {#each g.timeTicks as t (t)}
          <text class="tick" x={g.x(t)} y={height - 6} text-anchor="middle">{zulu(t)}</text>
        {/each}

        <path d={g.area} fill="url(#{uid}-a)" mask="url(#{uid}-m)" />
        <path d={g.alt} fill="none" stroke="url(#{uid}-a)" stroke-width="2" stroke-linejoin="round" />
        <path d={g.spd} fill="none" stroke={SPEED_COLOR} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
        <line class="axis" x1={PAD.l} x2={PAD.l + g.w} y1={g.floor} y2={g.floor} />

        {#if hp}
          {@const hx = g.x(hp[4])}
          <line class="cross" x1={hx} x2={hx} y1={PAD.t} y2={g.floor} />
          <circle cx={hx} cy={g.ya(hp[2])} r="4.5" class="dot" fill="url(#{uid}-a)" />
          <circle cx={hx} cy={g.ys(hp[3])} r="4.5" class="dot" fill={SPEED_COLOR} />
        {/if}
      </svg>
      {#if hp}
        {@const hx = g.x(hp[4])}
        <div class="tip" style:left="{Math.min(Math.max(hx, 70), width - 70)}px">
          <span class="when">{zulu(hp[4])} <em>+{elapsed(hp[4])}</em></span>
          <span><i class="sw alt"></i><b>{Math.round(hp[2]).toLocaleString("en-US")}</b> ft</span>
          <span><i class="sw spd" style:background={SPEED_COLOR}></i><b>{Math.round(hp[3])}</b> kt</span>
        </div>
      {/if}
    {/if}
  </div>
</figure>

<style>
  .graph {
    margin: 0;
    padding: 12px 12px 6px;
    border-radius: 10px;
    background: linear-gradient(180deg, #1a2a37, #15222d);
    border: 1px solid var(--rule);
  }
  figcaption {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin: 0 0 6px 4px;
    font-size: 12px;
    color: var(--ink-dim);
  }
  .name {
    font-family: var(--cond);
    font-weight: 600;
    font-size: 15px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
    flex-basis: 100%;
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .sw {
    display: inline-block;
    width: 14px;
    height: 4px;
    border-radius: 2px;
  }
  .sw.alt {
    background: linear-gradient(90deg, #ef4b4b, #f6b73c, #34c98b);
  }
  .plot {
    position: relative;
    width: 100%;
  }
  svg {
    display: block;
    touch-action: none;
  }
  .grid {
    stroke: rgba(201, 214, 224, 0.08);
    stroke-width: 1;
  }
  .axis {
    stroke: rgba(201, 214, 224, 0.22);
  }
  .tick {
    fill: var(--ink-dim);
    font-family: "IBM Plex Mono", monospace;
    font-size: 10.5px;
  }
  .cross {
    stroke: rgba(232, 238, 242, 0.45);
    stroke-dasharray: 3 3;
  }
  .dot {
    stroke: #15222d;
    stroke-width: 2;
  }
  .tip {
    position: absolute;
    top: -30px;
    transform: translateX(-50%);
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 5px 9px;
    border-radius: 6px;
    background: rgba(13, 24, 32, 0.92);
    border: 1px solid var(--rule);
    font-size: 12px;
    color: var(--ink-dim);
    white-space: nowrap;
    pointer-events: none;
  }
  .tip span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .tip b {
    color: var(--ink);
    font-family: "IBM Plex Mono", monospace;
    font-weight: 400;
  }
  .tip .when {
    color: var(--ink);
    font-family: "IBM Plex Mono", monospace;
  }
  .tip em {
    font-style: normal;
    color: var(--ink-dim);
  }
  .empty {
    margin: 0;
    padding: 40px 0;
    text-align: center;
    color: var(--ink-dim);
    font-size: 13px;
  }
</style>
