<script lang="ts">
  import Logo from "./Logo.svelte";
  // First visit from another device: enter the code shown in Settings on
  // the flight sim PC.
  import { pair, session } from "./remote.svelte";

  let code = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function go(e: Event) {
    e.preventDefault();
    busy = true;
    error = null;
    try {
      error = await pair(code.replace(/\s/g, ""));
    } catch {
      error = "Could not reach FlightDeck. Is the flight sim PC on, with FlightDeck running?";
    } finally {
      busy = false;
    }
  }
</script>

<div class="pair">
  <form onsubmit={go}>
    <Logo size={72} />
    <h1>FlightDeck</h1>
    {#if session.offline}
      <p>Can't reach FlightDeck on the flight sim PC. Check that the PC is on, FlightDeck is running, and this device is on the same Wi-Fi.</p>
    {:else}
      <p>
        To use FlightDeck on this device, enter the pairing code shown on the flight sim PC in
        <strong>Settings, Other devices</strong>.
      </p>
      <input
        bind:value={code}
        inputmode="numeric"
        autocomplete="one-time-code"
        maxlength="7"
        placeholder="000000"
        aria-label="Pairing code"
      />
      {#if error}<p class="err" role="alert">{error}</p>{/if}
      <button type="submit" disabled={busy || code.replace(/\s/g, "").length < 6}>{busy ? "Pairing…" : "Pair this device"}</button>
      <p class="small">FlightDeck only answers devices on your home network. Your data stays on the flight sim PC.</p>
    {/if}
  </form>
</div>

<style>
  .pair {
    min-height: 100%;
    display: grid;
    place-items: center;
    padding: 24px 16px;
    background: radial-gradient(ellipse at 50% 30%, rgba(240, 178, 58, 0.06), transparent 60%), #17181a;
  }
  form {
    width: min(380px, 100%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    text-align: center;
  }
  h1 {
    margin: 0;
    font-family: var(--cond);
    font-size: 30px;
  }
  p {
    margin: 0;
    color: var(--ink-dim);
    line-height: 1.5;
  }
  strong {
    color: var(--ink);
  }
  input {
    width: 100%;
    font: 600 34px/1 var(--cond);
    letter-spacing: 0.3em;
    text-align: center;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 10px;
    padding: 14px 10px;
  }
  input:focus {
    outline: none;
    border-color: var(--amber);
  }
  button {
    width: 100%;
    border: none;
    border-radius: 10px;
    padding: 14px;
    background: var(--amber);
    color: #1b1406;
    font-family: var(--cond);
    font-size: 19px;
    font-weight: 600;
  }
  button:disabled {
    opacity: 0.5;
  }
  .err {
    color: var(--bad);
  }
  .small {
    font-size: 12.5px;
  }
</style>
