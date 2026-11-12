// Reading an MSFS flight plan (.PLN): from MSFS 2020 and 2024's own
// planner, SimBrief, Little Navmap, FlightDeck and others. Some give every
// position (WorldPosition, DepartureLLA); SimBrief's and MSFS 2024's give
// only idents with their ICAO region, and MSFS 2024's may name just the
// departure and arrival procedures. Positions missing here are looked up
// afterwards (`needsPositions`).

import type { PlanFix } from "../api";

export interface PlnPoint {
  ident: string;
  region: string;
  /** "apt", "vor", "ndb", "wpt" or "ltlg" (a user point). */
  kind: string;
  /** NaN when the file gives no position. */
  lat: number;
  lon: number;
  /** Feet; 0 when not given. */
  alt: number;
  procedure: boolean;
}

export interface PlnPlan {
  title: string;
  rules: "VFR" | "IFR";
  cruiseFt: number;
  dep: { icao: string; lat: number; lon: number };
  dest: { icao: string; lat: number; lon: number };
  points: PlnPoint[];
  /** Runway to land on ("26L"), from the arrival or approach. */
  arrRunway: string;
  /** Named procedures, e.g. "ECHHO2.SLI", "RNDRZ4.MISEN", "ILS Y 26L". */
  departure: string;
  arrival: string;
  approach: string;
}

/** "N47° 26' 56.00",W122° 18' 32.00",+000433.00" → [lat, lon, alt ft]. */
export function parseLla(s: string): [number, number, number] | null {
  const m = s.match(/([NS])\s*(\d+)\D+(\d+)\D+([\d.]+)[^,]*,\s*([EW])\s*(\d+)\D+(\d+)\D+([\d.]+)[^,]*(?:,\s*([+-]?[\d.]+))?/i);
  if (!m) return null;
  const lat = (Number(m[2]) + Number(m[3]) / 60 + Number(m[4]) / 3600) * (m[1].toUpperCase() === "S" ? -1 : 1);
  const lon = (Number(m[6]) + Number(m[7]) / 60 + Number(m[8]) / 3600) * (m[5].toUpperCase() === "W" ? -1 : 1);
  const alt = m[9] ? Number(m[9]) : 0;
  return Number.isFinite(lat) && Number.isFinite(lon) ? [lat, lon, Number.isFinite(alt) ? alt : 0] : null;
}

const KIND: Record<string, string> = { airport: "apt", user: "ltlg", intersection: "wpt", vor: "vor", ndb: "ndb" };
const SIDE: Record<string, string> = { LEFT: "L", RIGHT: "R", CENTER: "C", CENTRE: "C", L: "L", R: "R", C: "C" };

/** The plan in a .PLN file's text; throws with a plain message if it is not one. */
export function parsePln(text: string): PlnPlan {
  // Some files carry a byte-order mark or stray text before the XML.
  const start = text.indexOf("<");
  const doc = new DOMParser().parseFromString(start > 0 ? text.slice(start) : text, "application/xml");
  if (doc.getElementsByTagName("parsererror").length) throw new Error("This file is not a flight plan FlightDeck can read.");
  const fp = doc.getElementsByTagName("FlightPlan.FlightPlan")[0];
  if (!fp) throw new Error("No flight plan found in this file (an MSFS .PLN is expected).");
  /** A direct child's text (not one inside a waypoint). */
  const own = (el: Element, name: string) => [...el.children].find((c) => c.tagName === name)?.textContent?.trim() ?? "";
  const tag = (el: Element, name: string) => el.getElementsByTagName(name)[0]?.textContent?.trim() ?? "";
  const depId = own(fp, "DepartureID").toUpperCase();
  const destId = own(fp, "DestinationID").toUpperCase();
  if (!depId || !destId) throw new Error("The flight plan has no departure or destination airport.");
  const depLla = parseLla(own(fp, "DepartureLLA"));
  const destLla = parseLla(own(fp, "DestinationLLA"));
  const cruiseFt = Math.round(Number(own(fp, "CruisingAlt")) || 0);
  const rules = own(fp, "FPType").toUpperCase() === "IFR" ? "IFR" : "VFR";
  const points: PlnPoint[] = [...fp.getElementsByTagName("ATCWaypoint")].map((w) => {
    const pos = parseLla(tag(w, "WorldPosition"));
    const type = tag(w, "ATCWaypointType").toLowerCase();
    const ident = (tag(w, "ICAOIdent") || w.getAttribute("id") || "").trim().toUpperCase();
    const procedure = ["DepartureFP", "ArrivalFP", "ApproachTypeFP", "SIDFP", "STARFP"].some((t) => w.getElementsByTagName(t).length > 0);
    return {
      ident,
      region: tag(w, "ICAORegion").toUpperCase(),
      kind: KIND[type] ?? (ident.length === 5 ? "wpt" : "vor"),
      lat: pos ? pos[0] : NaN,
      lon: pos ? pos[1] : NaN,
      alt: pos ? Math.round(pos[2]) : 0,
      procedure,
    };
  });
  // The airports themselves often come first and last: kept as origin and destination.
  let mid = points;
  if (mid.length && mid[0].kind === "apt" && mid[0].ident === depId) mid = mid.slice(1);
  if (mid.length && mid[mid.length - 1].kind === "apt" && mid[mid.length - 1].ident === destId) mid = mid.slice(0, -1);
  const first = points[0];
  const last = points[points.length - 1];
  const dep = depLla ? { icao: depId, lat: depLla[0], lon: depLla[1] } : first?.ident === depId ? { icao: depId, lat: first.lat, lon: first.lon } : { icao: depId, lat: NaN, lon: NaN };
  const dest = destLla ? { icao: destId, lat: destLla[0], lon: destLla[1] } : last?.ident === destId ? { icao: destId, lat: last.lat, lon: last.lon } : { icao: destId, lat: NaN, lon: NaN };
  // Procedures and the landing runway (MSFS 2024 and SimBrief).
  const block = (name: string) => [...fp.children].find((c) => c.tagName === name);
  const rwy = (el: Element | undefined) => {
    if (!el) return "";
    const n = tag(el, "RunwayNumberFP");
    return n ? `${n.padStart(2, "0")}${SIDE[tag(el, "RunwayDesignatorFP").toUpperCase()] ?? ""}` : "";
  };
  const depB = block("DepartureDetails");
  const arrB = block("ArrivalDetails");
  const appB = block("ApproachDetails");
  const named = (el: Element | undefined, key: string) => (el && tag(el, key) ? [tag(el, key), tag(el, "TransitionFP")].filter(Boolean).join(".") : "");
  const approach = appB && tag(appB, "ApproachTypeFP") ? [tag(appB, "ApproachTypeFP"), tag(appB, "SuffixFP"), rwy(appB)].filter(Boolean).join(" ") : "";
  return {
    title: own(fp, "Title"),
    rules,
    cruiseFt,
    dep,
    dest,
    points: mid,
    arrRunway: rwy(arrB) || rwy(appB),
    departure: named(depB, "DepartureFP"),
    arrival: named(arrB, "ArrivalFP"),
    approach,
  };
}

/** The idents whose positions the file leaves out. */
export function needsPositions(p: PlnPlan): string[] {
  return [...new Set(p.points.filter((w) => !Number.isFinite(w.lat) && w.kind !== "apt").map((w) => w.ident).filter(Boolean))];
}

/** The plan's points as the route's fixes (those with a position), with
 * their altitudes kept where they differ from the cruise. */
export function toFixes(points: PlnPoint[], cruiseFt: number): PlanFix[] {
  return points
    .filter((w) => Number.isFinite(w.lat) && Number.isFinite(w.lon))
    .map((w) => {
      const own = w.alt > 0 && Math.abs(w.alt - cruiseFt) > 50;
      return { ident: w.ident || "", kind: w.kind, lat: w.lat, lon: w.lon, alt_ft: own ? w.alt : cruiseFt, stage: own ? "USR" : "CRZ", procedure: w.procedure };
    });
}
