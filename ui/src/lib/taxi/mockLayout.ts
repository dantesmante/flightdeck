// Test only: a made-up taxi layout at Portland (KPDX), laid out from the
// mock's runway ends, so Taxi assist can be tried without map tiles.
// Lines are cut into overlapping pieces the way map tiles cut them.

import { toLL, toXY, type LL, type RawFeature, type XY } from "./graph";

const S: [LL, LL] = [
  [-122.6094, 45.5889],
  [-122.5671, 45.5823],
];
const N: [LL, LL] = [
  [-122.6012, 45.5942],
  [-122.5693, 45.5891],
];

export function mockTaxiLayout(at: LL): RawFeature[] | null {
  const o = S[0];
  if (Math.abs(at[0] - o[0]) > 0.1 || Math.abs(at[1] - o[1]) > 0.1) return null;
  const xy = (p: LL) => toXY(o, p);
  const s0 = xy(S[0]);
  const s1 = xy(S[1]);
  const n0 = xy(N[0]);
  const n1 = xy(N[1]);
  // A point along a runway at fraction t, `off` metres to its left (north).
  const along = (a: XY, b: XY, t: number, off: number): XY => {
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const d = Math.hypot(dx, dy);
    return [a[0] + dx * t - (dy / d) * off, a[1] + dy * t + (dx / d) * off];
  };
  const onS = (t: number, off = 0) => along(s0, s1, t, off);
  const onN = (t: number, off = 0) => along(n0, n1, t, off);
  // The fraction along N at the same x as fraction t along S.
  const tN = (t: number) => (onS(t)[0] - n0[0]) / (n1[0] - n0[0]);
  const out: RawFeature[] = [];
  const line = (kind: "taxiway" | "runway", ref: string, pts: XY[], pieces = 1) => {
    // Cut long lines into overlapping pieces, like tiles do.
    const ll = pts.map((p) => toLL(o, p));
    if (pieces === 1 || ll.length !== 2) {
      out.push({ kind, ref, coords: ll });
      return;
    }
    for (let i = 0; i < pieces; i++) {
      const a = Math.max(0, i / pieces - 0.02);
      const b = Math.min(1, (i + 1) / pieces + 0.02);
      const p = (t: number): LL => [ll[0][0] + (ll[1][0] - ll[0][0]) * t, ll[0][1] + (ll[1][1] - ll[0][1]) * t];
      out.push({ kind, ref, coords: [p(a), p(b)] });
    }
  };
  line("runway", "10R/28L", [s0, s1], 3);
  line("runway", "10L/28R", [n0, n1], 3);
  // Parallels, as polylines through points every 10 %.
  const par = (f: (t: number) => XY, a: number, b: number) => {
    const p: XY[] = [];
    for (let t = a; t <= b + 1e-9; t += 0.1) p.push(f(Math.min(b, t)));
    return p;
  };
  const A = par((t) => onS(t, -180), 0.02, 0.98);
  const B = par((t) => onS(t, 230), 0.05, 0.95);
  const C = par((t) => onN(t, 170), 0.04, 0.96);
  line("taxiway", "A", A);
  line("taxiway", "B", B);
  line("taxiway", "C", C);
  // High-speed exits from 10R/28L to B, and plain connectors to A.
  for (const [ref, t] of [
    ["B2", 0.2],
    ["B4", 0.4],
    ["B6", 0.6],
    ["B8", 0.8],
  ] as const)
    line("taxiway", ref, [onS(t, -180), onS(t), onS(t + 0.035, 230)]);
  for (const [ref, t] of [
    ["A3", 0.3],
    ["A7", 0.7],
  ] as const)
    line("taxiway", ref, [onS(t, -180), onS(t, 230)]);
  // Crossing 10L/28R from B to C.
  for (const [ref, t] of [
    ["C3", 0.3],
    ["C5", 0.5],
    ["C7", 0.7],
  ] as const) {
    const u = tN(t);
    line("taxiway", ref, [onS(t, 230), onN(u), onN(u, 170)]);
  }
  // The terminal: an unnamed taxilane north of C, gates along it.
  const lane = par((t) => onN(t, 290), 0.3, 0.7);
  line("taxiway", "", lane);
  line("taxiway", "", [onN(0.3, 170), onN(0.3, 290)]);
  line("taxiway", "", [onN(0.7, 170), onN(0.7, 290)]);
  for (let i = 0; i < 8; i++) {
    const t = 0.32 + i * 0.05;
    out.push({ kind: "gate", ref: `D${i + 1}`, coords: [toLL(o, onN(t, 345))] });
    line("taxiway", "", [onN(t, 290), onN(t, 335)]);
  }
  return out;
}
