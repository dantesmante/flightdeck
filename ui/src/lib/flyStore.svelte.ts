// "Fly this aircraft" from a parked aircraft on the map: the Start a flight
// card opens with that aircraft, and the flight is logged to it.
import type { HangarItem, PlanFix } from "./api";
import type { Spot } from "./plan/AirportField.svelte";

export const fly = $state<{ item: HangarItem | null }>({ item: null });

/**
 * The origin and destination being typed into Start a flight, before the
 * flight is started: the map previews that route.
 */
export const draft = $state<{ origin: Spot | null; dest: Spot | null; fixes: PlanFix[]; cruise_ft: number; runway: string; locked: boolean }>({ origin: null, dest: null, fixes: [], cruise_ft: 0, runway: "", locked: false });

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

/** "Clear" in the Start a flight card's header: the form starts over. */
export const planClear = $state({ n: 0 });
/** Cancel at the foot of Start a flight: the card closes. */
export const planClose = $state({ n: 0 });
/** One waypoint's altitude set from the map or the profile: the index into
 * the plan's fixes, feet (null: follow the cruise); `n` counts requests. */
export const planAlt = $state<{ i: number; ft: number | null; n: number }>({ i: -1, ft: null, n: 0 });
/** The planned flight's airline livery for the Start a flight header:
 * background, accent, swoosh colour and the text colour that reads on it. */
export const planLook = $state<{ on: boolean; a: string; b: string; c: string; ink: string }>({ on: false, a: "", b: "", c: "", ink: "#fff" });

/** The plan's time and fuel for the card at the bottom of the map. */
export interface PlanInfo {
  distance: string;
  ete: string;
  fuel: string;
  fuelSub: string;
  wind: string;
  /** The breakdown, opened from the card. */
  rows: [string, string, string?][];
  source: string;
  warn: string | null;
}
export const planInfo = $state<{ v: PlanInfo | null }>({ v: null });

/**
 * The planned route changed on the map (a point dragged, put in or taken
 * out of the dashed line): the new points between origin and destination,
 * for Start a flight to take.
 */
export const planEdit = $state<{ fixes: import("./api").PlanFix[] | null }>({ fixes: null });
