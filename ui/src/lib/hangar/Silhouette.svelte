<script lang="ts">
  // A plain side-view silhouette, shown when there is no photo of the
  // aircraft: one shape per kind of aircraft.
  import { artKind, regionalTurboprop } from "./kind";

  let { icao_type, engine_type, engine_count, title }: { icao_type: string; engine_type: string; engine_count: number; title: string } =
    $props();
  const kind = $derived(artKind({ icao_type, engine_type, engine_count, title }));
  const shape = $derived(
    kind === "helicopter" ? "heli" : kind === "bizjet" ? "bizjet" : ["widebody", "airliner", "regional"].includes(kind) ? "jet" : kind === "glider" ? "glider" : kind === "turboprop-twin" && regionalTurboprop(icao_type) ? "tprop" : kind.includes("twin") ? "twin" : "single",
  );
</script>

<svg viewBox="0 0 320 160" role="img" aria-label="No photo yet">
  <g fill="currentColor">
    {#if shape === "jet"}
      <path d="M26 86 C26 73 42 68 66 67 L262 66 L298 30 L310 30 L304 70 L290 90 L66 94 C42 94 26 92 26 86 Z" />
      <path d="M118 88 L206 92 L150 99 Z" opacity="0.85" />
      <ellipse cx="144" cy="101" rx="25" ry="9" />
      <path d="M262 70 L300 66 L312 72 L270 76 Z" opacity="0.8" />
    {:else if shape === "bizjet"}
      <path d="M40 88 C40 78 54 73 76 72 L236 72 L262 66 L268 70 L254 92 L80 96 C56 96 40 94 40 88 Z" />
      <path d="M244 72 L270 30 L282 30 L270 72 Z" />
      <path d="M256 30 L298 26 L300 32 L262 36 Z" />
      <rect x="204" y="66" width="40" height="13" rx="6" />
      <path d="M126 90 L196 93 L150 99 Z" opacity="0.85" />
    {:else if shape === "heli"}
      <path d="M70 92 C70 70 92 60 122 60 L160 62 C176 64 184 76 184 92 C184 104 170 110 150 110 L96 110 C80 110 70 104 70 92 Z" />
      <path d="M180 78 L286 72 L288 80 L182 90 Z" />
      <path d="M278 52 L290 52 L296 84 L284 84 Z" />
      <rect x="40" y="46" width="220" height="5" rx="2.5" />
      <rect x="124" y="50" width="10" height="12" />
      <path d="M86 122 L180 122 L184 126 L82 126 Z M104 110 L100 122 M156 110 L160 122" stroke="currentColor" stroke-width="5" />
    {:else if shape === "tprop"}
      <path d="M24 88 C24 76 40 70 64 69 L256 68 L282 34 L294 34 L290 72 L276 92 L64 96 C40 96 24 94 24 88 Z" />
      <path d="M268 32 L312 28 L314 35 L272 39 Z" />
      <rect x="96" y="60" width="128" height="8" rx="4" />
      <path d="M118 66 L170 66 L170 78 L124 80 C116 80 114 72 118 66 Z" />
      <rect x="112" y="56" width="4" height="34" rx="2" />
      <path d="M120 96 L118 108 M210 94 L212 108" stroke="currentColor" stroke-width="4" />
      <circle cx="118" cy="110" r="5" /><circle cx="212" cy="110" r="5" />
    {:else if shape === "glider"}
      <path d="M40 90 C40 80 54 76 76 75 L260 80 L292 50 L302 50 L296 86 L80 96 C56 96 40 94 40 90 Z" />
      <rect x="40" y="70" width="240" height="5" rx="2.5" opacity="0.85" />
    {:else}
      <path d="M44 92 C44 82 54 78 72 76 L118 66 L250 76 L292 52 L302 52 L296 82 L284 92 L120 100 Z" />
      <rect x="84" y="58" width="112" height="7" rx="3" />
      <path d="M126 66 L140 90" stroke="currentColor" stroke-width="3" />
      <rect x="36" y="64" width="5" height="44" rx="2.5" />
      <path d="M104 100 L98 116 M150 98 L156 116" stroke="currentColor" stroke-width="4" />
      <circle cx="98" cy="118" r="6" /><circle cx="156" cy="118" r="6" />
      {#if shape === "twin"}<ellipse cx="128" cy="78" rx="22" ry="7" />{/if}
    {/if}
  </g>
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 100%;
    color: #464a51;
  }
</style>
