<script lang="ts">
  // In a browser on another device, pair first; otherwise straight in.
  import App from "./App.svelte";
  import Pair from "./lib/Pair.svelte";
  import { checkSession, inRemote, session } from "./lib/remote.svelte";

  if (inRemote) checkSession();
</script>

{#if !inRemote || session.paired}
  <App />
{:else if session.checked}
  <Pair />
{/if}
