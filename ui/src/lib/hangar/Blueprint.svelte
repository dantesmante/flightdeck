<script lang="ts">
  // A top-down blueprint of the aircraft: white line work on blueprint
  // blue, drawn from a few measurements per kind of aircraft, so every type
  // gets a fitting plan without any image files.
  import { artKind, type ArtKind } from "./kind";

  interface Props {
    icao_type: string;
    engine_type: string;
    engine_count: number;
    title: string;
  }
  let { icao_type, engine_type, engine_count, title }: Props = $props();

  const kind = $derived<ArtKind>(artKind({ icao_type, engine_type, engine_count, title }));

  type Spec = {
    len: number; // fuselage length
    w: number; // fuselage width
    span: number;
    sweep: number; // degrees
    root: number; // wing root chord
    tip: number; // wing tip chord
    wingAt: number; // wing root position along the fuselage, 0..1
    tail: number; // tailplane span
    engines: "under" | "rear" | "wingprop" | "noseprop" | "none";
    count: number;
    tTail?: boolean;
  };

  const SPECS: Record<Exclude<ArtKind, "helicopter">, Spec> = {
    widebody: { len: 176, w: 15, span: 184, sweep: 31, root: 40, tip: 9, wingAt: 0.4, tail: 66, engines: "under", count: 2 },
    airliner: { len: 166, w: 12, span: 158, sweep: 26, root: 34, tip: 8, wingAt: 0.4, tail: 56, engines: "under", count: 2 },
    regional: { len: 160, w: 10, span: 128, sweep: 24, root: 28, tip: 7, wingAt: 0.42, tail: 48, engines: "rear", count: 2, tTail: true },
    bizjet: { len: 128, w: 9, span: 112, sweep: 22, root: 24, tip: 7, wingAt: 0.46, tail: 40, engines: "rear", count: 2, tTail: true },
    "turboprop-twin": { len: 140, w: 10, span: 160, sweep: 3, root: 22, tip: 12, wingAt: 0.36, tail: 54, engines: "wingprop", count: 2, tTail: true },
    "turboprop-single": { len: 124, w: 10, span: 150, sweep: 0, root: 21, tip: 15, wingAt: 0.3, tail: 50, engines: "noseprop", count: 1 },
    "piston-twin": { len: 112, w: 9, span: 128, sweep: 2, root: 19, tip: 12, wingAt: 0.34, tail: 44, engines: "wingprop", count: 2 },
    "piston-high": { len: 104, w: 8, span: 136, sweep: 0, root: 18, tip: 15, wingAt: 0.28, tail: 42, engines: "noseprop", count: 1 },
    "piston-low": { len: 104, w: 8, span: 128, sweep: 2, root: 20, tip: 12, wingAt: 0.32, tail: 42, engines: "noseprop", count: 1 },
    glider: { len: 112, w: 6, span: 192, sweep: 0, root: 14, tip: 5, wingAt: 0.3, tail: 34, engines: "none", count: 0, tTail: true },
  };

  const CX = 100;
  const f = (n: number) => n.toFixed(1);

  const plan = $derived.by(() => {
    if (kind === "helicopter") return null;
    const s = { ...SPECS[kind] };
    if (kind === "widebody" && engine_count >= 4) s.count = 4;
    const top = (200 - s.len) / 2;
    const r = s.w / 2;
    const noseEnd = top + s.w * 1.7;
    const tailEnd = top + s.len;
    const taper = tailEnd - s.len * 0.2;
    const fuselage = [
      `M${f(CX)} ${f(top)}`,
      `C${f(CX + r * 0.65)} ${f(top)} ${f(CX + r)} ${f(top + s.w * 0.6)} ${f(CX + r)} ${f(noseEnd)}`,
      `L${f(CX + r)} ${f(taper)}`,
      `Q${f(CX + r * 0.8)} ${f(tailEnd - 2)} ${f(CX + r * 0.18)} ${f(tailEnd)}`,
      `L${f(CX - r * 0.18)} ${f(tailEnd)}`,
      `Q${f(CX - r * 0.8)} ${f(tailEnd - 2)} ${f(CX - r)} ${f(taper)}`,
      `L${f(CX - r)} ${f(noseEnd)}`,
      `C${f(CX - r)} ${f(top + s.w * 0.6)} ${f(CX - r * 0.65)} ${f(top)} ${f(CX)} ${f(top)} Z`,
    ].join(" ");

    // A swept surface on both sides: root at (x0, y0), tip at half span.
    const surface = (y0: number, half: number, sweep: number, root: number, tip: number, x0: number) => {
      const k = Math.tan((sweep * Math.PI) / 180);
      const tipLead = y0 + k * (half - x0);
      // Trailing edge kinks slightly at a third of the span on swept wings.
      const side = (sgn: number) =>
        `M${f(CX + sgn * x0)} ${f(y0)} L${f(CX + sgn * half)} ${f(tipLead)} ` +
        `L${f(CX + sgn * half)} ${f(tipLead + tip)} ` +
        (sweep > 10
          ? `L${f(CX + sgn * (x0 + (half - x0) * 0.33))} ${f(y0 + k * (half - x0) * 0.33 + root * 0.62)} `
          : "") +
        `L${f(CX + sgn * x0)} ${f(y0 + root)}`;
      return { d: `${side(1)} ${side(-1)}`, k, tipLead };
    };

    const wy = top + s.len * s.wingAt;
    const wing = surface(wy, s.span / 2, s.sweep, s.root, s.tip, r);
    // Flap and aileron line along the trailing edge.
    const flapK = wing.k;
    const flaps = [1, -1]
      .map((sgn) => {
        const a = r + 3;
        const b = s.span / 2 - 6;
        const y = (x: number) => wy + flapK * (x - r) + s.root * (1 - (x - r) / (s.span / 2 - r)) * 0.88 + s.tip * ((x - r) / (s.span / 2 - r)) * 0.6;
        return `M${f(CX + sgn * a)} ${f(y(a))} L${f(CX + sgn * b)} ${f(y(b))}`;
      })
      .join(" ");

    const ty = tailEnd - s.len * (s.tTail ? 0.11 : 0.15);
    const tailplane = surface(ty, s.tail / 2, s.sweep * 0.9 + 4, s.root * 0.42, s.tip * 0.7, r * 0.5);
    // Fin seen from above: a line down the tail.
    const fin = `M${f(CX)} ${f(ty - s.len * 0.07)} L${f(CX)} ${f(tailEnd + 1)}`;

    // Engines.
    const pods: string[] = [];
    const props: string[] = [];
    if (s.engines === "under") {
      const at = s.count >= 4 ? [0.3, 0.62] : [0.36];
      for (const frac of at) {
        for (const sgn of [1, -1]) {
          const x = r + (s.span / 2 - r) * frac;
          const lead = wy + wing.k * (x - r);
          const L = s.w * 1.55;
          const W = s.w * 0.62;
          pods.push(`M${f(CX + sgn * x - W / 2)} ${f(lead - L * 0.55)} h${f(W)} v${f(L)} h${f(-W)} Z`);
          pods.push(`M${f(CX + sgn * x - W / 2)} ${f(lead - L * 0.55 + 2.5)} h${f(W)}`);
        }
      }
    } else if (s.engines === "rear") {
      for (const sgn of [1, -1]) {
        const x = CX + sgn * (r + s.w * 0.5);
        const y = top + s.len * 0.7;
        const L = s.w * 2;
        const W = s.w * 0.75;
        pods.push(`M${f(x - W / 2)} ${f(y)} h${f(W)} v${f(L)} h${f(-W)} Z`);
        pods.push(`M${f(x - W / 2)} ${f(y + 2.5)} h${f(W)}`);
      }
    } else if (s.engines === "wingprop") {
      for (const sgn of [1, -1]) {
        const x = CX + sgn * (r + (s.span / 2 - r) * 0.32);
        const L = s.root * 1.4;
        const W = s.w * 0.8;
        const y = wy - L * 0.32;
        pods.push(`M${f(x - W / 2)} ${f(y + 3)} Q${f(x)} ${f(y - 2)} ${f(x + W / 2)} ${f(y + 3)} V${f(y + L)} Q${f(x)} ${f(y + L + 4)} ${f(x - W / 2)} ${f(y + L)} Z`);
        props.push(`M${f(x - s.span * 0.11)} ${f(y - 2)} L${f(x + s.span * 0.11)} ${f(y - 2)}`);
      }
    } else if (s.engines === "noseprop") {
      props.push(`M${f(CX - s.span * 0.14)} ${f(top - 2)} L${f(CX + s.span * 0.14)} ${f(top - 2)}`);
    }

    // Cockpit windscreen and a row of cabin windows on airliners.
    const wind = `M${f(CX - r * 0.7)} ${f(top + s.w * 1.05)} Q${f(CX)} ${f(top + s.w * 0.7)} ${f(CX + r * 0.7)} ${f(top + s.w * 1.05)}`;
    const windows: string[] = [];
    if (["widebody", "airliner", "regional"].includes(kind)) {
      for (let y = noseEnd + 6; y < taper - 4; y += 5.5) {
        if (Math.abs(y - (wy + s.root / 2)) < s.root / 2 + 2) continue;
        windows.push(`M${f(CX - r + 1.6)} ${f(y)} v1.6 M${f(CX + r - 1.6)} ${f(y)} v1.6`);
      }
    }
    return {
      fuselage,
      wing: wing.d,
      flaps,
      tailplane: tailplane.d,
      fin,
      pods: pods.join(" "),
      props: props.join(" "),
      wind,
      windows: windows.join(" "),
      center: `M${f(CX)} ${f(top - 8)} L${f(CX)} ${f(tailEnd + 8)}`,
      spinner: s.engines === "noseprop" ? { x: CX, y: top - 1 } : null,
    };
  });
</script>

<svg class="bp" viewBox="0 0 200 200" role="img" aria-label="Blueprint of the {title || icao_type || 'aircraft'}">
  <defs>
    <pattern id="bp-grid" width="10" height="10" patternUnits="userSpaceOnUse">
      <path d="M10 0H0V10" fill="none" stroke="#fff" stroke-opacity="0.07" stroke-width="0.5" />
    </pattern>
  </defs>
  <rect x="-50" y="-50" width="300" height="300" fill="url(#bp-grid)" />
  {#if plan}
    <g fill="none" stroke="#fff" stroke-linejoin="round" stroke-linecap="round">
      <path d={plan.center} stroke-opacity="0.22" stroke-width="0.6" stroke-dasharray="4 3" />
      <path d={plan.wing} stroke-width="1.3" />
      <path d={plan.flaps} stroke-width="0.6" stroke-opacity="0.7" />
      <path d={plan.tailplane} stroke-width="1.2" />
      <path d={plan.pods} stroke-width="1.1" />
      <path d={plan.props} stroke-width="0.9" stroke-opacity="0.75" stroke-dasharray="1.5 1.5" />
      <path d={plan.fuselage} stroke-width="1.4" fill="#174a9f" />
      <path d={plan.fin} stroke-width="1.1" />
      <path d={plan.wind} stroke-width="0.9" />
      <path d={plan.windows} stroke-width="0.7" stroke-opacity="0.7" />
      {#if plan.spinner}<circle cx={plan.spinner.x} cy={plan.spinner.y} r="2" stroke-width="1" />{/if}
    </g>
  {:else}
    <!-- Helicopter: cabin, tail boom, main rotor disc and tail rotor. -->
    <g fill="none" stroke="#fff" stroke-linejoin="round" stroke-linecap="round">
      <circle cx="100" cy="86" r="62" stroke-width="0.8" stroke-opacity="0.45" stroke-dasharray="3 3" />
      <path d="M100 86 L52 38 M100 86 L148 134 M100 86 L148 38 M100 86 L52 134" stroke-width="1.2" />
      <path d="M100 52 C 114 52 118 70 118 88 C 118 106 110 116 100 116 C 90 116 82 106 82 88 C 82 70 86 52 100 52 Z" stroke-width="1.4" fill="#174a9f" />
      <path d="M96 116 L97 172 L103 172 L104 116" stroke-width="1.2" />
      <path d="M88 160 L112 160" stroke-width="1.1" />
      <path d="M106 168 L106 182 M100 175 L112 175" stroke-width="0.9" stroke-opacity="0.8" />
      <path d="M90 62 Q100 56 110 62" stroke-width="0.9" />
      <circle cx="100" cy="86" r="3" stroke-width="1" />
    </g>
  {/if}
</svg>

<style>
  .bp {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
