<script lang="ts">
  // The printed strip at the foot of the identity pages: a scalloped edge,
  // a fine cross-hatched band, a barcode and the document number.
  import { barcode } from "./paper";

  let { number }: { number: string } = $props();
  const bars = $derived(barcode(number, 150));
  // Scallops along the top edge, in a 360-unit-wide strip.
  const wave = $derived.by(() => {
    let d = "M0 10";
    for (let x = 0; x < 360; x += 30) d += ` Q${x + 15} 0 ${x + 30} 10`;
    return d;
  });
</script>

<div class="strip" aria-hidden="true">
  <svg class="edge" viewBox="0 0 360 12" preserveAspectRatio="none">
    <path d="{wave} V12 H0 Z" class="fill" />
    <path d={wave} class="line" />
  </svg>
  <div class="band">
    <svg class="bars" viewBox="0 0 150 24" preserveAspectRatio="none">
      {#each bars as [x, w], i (i)}<rect {x} y="0" width={w} height="24" />{/each}
    </svg>
    <span class="no">{number}</span>
  </div>
</div>

<style>
  .strip {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    pointer-events: none;
  }
  .edge {
    display: block;
    width: 100%;
    height: 10px;
  }
  .fill {
    fill: #a9abc8;
  }
  .line {
    fill: none;
    stroke: rgba(255, 255, 255, 0.85);
    stroke-width: 1.4;
    vector-effect: non-scaling-stroke;
  }
  .band {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 34px;
    padding: 0 18px 4px;
    background-color: #a9abc8;
    background-image:
      repeating-linear-gradient(45deg, rgba(255, 255, 255, 0.16) 0 1px, transparent 1px 3px),
      repeating-linear-gradient(-45deg, rgba(60, 60, 100, 0.12) 0 1px, transparent 1px 3px);
  }
  .bars {
    width: 118px;
    height: 22px;
    flex: none;
  }
  .bars rect {
    fill: #121218;
  }
  .no {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 15px;
    font-weight: 300;
    letter-spacing: 0.28em;
    color: #121218;
    white-space: nowrap;
    overflow: hidden;
  }
</style>
