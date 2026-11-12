// "Fly this aircraft" from a parked aircraft on the map: the Start a flight
// card opens with that aircraft, and the flight is logged to it.
import type { HangarItem, PlanFix } from "./api";
import type { Spot } from "./plan/AirportField.svelte";

export const fly = $state<{ item: HangarItem | null }>({ item: null });

/**
 * The origin and destination being typed into Start a flight, before the
 * flight is started: the map previews that route.
 */
export const draft = $state<{ origin: Spot | null; dest: Spot | null; fixes: PlanFix[]; cruise_ft: number; runway: string }>({ origin: null, dest: null, fixes: [], cruise_ft: 0, runway: "" });

/**
 * An airport sent to the flight planner from the map (right-click, "Set
 * as origin" or "Set as destination"). The Start a flight card opens and
 * takes it; `locked` is set while a flight is under way and nothing can be
 * changed.
 */
export const pick = $state<{ which: "origin" | "dest" | null; spot: Spot | null; locked: boolean }>({ which: null, spot: null, locked: false });

/** A point of a route drawn on the map: an airport passed over, or a
 * point clicked anywhere (a user waypoint). */
export interface RoutePt {
  lat: number;
  lon: number;
  /** The airport's code; empty for a user waypoint. */
  icao: string;
  name: string;
  /** An altitude set for this point, feet (else the cruise). */
  alt?: number | null;
}

/**
 * Draw route, from Start a flight: first the origin (an airport), then
 * points clicked on the map, ending on the destination airport. The map
 * takes the clicks while `stage` is "origin" or "route".
 */
export const drawing = $state<{ stage: "off" | "origin" | "route"; origin: Spot | null; pts: RoutePt[]; note: string }>({
  stage: "off",
  origin: null,
  pts: [],
  note: "",
});

/**
 * The planned route changed on the map (a point dragged, put in or taken
 * out of the dashed line): the new points between origin and destination,
 * for Start a flight to take.
 */
export const planEdit = $state<{ fixes: import("./api").PlanFix[] | null }>({ fixes: null });
