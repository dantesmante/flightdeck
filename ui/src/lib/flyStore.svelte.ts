// "Fly this aircraft" from a parked aircraft on the map: the Start a flight
// card opens with that aircraft, and the flight is logged to it.
import type { HangarItem } from "./api";
import type { Spot } from "./plan/AirportField.svelte";

export const fly = $state<{ item: HangarItem | null }>({ item: null });

/**
 * The origin and destination being typed into Start a flight, before the
 * flight is started: the map previews that route.
 */
export const draft = $state<{ origin: Spot | null; dest: Spot | null }>({ origin: null, dest: null });
