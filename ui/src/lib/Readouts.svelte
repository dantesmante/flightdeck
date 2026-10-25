<script lang="ts">
  import type { LiveFlight, Telemetry } from "./api";
  import { dec, heading, hhmm, int, signed } from "./format";

  interface Props {
    telemetry: Telemetry | null;
    live: LiveFlight | null;
  }
  let { telemetry: t, live }: Props = $props();

  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => (now = t?.t_ms ?? Date.now()), 1000);
    return () => clearInterval(id);
  });

  const elapsed = $derived.by(() => {
    if (!live) return "–";
    const from = live.block_out_ms ?? live.start_ms;
    return hhmm(((t?.t_ms ?? now) - from) / 60000);
  });

  const items = $derived([
    { label: "Altitude", value: int(t?.alt_msl_ft), unit: "ft" },
    { label: "Ground speed", value: int(t?.gs_kt), unit: "kt" },
    { label: "Vertical speed", value: t ? signed(Math.abs(t.vs_fpm) < 50 ? 0 : t.vs_fpm) : "–", unit: "fpm" },
    { label: "Heading", value: heading(t?.hdg_true_deg), unit: "°T" },
    { label: "Distance flown", value: dec(live?.distance_nm), unit: "nm" },
    { label: live?.block_out_ms ? "Block time" : "Time", value: elapsed, unit: "" },
  ]);
</script>

<dl class="readouts">
  {#each items as item (item.label)}
    <div class="cell">
      <dt>{item.label}</dt>
      <dd><span class="num value">{item.value}</span><span class="unit">{item.unit}</span></dd>
    </div>
  {/each}
</dl>

<style>
  .readouts {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    margin: 0;
    border-top: 1px solid var(--rule);
    border-left: 1px solid var(--rule);
  }
  .cell {
    padding: 9px 11px 8px;
    border-right: 1px solid var(--rule);
    border-bottom: 1px solid var(--rule);
  }
  dt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  dd {
    margin: 2px 0 0;
    display: flex;
    align-items: baseline;
    gap: 4px;
  }
  .value {
    font-size: 26px;
    font-weight: 500;
    line-height: 1.05;
    color: var(--ink);
  }
  .unit {
    font-size: 12px;
    color: var(--ink-faint);
  }
</style>
