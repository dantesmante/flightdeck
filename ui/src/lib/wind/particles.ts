// Wind particles drawn on a plain 2D canvas over the map, on the processor
// rather than the graphics card, so the simulator keeps the GPU:
//
// - the canvas asks for a software context ("willReadFrequently" makes
//   Chromium and WebView2 keep it off the GPU);
// - the wind is turned into screen motion once per view, on a coarse grid,
//   so each frame is only a lookup and a short line per particle;
// - at most 15 frames a second, nothing while the map moves (the canvas is
//   cleared and redrawn when it stops), nothing while the window is hidden.

import type { Map as MlMap } from "maplibre-gl";
import { WIND_STOPS, windBucket, type Field, sample } from "./wind";

const FPS = 15;
const CELL = 12; // px between the points where the wind is worked out
const TRAIL = 7; // positions kept per particle
const PX_PER_KT = 0.09; // movement per frame
const MAX_STEP = 8;

export class WindParticles {
  private map: MlMap;
  private field: Field | null = null;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private ratio = 1;
  private w = 0;
  private h = 0;
  // Screen motion on the coarse grid: px per frame, and speed in knots.
  private cols = 0;
  private rows = 0;
  private vx = new Float32Array(0);
  private vy = new Float32Array(0);
  private kt = new Float32Array(0);
  // Particles: trail positions (ring), age, life.
  private n = 0;
  private trail = new Float32Array(0);
  private head = new Uint8Array(0);
  private filled = new Uint8Array(0);
  private age = new Uint16Array(0);
  private life = new Uint16Array(0);
  private raf = 0;
  private last = 0;
  private moving = false;
  private running = false;

  constructor(map: MlMap) {
    this.map = map;
    this.canvas = document.createElement("canvas");
    this.canvas.className = "wind-particles";
    Object.assign(this.canvas.style, { position: "absolute", left: "0", top: "0", pointerEvents: "none" });
    const gl = map.getCanvas();
    gl.parentElement?.insertBefore(this.canvas, gl.nextSibling);
    this.ctx = this.canvas.getContext("2d", { willReadFrequently: true })!;
    map.on("movestart", this.onMoveStart);
    map.on("moveend", this.onMoveEnd);
    map.on("resize", this.onMoveEnd);
  }

  setField(f: Field | null) {
    this.field = f;
    this.rebuild();
  }

  start() {
    if (this.running) return;
    this.running = true;
    this.canvas.style.display = "";
    this.rebuild();
    this.loop(performance.now());
  }

  stop() {
    this.running = false;
    cancelAnimationFrame(this.raf);
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    this.canvas.style.display = "none";
  }

  destroy() {
    this.stop();
    this.map.off("movestart", this.onMoveStart);
    this.map.off("moveend", this.onMoveEnd);
    this.map.off("resize", this.onMoveEnd);
    this.canvas.remove();
  }

  private onMoveStart = () => {
    this.moving = true;
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  };

  private onMoveEnd = () => {
    this.moving = false;
    if (this.running) this.rebuild();
  };

  /** Work out the screen motion for the current view and scatter particles. */
  private rebuild() {
    if (!this.running) return;
    const el = this.map.getCanvas();
    this.w = el.clientWidth;
    this.h = el.clientHeight;
    this.ratio = Math.min(2, window.devicePixelRatio || 1);
    this.canvas.width = Math.round(this.w * this.ratio);
    this.canvas.height = Math.round(this.h * this.ratio);
    this.canvas.style.width = `${this.w}px`;
    this.canvas.style.height = `${this.h}px`;
    this.ctx.setTransform(this.ratio, 0, 0, this.ratio, 0, 0);
    this.ctx.lineCap = "round";

    const f = this.field;
    this.cols = Math.ceil(this.w / CELL) + 1;
    this.rows = Math.ceil(this.h / CELL) + 1;
    const size = this.cols * this.rows;
    this.vx = new Float32Array(size);
    this.vy = new Float32Array(size);
    this.kt = new Float32Array(size).fill(NaN);
    if (f) {
      const d = 0.05;
      for (let r = 0; r < this.rows; r++) {
        for (let c = 0; c < this.cols; c++) {
          const x = c * CELL;
          const y = r * CELL;
          const ll = this.map.unproject([x, y]);
          if (!Number.isFinite(ll.lat) || Math.abs(ll.lat) > 85) continue;
          const [u, v] = sample(f, ll.lng, ll.lat);
          // Which way east and north point on screen here (handles rotation and tilt).
          const pe = this.map.project([ll.lng + d, ll.lat]);
          const pn = this.map.project([ll.lng, ll.lat + d]);
          const ex = pe.x - x, ey = pe.y - y;
          const nx = pn.x - x, ny = pn.y - y;
          const el2 = Math.hypot(ex, ey) || 1;
          const nl2 = Math.hypot(nx, ny) || 1;
          const sx = (u * ex) / el2 + (v * nx) / nl2;
          const sy = (u * ey) / el2 + (v * ny) / nl2;
          const k = PX_PER_KT;
          let mx = sx * k;
          let my = sy * k;
          const m = Math.hypot(mx, my);
          if (m > MAX_STEP) {
            mx *= MAX_STEP / m;
            my *= MAX_STEP / m;
          }
          const i = r * this.cols + c;
          this.vx[i] = mx;
          this.vy[i] = my;
          this.kt[i] = Math.hypot(u, v);
        }
      }
    }

    this.n = Math.min(1800, Math.round((this.w * this.h) / 1100));
    this.trail = new Float32Array(this.n * TRAIL * 2);
    this.head = new Uint8Array(this.n);
    this.filled = new Uint8Array(this.n);
    this.age = new Uint16Array(this.n);
    this.life = new Uint16Array(this.n);
    for (let p = 0; p < this.n; p++) this.spawn(p, true);
  }

  private spawn(p: number, anyAge = false) {
    const x = Math.random() * this.w;
    const y = Math.random() * this.h;
    const o = p * TRAIL * 2;
    this.trail[o] = x;
    this.trail[o + 1] = y;
    this.head[p] = 0;
    this.filled[p] = 1;
    this.life[p] = 30 + Math.floor(Math.random() * 60);
    this.age[p] = anyAge ? Math.floor(Math.random() * this.life[p]) : 0;
  }

  private cell(x: number, y: number): number {
    const c = Math.round(x / CELL);
    const r = Math.round(y / CELL);
    if (c < 0 || r < 0 || c >= this.cols || r >= this.rows) return -1;
    return r * this.cols + c;
  }

  private loop = (t: number) => {
    if (!this.running) return;
    this.raf = requestAnimationFrame(this.loop);
    if (this.moving || !this.field || t - this.last < 1000 / FPS - 2) return;
    // Nothing to do while the map is out of sight (another page is open).
    if (this.canvas.offsetParent === null) return;
    this.last = t;
    this.step();
    this.draw();
  };

  private step() {
    for (let p = 0; p < this.n; p++) {
      const o = p * TRAIL * 2;
      const h = this.head[p];
      const x = this.trail[o + h * 2];
      const y = this.trail[o + h * 2 + 1];
      const i = this.cell(x, y);
      if (++this.age[p] > this.life[p] || i < 0 || Number.isNaN(this.kt[i])) {
        this.spawn(p);
        continue;
      }
      const nh = (h + 1) % TRAIL;
      this.trail[o + nh * 2] = x + this.vx[i];
      this.trail[o + nh * 2 + 1] = y + this.vy[i];
      this.head[p] = nh;
      if (this.filled[p] < TRAIL) this.filled[p]++;
    }
  }

  private draw() {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.w, this.h);
    // One path per colour and half of the trail: the older half fainter.
    const buckets = WIND_STOPS.length;
    const paths: Path2D[][] = [Array.from({ length: buckets }, () => new Path2D()), Array.from({ length: buckets }, () => new Path2D())];
    for (let p = 0; p < this.n; p++) {
      const len = this.filled[p];
      if (len < 2) continue;
      const o = p * TRAIL * 2;
      const h = this.head[p];
      const i = this.cell(this.trail[o + h * 2], this.trail[o + h * 2 + 1]);
      if (i < 0) continue;
      const b = windBucket(this.kt[i]);
      // Two polylines: the older half of the trail, then the newer half.
      const mid = Math.floor(len / 2);
      const tail = (h - (len - 1) + TRAIL) % TRAIL;
      const old = paths[0][b];
      const fresh = paths[1][b];
      let a = tail;
      old.moveTo(this.trail[o + a * 2], this.trail[o + a * 2 + 1]);
      for (let s = 1; s < len; s++) {
        a = (a + 1) % TRAIL;
        const x = this.trail[o + a * 2];
        const y = this.trail[o + a * 2 + 1];
        if (s <= mid) old.lineTo(x, y);
        if (s === mid) fresh.moveTo(x, y);
        if (s > mid) fresh.lineTo(x, y);
      }
    }
    ctx.lineWidth = 1.6;
    for (let half = 0; half < 2; half++) {
      ctx.globalAlpha = half === 0 ? 0.35 : 0.85;
      for (let b = 0; b < buckets; b++) {
        ctx.strokeStyle = WIND_STOPS[b][1];
        ctx.stroke(paths[half][b]);
      }
    }
    ctx.globalAlpha = 1;
  }
}
