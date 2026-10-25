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
  }
  let { world, visited, half, width }: Props = $props();

  const hw = $derived(world.w / 2);
  const viewBox = $derived(`${half === "left" ? 0 : hw} 0 ${hw} ${world.h}`);
  const height = $derived((width * world.h) / hw);

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

<svg {viewBox} {width} {height} class="world" role="img" aria-label="World map{half === 'left' ? '' : ', continued'}">
  <defs>
    <pattern id="ink-hatch-{half}" width="3" height="3" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
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
  {#each world.countries.filter((c) => c.dot && visited.has(c.c)) as c (c.c)}
    <circle cx={c.dot![0]} cy={c.dot![1]} r="3.2" class="dot"><title>{c.n}, visited</title></circle>
  {/each}
</svg>

<style>
  .world {
    display: block;
  }
  .sea {
    fill: rgba(159, 193, 189, 0.16);
    stroke: #9fb7b3;
    stroke-width: 0.8;
  }
  .grid {
    fill: none;
    stroke: #9fb7b3;
    stroke-width: 0.4;
    opacity: 0.7;
  }
  path:not(.sea):not(.grid) {
    fill: #e6dec9;
    stroke: #a99d81;
    stroke-width: 0.45;
    stroke-linejoin: round;
  }
  path.visited {
    stroke: #2f5a55;
    stroke-width: 0.6;
  }
  .dot {
    fill: #3f6f69;
    stroke: #f3eedf;
    stroke-width: 0.8;
  }
</style>
