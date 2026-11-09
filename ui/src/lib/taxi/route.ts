// A taxi clearance turned into a path: the taxiways in the order ATC gave
// them, runways to cross or hold short of, and where it ends (a gate, or
// holding short of the departure runway). The path is the shortest one
// that follows the taxiways in that order: a search over (place, how far
// through the clearance) pairs, which only moves on to the next taxiway
// where the two meet. Then the path is cut into GPS-style steps.

import { distTo, nearestEdge, type TaxiGraph, type XY } from "./graph";

export type Item =
  | { t: "twy"; name: string }
  | { t: "rwy"; name: string; mode: "cross" | "hold" }
  | { t: "gate"; ref: string; xy: XY };

export interface Step {
  /** Metres from the start of the path where this step happens. */
  at: number;
  kind: "exit" | "turn" | "hold" | "cross" | "warn" | "gate" | "end";
  /** "left" | "right" | "straight" for turns. */
  dir?: "left" | "right" | "straight";
  text: string;
  /** For holds and crossings: the runway. */
  runway?: string;
}

export interface Hold {
  runway: string;
  at: number;
  xy: XY;
  /** Direction of travel there (unit vector), to draw the bar across. */
  dirXY: XY;
}

export interface TaxiRoute {
  /** Points of the path, in local metres, with the distance along at each. */
  pts: XY[];
  along: number[];
  length: number;
  steps: Step[];
  holds: Hold[];
  /** Runways crossed that the clearance does not mention. */
  uncleared: { runway: string; at: number; xy: XY }[];
  /** Why there is no full route (shown under the bubbles), or "". */
  problem: string;
  /** The last taxiway when the route stops where it joins it (no destination). */
  openEnd: string;
}

/** Fallback hold line: this far from the runway centreline (FAA: 250 ft for most airliner runways). */
const HOLD_FROM_CL_M = 76;
const UNNAMED_LINK_M = 120;

class Heap {
  private a: { k: number; s: number }[] = [];
  get size() {
    return this.a.length;
  }
  push(k: number, s: number) {
    const a = this.a;
    a.push({ k, s });
    let i = a.length - 1;
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (a[p].k <= a[i].k) break;
      [a[p], a[i]] = [a[i], a[p]];
      i = p;
    }
  }
  pop(): { k: number; s: number } {
    const a = this.a;
    const top = a[0];
    const last = a.pop()!;
    if (a.length) {
      a[0] = last;
      let i = 0;
      for (;;) {
        const l = 2 * i + 1;
        const r = l + 1;
        let m = i;
        if (l < a.length && a[l].k < a[m].k) m = l;
        if (r < a.length && a[r].k < a[m].k) m = r;
        if (m === i) break;
        [a[m], a[i]] = [a[i], a[m]];
        i = m;
      }
    }
    return top;
  }
}

/**
 * The route for a clearance from where the aircraft is (`from`, local
 * metres) heading `hdg` (degrees true).
 */
/**
 * `holdPts` are the hold lines mapped in OpenStreetMap (holding_position
 * points on the taxiways), used where the route passes over one.
 */
export function routeFor(g: TaxiGraph, items: Item[], from: XY, hdg: number, holdPts: XY[] = []): TaxiRoute {
  const empty = (problem: string, openEnd = ""): TaxiRoute => ({ pts: [], along: [], length: 0, steps: [], holds: [], uncleared: [], problem, openEnd });
  const T = items.filter((i): i is Extract<Item, { t: "twy" }> => i.t === "twy").map((i) => i.name);
  if (!T.length) return empty("");
  const last = items[items.length - 1];
  const gate = last.t === "gate" ? last : null;
  const holdEnd = last.t === "rwy" && last.mode === "hold" ? last.name : null;
  const n = T.length;
  const L = n + 2; // states: 0 = before the first taxiway, 1..n = on T[i-1], n+1 = to the gate

  const start = nearestEdge(g, from, 150);
  if (!start) return empty("No taxiway near the aircraft");
  const se = g.edges[start.edge];
  const startName = se.name;
  const startRunway = se.runway;

  // Virtual start node: the point on the start edge.
  const N = g.nodes.length;
  const S = N; // id of the start node
  const nodeXY = (id: number): XY => (id === S ? start.at : g.nodes[id]);
  const hx = Math.sin((hdg * Math.PI) / 180);
  const hy = Math.cos((hdg * Math.PI) / 180);
  const back = (to: XY) => {
    const dx = to[0] - start.at[0];
    const dy = to[1] - start.at[1];
    const d = Math.hypot(dx, dy) || 1;
    return (dx * hx + dy * hy) / d < -0.3 ? 400 : 0; // turning round costs
  };

  const dist = new Float64Array((N + 1) * L).fill(Infinity);
  const prev = new Int32Array((N + 1) * L).fill(-1);
  const heap = new Heap();
  const idx = (node: number, st: number) => node * L + st;

  // Which state taking an edge leads to from state st, or -1.
  const stepState = (st: number, e: { name: string; runway: boolean; len: number }): number => {
    if (st === 0) {
      if (!e.runway && e.name === T[0]) return 1;
      if (e.name === startName && e.runway === startRunway) return 0;
      if (!e.runway && !e.name) return 0;
      return -1;
    }
    if (st <= n) {
      if (e.runway) return -1;
      if (e.name === T[st - 1]) return st;
      if (st < n && e.name === T[st]) return st + 1;
      if (!e.name && e.len <= UNNAMED_LINK_M) return st;
      return -1;
    }
    // To the gate: aprons and taxilanes.
    if (e.runway) return -1;
    return st;
  };
  // Unnamed links (corner fillets, connectors) are fine between taxiways
  // but cost more, so the route does not cut across aprons.
  const costOf = (st: number, e: { name: string; len: number }) =>
    st === n + 1 ? (e.name && e.name !== T[n - 1] ? e.len * 1.5 : e.len) : st >= 1 && !e.name ? e.len * 2.5 : e.len;

  const relax = (node: number, st: number, d: number, from: number) => {
    const k = idx(node, st);
    if (d < dist[k]) {
      dist[k] = d;
      prev[k] = from;
      heap.push(d, k);
    }
  };

  // From the start point along its edge, both ways.
  for (const [end, frac] of [
    [se.a, start.t],
    [se.b, 1 - start.t],
  ] as const) {
    const st0 = stepState(0, se);
    if (st0 < 0) continue;
    relax(end, st0, se.len * frac + back(g.nodes[end]), idx(S, 0));
  }
  dist[idx(S, 0)] = 0;

  const touches = T.map((name) => {
    const set = new Set<number>();
    for (const ei of g.byName.get(name) ?? []) if (!g.edges[ei].runway) set.add(g.edges[ei].a).add(g.edges[ei].b);
    return set;
  });
  const GOAL = (N + 1) * L; // virtual goal for gates
  let goalCost = Infinity;
  let goalFrom = -1;
  let found = -1;
  while (heap.size) {
    const { k, s } = heap.pop();
    if (s === GOAL) {
      found = GOAL;
      break;
    }
    if (k > dist[s]) continue;
    const node = Math.floor(s / L);
    const st = s % L;
    const here = nodeXY(node);
    // Goals.
    if (gate && st === n + 1) {
      const d = Math.hypot(gate.xy[0] - here[0], gate.xy[1] - here[1]);
      if (d < 250) {
        const c = k + d * 1.2;
        if (c < goalCost) {
          goalCost = c;
          goalFrom = s;
          heap.push(c, GOAL);
        }
      }
    } else if (!gate && st === n) {
      if (holdEnd ? g.runwayAt.get(node)?.has(holdEnd) : node !== S) {
        found = s;
        break;
      }
    }
    if (gate && st === n) relax(node, n + 1, k, s);
    // At a junction with the next taxiway: move on to it right here (it
    // may only be crossed: "C3, B, A3" where all three meet at one point).
    if (st < n && node !== S && touches[st].has(node)) relax(node, st + 1, k, s);
    if (node === S) continue;
    for (const ei of g.adj[node]) {
      const e = g.edges[ei];
      const ns = stepState(st, e);
      if (ns < 0) continue;
      relax(e.a === node ? e.b : e.a, ns, k + costOf(ns, e), s);
    }
  }
  if (found < 0) {
    // Say which part of the clearance could not be followed.
    let reached = 0;
    for (let node = 0; node < N; node++) for (let st = 0; st < L; st++) if (dist[idx(node, st)] < Infinity) reached = Math.max(reached, st);
    if (reached < 1) return empty(`Can't reach ${T[0]} from here`);
    if (reached <= n - 1) return empty(`${T[reached - 1]} doesn't meet ${T[reached]}`);
    if (gate) return empty(`No way found from ${T[n - 1]} to gate ${gate.ref}`);
    if (holdEnd) return empty(`${T[n - 1]} doesn't reach runway ${holdEnd}`);
    return empty("No route found");
  }

  // Walk back.
  const chain: number[] = [];
  let cur = found === GOAL ? goalFrom : found;
  while (cur >= 0) {
    chain.push(cur);
    cur = prev[cur];
  }
  chain.reverse();
  const pts: XY[] = [];
  // The state of the edge arriving at each point (moving on to the next
  // taxiway at a point shows on the edge leaving it).
  const states: number[] = [];
  // Whether the edge arriving at each point has a name (not a fillet).
  const named: boolean[] = [];
  let prevNode = -1;
  for (const s of chain) {
    const node = Math.floor(s / L);
    const p = nodeXY(node);
    const last = pts[pts.length - 1];
    if (last && last[0] === p[0] && last[1] === p[1]) {
      prevNode = node;
      continue;
    }
    let isNamed = !!se.name;
    if (prevNode >= 0 && prevNode !== S) isNamed = g.adj[prevNode].some((ei) => (g.edges[ei].a === node || g.edges[ei].b === node) && !!g.edges[ei].name);
    pts.push(p);
    states.push(s % L);
    named.push(isNamed);
    prevNode = node;
  }
  if (gate) {
    pts.push(gate.xy);
    states.push(n + 1);
    named.push(false);
  }
  // The turn at point k: from the direction on the last named taxiway
  // before it (past any curved corner fillet) to the way ahead.
  const turnAt = (k: number): "left" | "right" | "straight" => {
    let j = k;
    while (j > 0 && !named[j] && along[k] - along[j] < 300) j--;
    if (j <= 0 || !named[j]) return turnDir(pts, along, along[k]);
    const v1 = [pts[j][0] - pts[j - 1][0], pts[j][1] - pts[j - 1][1]];
    const ahead = pointAt(pts, along, Math.min(along[along.length - 1], along[k] + 40)).xy;
    const v2 = [ahead[0] - pts[k][0], ahead[1] - pts[k][1]];
    if (Math.hypot(v1[0], v1[1]) < 0.5 || Math.hypot(v2[0], v2[1]) < 1) return turnDir(pts, along, along[k]);
    const ang = (Math.atan2(v1[0] * v2[1] - v1[1] * v2[0], v1[0] * v2[0] + v1[1] * v2[1]) * 180) / Math.PI;
    return ang > 25 ? "left" : ang < -25 ? "right" : "straight";
  };
  const along = [0];
  for (let i = 1; i < pts.length; i++) along.push(along[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const length = along[along.length - 1];

  // Runway crossings along the way, matched to the clearance in order.
  const nodesOnPath = chain.map((s) => Math.floor(s / L));
  const crossings: { runway: string; at: number; i: number }[] = [];
  {
    let pi = 0;
    for (let c = 0; c < nodesOnPath.length; c++) {
      const p = nodeXY(nodesOnPath[c]);
      while (pi < pts.length - 1 && (pts[pi][0] !== p[0] || pts[pi][1] !== p[1])) pi++;
      const rs = g.runwayAt.get(nodesOnPath[c]);
      if (!rs || nodesOnPath[c] === S) continue;
      for (const r of rs) {
        // Rolling along and leaving the runway we landed on is not a crossing.
        if (startRunway && r === startName && states[pi] === 0) continue;
        const prevSame = crossings.find((x) => x.runway === r && along[pi] - x.at < 120);
        if (!prevSame) crossings.push({ runway: r, at: along[pi], i: pi });
      }
    }
  }

  // Where to stop before a runway crossing at `x`: the mapped hold line the
  // route passes over within 250 m before it, else the point 76 m from the
  // runway's centreline (measured square to the runway, not along the path).
  const holdBefore = (x: { runway: string; at: number }): number => {
    let best = -1;
    for (const h of holdPts) {
      for (let i = 1; i < pts.length; i++) {
        if (along[i] < x.at - 250 || along[i - 1] > x.at - 3) continue;
        const a = pts[i - 1];
        const b = pts[i];
        const dx = b[0] - a[0];
        const dy = b[1] - a[1];
        const l2 = dx * dx + dy * dy || 1;
        const t = Math.max(0, Math.min(1, ((h[0] - a[0]) * dx + (h[1] - a[1]) * dy) / l2));
        const off = Math.hypot(a[0] + t * dx - h[0], a[1] + t * dy - h[1]);
        const at = along[i - 1] + t * Math.sqrt(l2);
        if (off <= 12 && at <= x.at - 3 && at > best) best = at;
      }
    }
    if (best >= 0) return best;
    for (let at = x.at; at > Math.max(0, x.at - 250); at -= 2) if (distTo(g, x.runway, pointAt(pts, along, at).xy) >= HOLD_FROM_CL_M) return at;
    return Math.max(0, x.at - HOLD_FROM_CL_M);
  };
  const holds: Hold[] = [];
  const uncleared: TaxiRoute["uncleared"] = [];
  const steps: Step[] = [];
  const rwyItems = items.filter((i): i is Extract<Item, { t: "rwy" }> => i.t === "rwy");
  const used = new Set<number>();
  for (const it of rwyItems) {
    const c = crossings.findIndex((x, j) => !used.has(j) && x.runway === it.name);
    if (c < 0) {
      steps.push({ at: 0, kind: "warn", text: `Runway ${it.name} is not on this route`, runway: it.name });
      continue;
    }
    used.add(c);
    const x = crossings[c];
    const hAt = holdBefore(x);
    if (it.mode === "hold") {
      const at = hAt;
      const { xy, dir } = pointAt(pts, along, at);
      holds.push({ runway: it.name, at, xy, dirXY: dir });
      steps.push({ at, kind: "hold", text: `Hold short RWY ${it.name}`, runway: it.name });
    } else {
      steps.push({ at: hAt, kind: "cross", text: `Cross RWY ${it.name}`, runway: it.name });
    }
  }
  crossings.forEach((x, j) => {
    if (used.has(j)) return;
    uncleared.push({ runway: x.runway, at: x.at, xy: pts[x.i] });
    steps.push({ at: holdBefore(x), kind: "warn", text: `RWY ${x.runway} ahead, not in your clearance`, runway: x.runway });
  });

  // Turns: wherever the state moves on to the next taxiway.
  for (let i = 1; i < pts.length; i++) {
    if (states[i] === states[i - 1] || states[i] > n || states[i] < 1) continue;
    // The edge into pts[i] is on the new taxiway: the change is at pts[i-1].
    const at = along[i - 1];
    // Taxiways only crossed on the way (several states at one point).
    for (let k = states[i - 1] + 1; k < states[i]; k++) if (k >= 1) steps.push({ at, kind: "turn", dir: "straight", text: `Straight across ${T[k - 1]}` });
    const name = T[states[i] - 1];
    const first = states[i] === 1;
    if (first && startRunway) {
      steps.push({ at, kind: "exit", text: `Exit via ${name}`, dir: turnAt(i - 1) });
      continue;
    }
    const dir = first && at < 15 ? "straight" : turnAt(i - 1);
    const verb = dir === "left" ? "Left on" : dir === "right" ? "Right on" : first ? "Taxi via" : "Continue on";
    steps.push({ at, kind: "turn", dir, text: `${verb} ${name}` });
  }
  if (gate) steps.push({ at: length, kind: "gate", text: `Gate ${gate.ref}` });
  if (!gate && !holdEnd && !steps.some((x) => x.at >= length - 1 && x.text.endsWith(` ${T[n - 1]}`))) steps.push({ at: length, kind: "end", text: `Join ${T[n - 1]}` });
  steps.sort((a, b) => a.at - b.at);

  // A route ending at "hold short": stop the line at the hold bar.
  if (holdEnd) {
    const h = [...holds].reverse().find((x) => x.runway === holdEnd);
    if (h) cutAt(pts, along, h.at);
  }
  return {
    pts,
    along,
    length: along[along.length - 1],
    steps,
    holds,
    uncleared,
    problem: "",
    openEnd: !gate && !holdEnd ? T[n - 1] : "",
  };
}

function cutAt(pts: XY[], along: number[], at: number) {
  const { xy } = pointAt(pts, along, at);
  let i = 0;
  while (i < along.length && along[i] < at) i++;
  pts.splice(i, pts.length - i, xy);
  along.splice(i, along.length - i, at);
}

/** The point `at` metres along, and the direction of travel there. */
export function pointAt(pts: XY[], along: number[], at: number): { xy: XY; dir: XY } {
  let i = 1;
  while (i < along.length - 1 && along[i] < at) i++;
  const a = pts[i - 1] ?? pts[0];
  const b = pts[i] ?? pts[0];
  const seg = along[i] - along[i - 1] || 1;
  const t = Math.max(0, Math.min(1, (at - along[i - 1]) / seg));
  const d = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  return { xy: [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t], dir: [(b[0] - a[0]) / d, (b[1] - a[1]) / d] };
}

function turnDir(pts: XY[], along: number[], at: number): "left" | "right" | "straight" {
  const before = pointAt(pts, along, Math.max(0, at - 25));
  const here = pointAt(pts, along, at);
  const after = pointAt(pts, along, Math.min(along[along.length - 1], at + 25));
  const v1 = [here.xy[0] - before.xy[0], here.xy[1] - before.xy[1]];
  const v2 = [after.xy[0] - here.xy[0], after.xy[1] - here.xy[1]];
  if (Math.hypot(...v1) < 1 || Math.hypot(...v2) < 1) return "straight";
  const ang = (Math.atan2(v1[0] * v2[1] - v1[1] * v2[0], v1[0] * v2[0] + v1[1] * v2[1]) * 180) / Math.PI;
  // Positive = counter-clockwise (x east, y north) = left.
  return ang > 25 ? "left" : ang < -25 ? "right" : "straight";
}

/** Where the aircraft is along the route: metres along and how far off it. */
export function progress(r: TaxiRoute, p: XY, hint: number): { along: number; off: number } {
  // Near the last place first (so a route that passes close to itself
  // does not jump ahead), anywhere when that is far off.
  const near = closest(r, p, hint - 80, hint + 600);
  return near.off <= 40 ? near : (() => {
    const all = closest(r, p, -Infinity, Infinity);
    return all.off < near.off ? all : near;
  })();
}

function closest(r: TaxiRoute, p: XY, lo: number, hi: number): { along: number; off: number } {
  let best = { along: Math.max(0, lo), off: Infinity };
  for (let i = 1; i < r.pts.length; i++) {
    if (r.along[i] < lo || r.along[i - 1] > hi) continue;
    const a = r.pts[i - 1];
    const b = r.pts[i];
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const l2 = dx * dx + dy * dy || 1;
    const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2));
    const off = Math.hypot(a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    if (off < best.off) best = { along: r.along[i - 1] + t * Math.sqrt(l2), off };
  }
  return best;
}
