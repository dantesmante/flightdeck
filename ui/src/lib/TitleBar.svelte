<script lang="ts">
  // FlightDeck's own title bar (the window has no system frame): the name
  // and simulator link on the left, the time in UTC (Zulu) with local time
  // under it, and the window buttons on the right. Drag the bar to move the
  // window; double-click it to maximize.
  import { onMount } from "svelte";
  import { inTauri } from "./api";
  import { inRemote, session } from "./remote.svelte";

  let { linkText, link }: { linkText: string; link: string } = $props();

  let now = $state(new Date());
  let maximized = $state(false);

  const two = (n: number) => String(n).padStart(2, "0");
  const zulu = $derived(`${two(now.getUTCHours())}${two(now.getUTCMinutes())}Z`);
  const local = $derived(`${two(now.getHours())}${two(now.getMinutes())}`);
  const zone = $derived(
    new Intl.DateTimeFormat("en-US", { timeZoneName: "short" }).formatToParts(now).find((p) => p.type === "timeZoneName")?.value ?? "local",
  );

  async function win() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow();
  }

  onMount(() => {
    // Tick on the minute.
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      now = new Date();
      timer = setTimeout(tick, 60_000 - (Date.now() % 60_000) + 50);
    };
    tick();

    let unlisten: (() => void) | undefined;
    if (inTauri) {
      win().then(async (w) => {
        maximized = await w.isMaximized();
        unlisten = await w.onResized(async () => (maximized = await w.isMaximized()));
      });
    }
    return () => {
      clearTimeout(timer);
      unlisten?.();
    };
  });

  const minimize = async () => (await win()).minimize();
  const toggle = async () => (await win()).toggleMaximize();
  const close = async () => (await win()).close();
</script>

<header class="bar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg viewBox="0 0 1024 1024" width="26" height="26" aria-hidden="true">
      <circle cx="512" cy="512" r="330" fill="none" stroke="#3a4148" stroke-width="64" />
      <path d="M190 700 C 360 520, 560 640, 820 330" fill="none" stroke="#E2468F" stroke-width="72" stroke-linecap="round" />
    </svg>
    <div class="name" data-tauri-drag-region>
      <strong data-tauri-drag-region>FlightDeck</strong>
      <span class="link {link}" data-tauri-drag-region><i></i>{linkText}</span>
    </div>
  </div>

  {#if inRemote}
    <span class="mode" class:edit={session.canEdit} title={session.canEdit ? "This device can make changes" : "Changes can be allowed in Settings on the flight sim PC"}>
      {session.offline ? "Reconnecting…" : session.canEdit ? "Remote, can edit" : "Remote, view only"}
    </span>
  {/if}
  <span class="grow" data-tauri-drag-region></span>

  <div class="clock" title="Coordinated Universal Time (Zulu), and your local time ({zone})" data-tauri-drag-region>
    <span class="z num" data-tauri-drag-region>{zulu}</span>
    <span class="l num" data-tauri-drag-region>{local} {zone}</span>
  </div>

  {#if inTauri}
    <div class="controls">
      <button aria-label="Minimize" title="Minimize" onclick={minimize}>
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 6h8" /></svg>
      </button>
      <button aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onclick={toggle}>
        {#if maximized}
          <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3.5 3.5V2h6.5v6.5H8.5M2 3.5h6.5V10H2z" /></svg>
        {:else}
          <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2h8v8H2z" /></svg>
        {/if}
      </button>
      <button class="close" aria-label="Close" title="Close" onclick={close}>
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" /></svg>
      </button>
    </div>
  {/if}
</header>

<style>
  .bar {
    height: 46px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding-left: 16px;
    background: #151618;
    border-bottom: 1px solid var(--rule);
    user-select: none;
    -webkit-user-select: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    display: flex;
    flex-direction: column;
    line-height: 1.1;
  }
  strong {
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--ink);
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--ink-dim);
  }
  .link i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #6b7078;
  }
  .link.connecting i {
    background: var(--amber);
  }
  .link.connected i {
    background: var(--good);
    box-shadow: 0 0 6px var(--good);
  }
  .mode {
    padding: 2px 9px;
    border-radius: 999px;
    border: 1px solid var(--rule);
    color: var(--ink-dim);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .mode.edit {
    color: var(--amber);
    border-color: rgba(240, 178, 58, 0.4);
  }
  .grow {
    flex: 1;
    align-self: stretch;
  }
  .clock {
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1.05;
    padding: 0 6px;
  }
  .z {
    font-size: 19px;
    font-weight: 700;
    color: var(--ink);
    letter-spacing: 0.02em;
  }
  .l {
    font-size: 12.5px;
    color: var(--ink-dim);
  }
  .controls {
    display: flex;
    align-self: stretch;
  }
  .controls button {
    width: 46px;
    border: none;
    background: transparent;
    color: var(--ink-dim);
    display: grid;
    place-items: center;
    transition: background 0.12s, color 0.12s;
  }
  .controls button:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--ink);
  }
  .controls .close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .controls svg {
    width: 11px;
    height: 11px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.1;
  }
  /* Text inside the bar lets the drag through to the bar. */
  .name *,
  .clock * {
    pointer-events: none;
  }
</style>
