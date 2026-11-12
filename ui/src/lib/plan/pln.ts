// Reading an MSFS flight plan (.PLN, from MSFS 2020 or 2024, Little Navmap,
// SimBrief and others): departure and destination, every waypoint with its
// position and planned altitude, the cruise altitude and the flight rules.

import type { PlanFix } from "../api";

export interface PlnPlan {
  title: string;
  rules: "VFR" | "IFR";
  cruiseFt: number;
  dep: { icao: string; lat: number; lon: number };
  dest: { icao: string; lat: number; lon: number };
  fixes: PlanFix[];
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

/** The plan in a .PLN file's text; throws with a plain message if it is not one. */
export function parsePln(text: string): PlnPlan {
  const doc = new DOMParser().parseFromString(text, "application/xml");
  if (doc.getElementsByTagName("parsererror").length) throw new Error("This file is not a flight plan FlightDeck can read.");
  const fp = doc.getElementsByTagName("FlightPlan.FlightPlan")[0];
  if (!fp) throw new Error("No flight plan found in this file (an MSFS .PLN is expected).");
  const tag = (el: Element, name: string) => el.getElementsByTagName(name)[0]?.textContent?.trim() ?? "";
  const depId = tag(fp, "DepartureID").toUpperCase();
  const destId = tag(fp, "DestinationID").toUpperCase();
  const depLla = parseLla(tag(fp, "DepartureLLA"));
  const destLla = parseLla(tag(fp, "DestinationLLA"));
  const cruiseFt = Math.round(Number(tag(fp, "CruisingAlt")) || 0);
  const rules = tag(fp, "FPType").toUpperCase() === "IFR" ? "IFR" : "VFR";
  const wps = [...fp.getElementsByTagName("ATCWaypoint")].map((w) => {
    const pos = parseLla(tag(w, "WorldPosition"));
    const type = tag(w, "ATCWaypointType").toLowerCase();
    const ident = (tag(w, "ICAOIdent") || w.getAttribute("id") || "").toUpperCase();
    const procedure = ["DepartureFP", "ArrivalFP", "ApproachTypeFP", "SIDFP", "STARFP"].some((t) => w.getElementsByTagName(t).length > 0);
    return pos ? { ident, type, lat: pos[0], lon: pos[1], alt: Math.round(pos[2]), procedure } : null;
  });
  const list = wps.filter((w): w is NonNullable<typeof w> => !!w);
  // The airports themselves come first and last: kept as origin and destination.
  const first = list[0];
  const last = list[list.length - 1];
  const dep = depLla ? { icao: depId, lat: depLla[0], lon: depLla[1] } : first ? { icao: depId || first.ident, lat: first.lat, lon: first.lon } : null;
  const dest = destLla ? { icao: destId, lat: destLla[0], lon: destLla[1] } : last ? { icao: destId || last.ident, lat: last.lat, lon: last.lon } : null;
  if (!dep || !dest || !dep.icao || !dest.icao) throw new Error("The flight plan has no departure or destination airport.");
  let mid = list;
  if (mid.length && mid[0].type === "airport" && mid[0].ident === dep.icao) mid = mid.slice(1);
  if (mid.length && mid[mid.length - 1].type === "airport" && mid[mid.length - 1].ident === dest.icao) mid = mid.slice(0, -1);
  // Altitudes: kept where they differ from the cruise (a climb, a
  // descent, a level set for a leg); else the point follows the cruise.
  const fixes: PlanFix[] = mid.map((w) => {
    const own = w.alt > 0 && Math.abs(w.alt - cruiseFt) > 50;
    return {
      ident: w.ident || "",
      kind: KIND[w.type] ?? "wpt",
      lat: w.lat,
      lon: w.lon,
      alt_ft: own ? w.alt : cruiseFt,
      stage: own ? "USR" : "CRZ",
      procedure: w.procedure,
    };
  });
  return { title: tag(fp, "Title"), rules, cruiseFt, dep, dest, fixes };
}
