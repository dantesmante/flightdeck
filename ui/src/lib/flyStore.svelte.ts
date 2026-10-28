// "Fly this aircraft" from a parked aircraft on the map: the Start a flight
// card opens with that aircraft, and the flight is logged to it.
import type { HangarItem } from "./api";

export const fly = $state<{ item: HangarItem | null }>({ item: null });
