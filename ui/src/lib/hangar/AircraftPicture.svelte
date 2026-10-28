<script lang="ts">
  // The aircraft's picture: the pilot's own photo, else the latest
  // Planespotters.net photo of its registration (credited and linked, as
  // their terms require), else a silhouette. On cards, hovering shows
  // Upload image.
  import type { HangarItem } from "../api";
  import { openExternal } from "../external";
  import Silhouette from "./Silhouette.svelte";
  import { loadSpotter, spotters } from "./spotters.svelte";

  interface Props {
    item: HangarItem;
    own: string | null;
    /** Card: hover to upload. Hero: picture only (the window has its own button). */
    variant?: "card" | "hero";
    /** Show the photographer credit over the picture (the hero shows it below). */
    credit?: boolean;
    onpick?: (e: Event) => void;
    onremove?: () => void;
  }
  let { item, own, variant = "card", credit = true, onpick, onremove }: Props = $props();

  $effect(() => {
    if (!own && item.registration) loadSpotter(item.registration);
  });
  const spot = $derived(!own && item.registration ? spotters[item.registration.trim().toUpperCase()] : null);

  function open(e: MouseEvent, url: string) {
    e.preventDefault();
    e.stopPropagation();
    openExternal(url);
  }
</script>

<div class="pic {variant}">
  <div class="art">
    {#if own}
      <img src={own} alt={item.registration || item.title} />
    {:else if spot}
      <a href={spot.link} onclick={(e) => open(e, spot.link)} title="See this photo on Planespotters.net">
        <img src={spot.src} alt="{item.registration} by {spot.photographer}" referrerpolicy="origin" />
      </a>
    {:else}
      <div class="sil"><Silhouette icao_type={item.icao_type} engine_type={item.engine_type} engine_count={item.engine_count} title={item.title} /></div>
    {/if}
  </div>

  {#if spot && credit}
    <a class="credit" href={spot.link} onclick={(e) => open(e, spot.link)}>© {spot.photographer || "Photographer"} · Planespotters.net</a>
  {/if}

  {#if variant === "card" && onpick}
    <label class="upload" class:small={!!spot}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 18h10a4 4 0 0 0 .6-7.96A6 6 0 0 0 6.1 9.1 4.5 4.5 0 0 0 7 18zM12 16v-6M9.5 12.5 12 10l2.5 2.5" /></svg>
      <span>{own ? "Change image" : "Upload image"}</span>
      <input type="file" accept="image/*" onchange={onpick} />
    </label>
    {#if own && onremove}
      <button class="remove" onclick={onremove}>Remove my photo</button>
    {/if}
  {/if}
</div>

<style>
  .pic {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: linear-gradient(160deg, #2c2f34, #222428);
  }
  .art {
    position: absolute;
    inset: 0;
    transition: filter 0.25s, transform 0.25s;
  }
  .art a {
    display: block;
    width: 100%;
    height: 100%;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .sil {
    position: absolute;
    inset: 12% 8%;
  }
  /* In the aircraft window the title sits over the lower part. */
  .hero .sil {
    inset: 4% 24% 36%;
  }
  .credit {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 2;
    padding: 14px 10px 5px;
    font-size: 10.5px;
    color: rgba(255, 255, 255, 0.9);
    text-decoration: none;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.6));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .credit:hover {
    text-decoration: underline;
  }
  .card:hover .art {
    filter: blur(6px) brightness(0.65);
    transform: scale(1.04);
  }
  .upload {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: #fff;
    font-weight: 600;
    font-size: 15px;
    opacity: 0;
    cursor: pointer;
    transition: opacity 0.2s;
  }
  /* Over a Planespotters photo only the button uploads; the rest of the
     picture still links to the photo's page. */
  .upload.small {
    inset: auto;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 3;
  }
  .upload svg {
    width: 34px;
    height: 34px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .upload span {
    padding: 5px 12px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.6);
    background: rgba(20, 22, 25, 0.45);
  }
  .upload input {
    display: none;
  }
  .card:hover .upload,
  .upload:focus-within {
    opacity: 1;
  }
  .remove {
    position: absolute;
    left: 50%;
    bottom: 10px;
    transform: translateX(-50%);
    z-index: 3;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font-size: 12px;
    text-decoration: underline;
    opacity: 0;
    transition: opacity 0.2s;
  }
  .card:hover .remove {
    opacity: 1;
  }
</style>
