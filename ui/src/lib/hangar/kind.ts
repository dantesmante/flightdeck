// Which drawing suits an aircraft, from its ICAO type, engines and title.

export type ArtKind =
  | "widebody"
  | "airliner"
  | "regional"
  | "bizjet"
  | "turboprop-twin"
  | "turboprop-single"
  | "piston-twin"
  | "piston-high"
  | "piston-low"
  | "helicopter"
  | "glider";

const starts = (t: string, list: string[]) => list.some((p) => t.startsWith(p));

const WIDEBODY = ["A30", "A31F", "A33", "A34", "A35", "A38", "B74", "B76", "B77", "B78", "MD11", "IL96", "DC10", "L101"];
const AIRLINER = ["A31", "A32", "A2", "A22", "BCS", "B73", "B75", "B71", "B72", "MD8", "MD9", "C919", "B38", "B39"];
const REGIONAL = ["CRJ", "E1", "E7", "E9", "E2", "E17", "E19", "E75", "SU95", "RJ", "B46", "F70", "F100", "AJ27"];
const HELI = ["R22", "R44", "R66", "EC", "H1", "H2", "B06", "B407", "B412", "B429", "S76", "S92", "AS3", "AS5", "UH", "CABR", "A109", "A139", "A169"];
const HIGH_PISTON = ["C15", "C17", "C18", "C20", "C21", "C206", "PA18", "PA20", "PA22", "J3", "CUB", "SAVG", "DHC2", "DHC3", "DHC6", "M7", "ICON", "KITF"];
const HIGH_TURBOPROP_SINGLE = ["C208", "KODI", "DHC2", "DHC3", "PC6", "GA8"];

export function artKind(a: { icao_type: string; engine_type: string; engine_count: number; title: string }): ArtKind {
  const t = (a.icao_type || "").toUpperCase();
  const title = (a.title || "").toLowerCase();
  if (a.engine_type === "helo" || starts(t, HELI) || /helicopter|copter/.test(title)) return "helicopter";
  if (a.engine_type === "glider" || /glider|sailplane/.test(title)) return "glider";
  if (a.engine_type === "jet") {
    if (starts(t, WIDEBODY) || a.engine_count >= 3) return "widebody";
    if (starts(t, REGIONAL)) return "regional";
    if (starts(t, AIRLINER) || /airbus|boeing|airliner/.test(title)) return "airliner";
    return "bizjet";
  }
  if (a.engine_type === "turboprop") {
    if (a.engine_count >= 2) return "turboprop-twin";
    return starts(t, HIGH_TURBOPROP_SINGLE) ? "turboprop-single" : "piston-low";
  }
  if (a.engine_count >= 2) return "piston-twin";
  if (starts(t, HIGH_PISTON) || /cessna|cub|beaver|otter/.test(title)) return "piston-high";
  if (t || a.engine_type === "piston") return "piston-low";
  return "piston-high";
}

export function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}

/** The hangar's filter groups. */
export type Category = "ga" | "airliner" | "cargo" | "helicopter" | "bizjet";

export const CATEGORIES: { id: Category; label: string }[] = [
  { id: "ga", label: "General aviation" },
  { id: "airliner", label: "Airliners" },
  { id: "cargo", label: "Cargo" },
  { id: "helicopter", label: "Helicopters" },
  { id: "bizjet", label: "Business jets" },
];

/** Twin turboprops that fly as airliners rather than private aircraft. */
const REGIONAL_TURBOPROP = ["AT4", "AT7", "ATR", "DH8", "SF34", "JS31", "JS32", "JS41", "E120", "D328", "SB20", "F50", "AN24", "AN26", "Q4", "B190", "L410", "DHC7"];
/** A twin turboprop airliner such as an ATR or a Dash 8. */
export function regionalTurboprop(icao: string): boolean {
  return starts((icao || "").toUpperCase(), REGIONAL_TURBOPROP);
}

/** ICAO airline codes of mainly cargo operators. */
const CARGO_AIRLINES = ["FDX", "UPS", "GTI", "CLX", "BCS", "ABW", "CKS", "PAC", "BOX", "DHK", "ABX", "ATN", "NCA", "MPH", "CAO", "CKK", "LCO", "TAY", "AZG", "SQC", "KZR", "GEC", "AHK", "CPA-C", "ICV", "WGN", "SWN", "RCH"];

export function category(a: {
  icao_type: string;
  engine_type: string;
  engine_count: number;
  title: string;
  last_callsign?: string;
}): Category {
  const k = artKind(a);
  if (k === "helicopter") return "helicopter";
  if (k === "bizjet") return "bizjet";
  const title = (a.title || "").toLowerCase();
  const airline = (a.last_callsign || "").toUpperCase().slice(0, 3);
  const freighter = /cargo|freight|\bbcf\b|\bbdsf\b|\b\d{3}-?\d*f\b|\b(sf|erf|lrf|pf)\b|-f\b/.test(title) || CARGO_AIRLINES.includes(airline);
  const airlinerish =
    k === "widebody" || k === "airliner" || k === "regional" ||
    (k === "turboprop-twin" && starts((a.icao_type || "").toUpperCase(), REGIONAL_TURBOPROP));
  if (airlinerish) return freighter ? "cargo" : "airliner";
  return "ga";
}
