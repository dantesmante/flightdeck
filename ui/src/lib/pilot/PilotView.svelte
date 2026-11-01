<script lang="ts">
  // The pilot profile: banner and photo, rating, rank and XP, career
  // statistics and every flight's score.
  import { onMount } from "svelte";
  import { api, RANKS, type PilotProfile } from "../api";
  import { day, hoursMinutes, int } from "../format";
  import { shrinkImage } from "../image";
  import { distance } from "../prefs.svelte";
  import Epaulette from "./Epaulette.svelte";
  import FlightScores from "./FlightScores.svelte";
  import ScoreRing from "./ScoreRing.svelte";

  let { onflight, onsettings }: { onflight: (id: number) => void; onsettings: () => void } = $props();

  let data = $state<PilotProfile | null>(null);
  let error = $state<string | null>(null);
  let busy = $state<"banner" | "photo" | null>(null);
  let bannerInput = $state<HTMLInputElement | null>(null);
  let photoInput = $state<HTMLInputElement | null>(null);
  let showRanks = $state(false);

  async function load() {
    try {
      data = await api.pilotProfile();
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  async function pick(kind: "banner" | "photo", e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file || !data) return;
    busy = kind;
    error = null;
    try {
      if (kind === "banner") {
        const url = await shrinkImage(file, 2000, 0.85);
        await api.setBanner(url);
        data.banner = url;
      } else {
        const url = await shrinkImage(file, 600, 0.85);
        await api.setPilotPhoto(url);
        data.pilot.photo_url = url;
      }
    } catch (err) {
      error = String(err);
    } finally {
      busy = null;
    }
  }

  async function remove(kind: "banner" | "photo") {
    if (!data) return;
    error = null;
    try {
      if (kind === "banner") {
        await api.setBanner(null);
        data.banner = null;
      } else {
        await api.setPilotPhoto(null);
        data.pilot.photo_url = null;
      }
    } catch (err) {
      error = String(err);
    }
  }

  const r = $derived(data?.rating);
  const initials = $derived(
    (data?.pilot.name || "Pilot")
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join(""),
  );
  const lastDelta = $derived.by(() => {
    const f = r?.flights.find((x) => x.rating_after != null);
    return f ? f.rating_after! - f.rating_before! : null;
  });
  const progress = $derived.by(() => {
    if (!r) return 0;
    if (!r.next_rank) return 1;
    return Math.max(0, Math.min(1, (r.xp - r.rank.min_xp) / (r.next_rank.min_xp - r.rank.min_xp)));
  });

  // The rating over time, for the sparkline.
  const spark = $derived.by(() => {
    const h = r?.history ?? [];
    if (h.length < 2) return null;
    const W = 320;
    const H = 84;
    const vals = h.map((p) => p.rating);
    let lo = Math.min(...vals, 1000);
    let hi = Math.max(...vals, 1000);
    const pad = Math.max(4, (hi - lo) * 0.15);
    lo -= pad;
    hi += pad;
    const x = (i: number) => (i / (h.length - 1)) * W;
    const y = (v: number) => H - ((v - lo) / (hi - lo)) * H;
    const line = vals.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
    return { W, H, line, area: `${line} L${W} ${H} L0 ${H} Z`, base: y(1000), end: [x(h.length - 1), y(vals[vals.length - 1])] };
  });

  const maxMonth = $derived(Math.max(60, ...(data?.stats.months ?? []).map((m) => m.minutes)));
  /** Minutes as hours: one decimal below ten. */
  const hrs = (min: number) => (min / 60 < 10 ? (Math.round(min / 6) / 10).toString() : String(Math.round(min / 60)));
  const monthName = (m: string) => ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][Number(m.slice(5)) - 1];
  const maxType = $derived(Math.max(1, ...(data?.stats.top_types ?? []).map((t) => t.minutes)));
  const maxPort = $derived(Math.max(1, ...(data?.stats.top_airports ?? []).map((t) => t.count)));
  const route = (s: { dep_icao: string | null; arr_icao: string | null }) => `${s.dep_icao ?? "····"} → ${s.arr_icao ?? "····"}`;
</script>

<input bind:this={bannerInput} type="file" accept="image/*" hidden onchange={(e) => pick("banner", e)} />
<input bind:this={photoInput} type="file" accept="image/*" hidden onchange={(e) => pick("photo", e)} />

<div class="page">
  {#if !data}
    <p class="loading">{error ?? "Loading your profile…"}</p>
  {:else}
    {@const p = data.pilot}
    {@const s = data.stats}
    {@const t = data.totals}
    <header class="hero">
      <div class="banner" class:busy={busy === "banner"}>
        {#if data.banner}
          <img src={data.banner} alt="" />
        {:else}
          <svg class="dusk" viewBox="0 0 1200 260" preserveAspectRatio="xMidYMax slice" aria-hidden="true">
            <defs>
              <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#10141d" />
                <stop offset="0.55" stop-color="#1d2130" />
                <stop offset="0.82" stop-color="#4a2f2c" />
                <stop offset="1" stop-color="#b8682e" />
              </linearGradient>
              <radialGradient id="glow" cx="0.5" cy="1" r="0.6">
                <stop offset="0" stop-color="#f0b23a" stop-opacity="0.55" />
                <stop offset="1" stop-color="#f0b23a" stop-opacity="0" />
              </radialGradient>
              <linearGradient id="ground" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#14161b" />
                <stop offset="1" stop-color="#1c1e21" />
              </linearGradient>
            </defs>
            <rect width="1200" height="200" fill="url(#sky)" />
            <ellipse cx="600" cy="200" rx="560" ry="90" fill="url(#glow)" />
            {#each Array.from({ length: 40 }, (_, i) => i) as i (i)}
              <circle cx={(i * 337) % 1200} cy={(i * 71) % 120} r={i % 5 ? 0.8 : 1.3} fill="#fff" opacity={0.25 + (i % 4) * 0.12} />
            {/each}
            <rect y="198" width="1200" height="62" fill="url(#ground)" />
            <!-- A runway running away to the horizon. -->
            <g transform="translate(330 0)">
              <path d="M590 200 L610 200 L760 260 L440 260 Z" fill="#262a31" />
              {#each [0, 1, 2, 3, 4, 5] as i (i)}
                {@const a = (i * 2 + 1) / 12}
                {@const b = (i * 2 + 1.6) / 12}
                <path
                  d="M{600 - 1 * a} {200 + 60 * a * a} L{600 + 1 * a} {200 + 60 * a * a} L{600 + 3 * b} {200 + 60 * b * b} L{600 - 3 * b} {200 + 60 * b * b} Z"
                  fill="#e8e2cf"
                  opacity="0.75"
                />
              {/each}
              {#each Array.from({ length: 10 }, (_, i) => i) as i (i)}
                {@const k = (i + 1) / 10}
                <circle cx={590 - 150 * k} cy={200 + 60 * k} r={0.7 + k * 1.8} fill="#f6d47a" />
                <circle cx={610 + 150 * k} cy={200 + 60 * k} r={0.7 + k * 1.8} fill="#f6d47a" />
              {/each}
            </g>
          </svg>
        {/if}
        <div class="shade"></div>
        <div class="banner-tools">
          <button class="chip" onclick={() => bannerInput?.click()} disabled={busy !== null}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 8h3l2-3h6l2 3h3v11H4zM12 17a4 4 0 100-8 4 4 0 000 8z" /></svg>
            {busy === "banner" ? "Saving…" : data.banner ? "Change cover" : "Add a cover picture"}
          </button>
          {#if data.banner}
            <button class="chip" onclick={() => remove("banner")} aria-label="Remove cover picture" title="Remove cover picture">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7L7 17" /></svg>
            </button>
          {/if}
        </div>
      </div>

      <div class="avatar-wrap">
        <button class="avatar" class:busy={busy === "photo"} onclick={() => photoInput?.click()} aria-label={p.photo_url ? "Change your photo" : "Add your photo"}>
          {#if p.photo_url}
            <img src={p.photo_url} alt="" />
          {:else}
            <span class="initials">{initials}</span>
          {/if}
          <span class="cam" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 8h3l2-3h6l2 3h3v11H4zM12 17a4 4 0 100-8 4 4 0 000 8z" /></svg>
          </span>
        </button>
        {#if p.photo_url}
          <button class="remove-photo" onclick={() => remove("photo")}>Remove photo</button>
        {/if}
      </div>

      <div class="identity">
        <h1>{p.name || "Your name"}</h1>
        <p class="meta">
          {#if p.pilot_id}<span class="num">{p.pilot_id}</span>{/if}
          {#if p.home_icao}<span>Home <b class="num">{p.home_icao}</b></span>{/if}
          {#if p.nationality}<span>{p.nationality}</span>{/if}
          {#if s.first_flight_ms}<span>Flying since {day(s.first_flight_ms).replace(/^\d+ /, "")}</span>{/if}
          {#if !p.name}<button class="link" onclick={onsettings}>Add your details in Settings</button>{/if}
        </p>
        <p class="rank-line"><span class="lvl num">{r!.rank.level}</span>{r!.rank.title}</p>
      </div>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
    </header>

    <section class="cards">
      <article class="card rating">
        <h2>Pilot rating</h2>
        <div class="big">
          <span class="num value">{int(r!.rating)}</span>
          {#if lastDelta != null}
            <span class="num change" class:up={lastDelta >= 0} class:down={lastDelta < 0}>
              {lastDelta >= 0 ? "▲" : "▼"}
              {Math.abs(lastDelta).toFixed(1)}
              <small>last flight</small>
            </span>
          {/if}
        </div>
        {#if spark}
          <svg class="spark" viewBox="0 0 {spark.W} {spark.H}" preserveAspectRatio="none" aria-hidden="true">
            <defs>
              <linearGradient id="spark-fill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#f0b23a" stop-opacity="0.32" />
                <stop offset="1" stop-color="#f0b23a" stop-opacity="0" />
              </linearGradient>
            </defs>
            <line x1="0" x2={spark.W} y1={spark.base} y2={spark.base} class="base" />
            <path d={spark.area} fill="url(#spark-fill)" />
            <path d={spark.line} class="line" />
            <circle cx={spark.end[0]} cy={spark.end[1]} r="3.5" class="dot" />
          </svg>
        {:else}
          <p class="hint">Everyone starts at 1,000. Each rated flight moves it up or down against what your rating expects.</p>
        {/if}
        <dl class="facts">
          <div><dt>Peak</dt><dd class="num">{int(r!.rating_peak)}</dd></div>
          <div><dt>Next flight expects</dt><dd class="num">{r!.expected}</dd></div>
          <div><dt>Rated flights</dt><dd class="num">{r!.rated_flights}</dd></div>
        </dl>
      </article>

      <article class="card rank">
        <h2>Rank</h2>
        <div class="board"><Epaulette insignia={r!.rank.insignia} width={250} /></div>
        <p class="title">{r!.rank.title}</p>
        <div class="xpbar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progress * 100)}>
          <i style="width:{progress * 100}%"></i>
        </div>
        <p class="xpline">
          <span class="num"><b>{int(r!.xp)}</b> XP</span>
          {#if r!.next_rank}
            <span><b class="num">{int(r!.next_rank.min_xp - r!.xp)}</b> to {r!.next_rank.title}</span>
          {:else}
            <span>Top rank</span>
          {/if}
        </p>
        <button class="link ranks-btn" onclick={() => (showRanks = !showRanks)} aria-expanded={showRanks}>
          {showRanks ? "Hide all ranks" : "All ranks"}
        </button>
      </article>

      <article class="card perf">
        <h2>Average scores</h2>
        <div class="rings">
          <ScoreRing value={r!.avg_performance} label="Overall" size={104} />
          <ScoreRing value={r!.avg_landing} label="Landing" size={84} />
          <ScoreRing value={r!.avg_safety} label="Safety" size={84} />
        </div>
        <p class="hint">
          Landing quality and flight safety count half each. Longer flights earn bonus XP but never change your rating.
        </p>
      </article>
    </section>

    {#if showRanks}
      <section class="ladder" aria-label="All ranks">
        {#each RANKS as k (k.level)}
          {@const reached = r!.xp >= k.min_xp}
          <div class="step" class:current={k.level === r!.rank.level} class:reached>
            <Epaulette insignia={k.insignia} width={150} dim={!reached} />
            <strong>{k.title}</strong>
            <small class="num">{k.min_xp ? `${int(k.min_xp)} XP` : "Start"}</small>
          </div>
        {/each}
      </section>
    {/if}

    <section class="block">
      <h3>Career</h3>
      <div class="tiles">
        <div class="tile"><span class="num">{int(t.flights)}</span><small>Flights</small></div>
        <div class="tile"><span class="num">{int(t.minutes / 60)}</span><small>Hours</small></div>
        <div class="tile"><span class="num">{distance(t.distance_nm).value}</span><small>{distance(t.distance_nm).unit} flown</small></div>
        <div class="tile"><span class="num">{int(t.landings)}</span><small>Landings</small></div>
        <div class="tile"><span class="num">{int(t.airports)}</span><small>Airports</small></div>
        <div class="tile"><span class="num">{int(t.countries)}</span><small>Countries</small></div>
        <div class="tile"><span class="num">{int(s.night_minutes / 60)}</span><small>Night hours</small></div>
        <div class="tile"><span class="num">{int(s.aircraft_types)}</span><small>Aircraft types</small></div>
      </div>

      <div class="two">
        <article class="card">
          <h2 class="split-h">
            Hours, last 12 months
            <span class="num">{hrs(s.months.reduce((a, m) => a + m.minutes, 0))} h</span>
          </h2>
          <div class="months">
            {#each s.months as m (m.month)}
              <div class="month" title="{monthName(m.month)}: {hoursMinutes(m.minutes)}, {m.flights} flights">
                <span class="col">
                  {#if m.minutes}<b class="num val">{hrs(m.minutes)}</b>{/if}
                  <i style="height:{(m.minutes / maxMonth) * 100}%" class:none={!m.minutes}></i>
                </span>
                <small>{monthName(m.month).slice(0, 3)}</small>
              </div>
            {/each}
          </div>
        </article>
        <article class="card">
          <h2>Records</h2>
          <ul class="records">
            <li>
              <span>Longest flight</span>
              {#if s.longest}<button class="rec" onclick={() => onflight(s.longest!.id)}><b class="num">{hoursMinutes(s.longest.value)}</b><small>{route(s.longest)}</small></button>{:else}<em>–</em>{/if}
            </li>
            <li>
              <span>Farthest flight</span>
              {#if s.farthest}<button class="rec" onclick={() => onflight(s.farthest!.id)}><b class="num">{distance(s.farthest.value).value} {distance(s.farthest.value).unit}</b><small>{route(s.farthest)}</small></button>{:else}<em>–</em>{/if}
            </li>
            <li>
              <span>Softest landing</span>
              {#if s.softest}<button class="rec" onclick={() => onflight(s.softest!.id)}><b class="num">{int(s.softest.value)} fpm</b><small>{route(s.softest)}</small></button>{:else}<em>–</em>{/if}
            </li>
            <li>
              <span>Average landing</span>
              <b class="num">{s.avg_landing_fpm == null ? "–" : `${int(s.avg_landing_fpm)} fpm`}</b>
            </li>
          </ul>
        </article>
      </div>

      {#if s.top_types.length || s.top_airports.length}
        <div class="two">
          <article class="card">
            <h2>Most flown aircraft</h2>
            <ul class="bars">
              {#each s.top_types as ty (ty.name)}
                <li>
                  <span class="num name">{ty.name}</span>
                  <span class="track"><i style="width:{(ty.minutes / maxType) * 100}%"></i></span>
                  <span class="num val">{int(ty.minutes / 60)} h</span>
                </li>
              {/each}
            </ul>
          </article>
          <article class="card">
            <h2>Favorite airports</h2>
            <ul class="bars">
              {#each s.top_airports as ap (ap.name)}
                <li>
                  <span class="num name">{ap.name}</span>
                  <span class="track"><i class="mag" style="width:{(ap.count / maxPort) * 100}%"></i></span>
                  <span class="num val">{ap.count}×</span>
                </li>
              {/each}
            </ul>
          </article>
        </div>
      {/if}
    </section>

    <section class="block">
      <h3>Flight scores</h3>
      <FlightScores flights={r!.flights} {onflight} />
    </section>
  {/if}
</div>

<style>
  .page {
    height: 100%;
    overflow-y: auto;
    padding: 0 0 48px;
  }
  .loading {
    padding: 40px;
    color: var(--ink-dim);
  }
  .cards,
  .block,
  .ladder {
    max-width: 1120px;
    margin: 0 auto;
    padding: 0 28px;
  }
  .hero {
    position: relative;
  }
  /* Edge to edge, whatever the window's width. */
  .banner {
    position: relative;
    width: 100%;
    height: clamp(170px, 19vw, 340px);
    overflow: hidden;
    background: #14171c;
    border-bottom: 1px solid var(--rule);
  }
  .banner img,
  .banner .dusk {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0) 55%, rgba(0, 0, 0, 0.45));
    pointer-events: none;
  }
  .busy {
    opacity: 0.7;
  }
  .banner-tools {
    position: absolute;
    right: 14px;
    top: 14px;
    display: flex;
    gap: 6px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .banner:hover .banner-tools,
  .banner-tools:focus-within,
  .banner:not(:has(img)) .banner-tools {
    opacity: 1;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    background: rgba(16, 18, 22, 0.72);
    backdrop-filter: blur(6px);
    color: #fff;
    font-size: 13px;
  }
  .chip:hover:not(:disabled) {
    border-color: var(--amber);
    color: var(--amber);
  }
  .chip svg,
  .cam svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .avatar-wrap {
    position: relative;
    margin: -78px auto 0;
    width: 156px;
    display: grid;
    justify-items: center;
    gap: 6px;
    z-index: 2;
  }
  .avatar {
    position: relative;
    width: 156px;
    height: 156px;
    padding: 0;
    border-radius: 50%;
    border: 4px solid var(--panel);
    background: linear-gradient(145deg, #2c3034, #1e2124);
    box-shadow:
      0 0 0 2px var(--amber),
      0 10px 30px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .initials {
    font-family: var(--cond);
    font-size: 54px;
    font-weight: 600;
    color: var(--amber);
    letter-spacing: 0.04em;
  }
  .cam {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.5);
    color: #fff;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .cam svg {
    width: 30px;
    height: 30px;
  }
  .avatar:hover .cam,
  .avatar:focus-visible .cam {
    opacity: 1;
  }
  .remove-photo {
    border: none;
    background: transparent;
    color: var(--ink-faint);
    font-size: 12px;
    padding: 0;
  }
  .remove-photo:hover {
    color: var(--ink);
  }
  .identity {
    text-align: center;
    margin-top: 8px;
  }
  h1 {
    margin: 0;
    font-family: var(--cond);
    font-size: 36px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .meta {
    margin: 4px 0 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 6px 18px;
    color: var(--ink-dim);
    font-size: 14px;
  }
  .meta b {
    color: var(--ink);
    font-weight: 500;
  }
  .rank-line {
    margin: 12px 0 0;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 4px 14px 4px 4px;
    border-radius: 999px;
    background: var(--amber-glow);
    border: 1px solid rgba(240, 178, 58, 0.35);
    color: var(--amber);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 13px;
  }
  .lvl {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--amber);
    color: #1c1e21;
    font-size: 14px;
  }
  .error {
    color: var(--bad);
    text-align: center;
  }
  .cards {
    display: grid;
    grid-template-columns: 1.15fr 1fr 1fr;
    gap: 16px;
    margin-top: 28px;
  }
  .card {
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 14px;
    padding: 18px 20px;
    min-width: 0;
  }
  h2 {
    margin: 0 0 10px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .big {
    display: flex;
    align-items: baseline;
    gap: 14px;
  }
  .big .value {
    font-size: 64px;
    line-height: 1;
    font-weight: 600;
    color: var(--amber);
    text-shadow: 0 0 24px rgba(240, 178, 58, 0.25);
  }
  .change {
    font-size: 17px;
    display: grid;
  }
  .change small {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--ink-faint);
  }
  .up {
    color: var(--good);
  }
  .down {
    color: var(--bad);
  }
  .spark {
    width: 100%;
    height: 84px;
    margin: 12px 0 6px;
    display: block;
    overflow: visible;
  }
  .spark .line {
    fill: none;
    stroke: var(--amber);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
    stroke-linejoin: round;
  }
  .spark .base {
    stroke: var(--rule);
    stroke-dasharray: 3 4;
    vector-effect: non-scaling-stroke;
  }
  .spark .dot {
    fill: var(--amber);
    stroke: var(--panel-2);
    stroke-width: 2;
  }
  .hint {
    color: var(--ink-dim);
    font-size: 13px;
    margin: 12px 0 0;
  }
  .facts {
    display: flex;
    gap: 22px;
    margin: 8px 0 0;
  }
  .facts div {
    display: grid;
  }
  .facts dt {
    font-size: 11px;
    color: var(--ink-faint);
  }
  .facts dd {
    margin: 0;
    font-size: 18px;
  }
  .rank {
    display: grid;
    justify-items: center;
    align-content: start;
    text-align: center;
  }
  .rank h2 {
    justify-self: start;
  }
  .board {
    padding: 10px 0 4px;
  }
  .rank .title {
    margin: 8px 0 12px;
    font-family: var(--cond);
    font-size: 24px;
    font-weight: 600;
  }
  .xpbar {
    width: 100%;
    height: 8px;
    border-radius: 4px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .xpbar i {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: linear-gradient(90deg, #b8862b, var(--amber), #fbe39a);
    box-shadow: 0 0 10px rgba(240, 178, 58, 0.5);
  }
  .xpline {
    width: 100%;
    display: flex;
    justify-content: space-between;
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--ink-dim);
  }
  .xpline b {
    color: var(--ink);
    font-weight: 600;
  }
  .ranks-btn {
    margin-top: 12px;
  }
  .link {
    border: none;
    background: transparent;
    color: var(--amber);
    padding: 0;
    font-size: 13px;
  }
  .link:hover {
    text-decoration: underline;
  }
  .rings {
    display: flex;
    justify-content: space-around;
    align-items: flex-end;
    padding: 6px 0;
  }
  .ladder {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 10px;
    margin-top: 16px;
  }
  .step {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 14px 8px;
    border-radius: 12px;
    border: 1px solid var(--rule);
    background: var(--panel-2);
    text-align: center;
  }
  .step strong {
    font-size: 13px;
    margin-top: 6px;
  }
  .step:not(.reached) strong {
    color: var(--ink-dim);
  }
  .step small {
    color: var(--ink-faint);
    font-size: 12px;
  }
  .step.current {
    border-color: var(--amber);
    background: var(--amber-glow);
  }
  .block {
    margin-top: 34px;
  }
  h3 {
    margin: 0 0 12px;
    font-family: var(--cond);
    font-size: 22px;
    font-weight: 600;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 10px;
  }
  .tile {
    display: grid;
    padding: 14px 14px 12px;
    border-radius: 12px;
    background: var(--panel-2);
    border: 1px solid var(--rule);
  }
  .tile span {
    font-size: 28px;
    font-weight: 600;
    line-height: 1.1;
  }
  .tile small {
    color: var(--ink-dim);
    font-size: 12px;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-top: 16px;
  }
  .months {
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    gap: 6px;
    height: 200px;
    align-items: end;
  }
  .month {
    display: grid;
    grid-template-rows: 1fr auto;
    height: 100%;
    gap: 6px;
    justify-items: center;
  }
  .col {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: stretch;
    gap: 3px;
  }
  .val {
    text-align: center;
    font-size: 12px;
    font-weight: 500;
    color: var(--ink);
    line-height: 1;
  }
  .split-h {
    display: flex;
    justify-content: space-between;
  }
  .split-h span {
    color: var(--amber);
    font-size: 14px;
    letter-spacing: 0;
    text-transform: none;
  }
  .col i {
    width: 100%;
    border-radius: 4px 4px 2px 2px;
    background: linear-gradient(to top, #b8862b, var(--amber));
    min-height: 3px;
  }
  .col i.none {
    background: var(--panel-3);
  }
  .month small {
    color: var(--ink-faint);
    font-size: 11px;
  }
  .records {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
  }
  .records li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 9px 0;
    border-bottom: 1px solid var(--rule);
    font-size: 14px;
    color: var(--ink-dim);
  }
  .records li:last-child {
    border-bottom: none;
  }
  .records b {
    color: var(--ink);
    font-size: 17px;
  }
  .rec {
    border: none;
    background: transparent;
    padding: 0;
    display: grid;
    justify-items: end;
    text-align: right;
  }
  .rec small {
    color: var(--ink-faint);
    font-size: 12px;
  }
  .rec:hover b {
    color: var(--amber);
  }
  .bars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .bars li {
    display: grid;
    grid-template-columns: 52px 1fr 56px;
    align-items: center;
    gap: 12px;
  }
  .bars .name {
    font-size: 16px;
  }
  .bars .track {
    height: 8px;
    border-radius: 4px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .bars .track i {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--amber);
  }
  .bars .track i.mag {
    background: var(--magenta);
  }
  .bars .val {
    text-align: right;
    color: var(--ink-dim);
  }
  @media (max-width: 1100px) {
    .cards {
      grid-template-columns: 1fr 1fr;
    }
    .cards .perf {
      grid-column: 1 / -1;
    }
    .tiles {
      grid-template-columns: repeat(4, 1fr);
    }
  }
</style>
