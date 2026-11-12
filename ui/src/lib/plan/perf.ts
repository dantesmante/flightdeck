// Time en route and fuel for a planned flight: an estimate worked out from
// typical published performance of the aircraft type (climb, cruise,
// descent and approach speeds, fuel flows, usable fuel), the route and
// the cruise altitude, with forecast winds aloft when known.
//
// The figures are rounded, typical book values (manufacturer specs and
// published handbook tables), not any one aircraft's exact numbers: a
// planning guide for the sim, never for real-world flight.
//
// Fuel is worked in pounds throughout; shown in US gallons for piston
// aircraft and in pounds and kilograms for turbines.

export type Kind = "piston" | "turboprop" | "jet" | "heli";
export type Fuel = "avgas" | "jet";

export interface Perf {
  /** What the figures are for, for the card. */
  name: string;
  kind: Kind;
  fuel: Fuel;
  /** Climb: indicated speed (kt) and average rate to the typical cruise altitude (ft/min). */
  climbKt: number;
  climbFpm: number;
  /** Cruise: true airspeed (kt) at the typical cruise altitude (ft). */
  cruiseKt: number;
  cruiseFt: number;
  /** Descent true airspeed (kt) and the approach speed (kt). */
  descentKt: number;
  approachKt: number;
  /** Fuel flow, lb per hour: climb, cruise, descent. */
  ffClimb: number;
  ffCruise: number;
  ffDescent: number;
  /** Usable fuel and taxi fuel, lb. */
  usable: number;
  taxi: number;
}

/** Pounds per US gallon. */
export const LB_PER_GAL: Record<Fuel, number> = { avgas: 6.0, jet: 6.7 };

type Row = [string, Kind, Fuel, number, number, number, number, number, number, number, number, number, number, number];
// name, kind, fuel, climb kt, climb fpm, cruise TAS, cruise ft, descent TAS,
// approach kt, then flows and fuel: in US gal (per hour) for piston and
// diesel types, in lb for turbines — see `gal` below.
const G = (r: Row): Perf => {
  const [name, kind, fuel, climbKt, climbFpm, cruiseKt, cruiseFt, descentKt, approachKt, ffClimb, ffCruise, ffDescent, usable, taxi] = r;
  return { name, kind, fuel, climbKt, climbFpm, cruiseKt, cruiseFt, descentKt, approachKt, ffClimb, ffCruise, ffDescent, usable, taxi };
};
/** A row given in gallons: flows, usable and taxi fuel turned into pounds. */
const gal = (r: Row): Perf => {
  const k = LB_PER_GAL[r[2]];
  const p = G(r);
  return { ...p, ffClimb: p.ffClimb * k, ffCruise: p.ffCruise * k, ffDescent: p.ffDescent * k, usable: p.usable * k, taxi: p.taxi * k };
};

/** Types by ICAO designator (several designators can share a row). */
const TYPES: [string[], Perf][] = [
  // Piston singles and twins (gallons).
  [["C152", "C150"], gal(["Cessna 152", "piston", "avgas", 67, 600, 100, 4000, 105, 60, 7, 6, 4.5, 24.5, 0.8])],
  [["C172"], gal(["Cessna 172", "piston", "avgas", 75, 700, 122, 6000, 125, 65, 11, 8.5, 6, 53, 1.1])],
  [["C182"], gal(["Cessna 182", "piston", "avgas", 85, 850, 140, 7000, 145, 75, 15, 12.5, 9, 87, 1.5])],
  [["C170", "C140"], gal(["Cessna 170", "piston", "avgas", 70, 600, 105, 4000, 110, 60, 8, 7, 5, 37, 0.8])],
  [["P28A", "P28B", "P28R", "PA28"], gal(["Piper PA-28", "piston", "avgas", 76, 650, 118, 6000, 120, 65, 10, 8.5, 6, 48, 1.0])],
  [["PA44"], gal(["Piper Seminole", "piston", "avgas", 88, 1000, 160, 6000, 165, 80, 22, 19, 14, 108, 1.5])],
  [["J3"], gal(["Piper J-3 Cub", "piston", "avgas", 55, 450, 65, 1500, 70, 45, 4.5, 4, 3, 12, 0.3])],
  [["PA18"], gal(["Piper Super Cub", "piston", "avgas", 65, 900, 95, 2500, 100, 50, 10, 8, 6, 36, 0.5])],
  [["SR22", "S22T"], gal(["Cirrus SR22", "piston", "avgas", 105, 1000, 175, 8000, 180, 80, 25, 17.5, 12, 92, 1.5])],
  [["SR20"], gal(["Cirrus SR20", "piston", "avgas", 100, 800, 155, 8000, 160, 75, 14, 11, 8, 56, 1.2])],
  [["BE36"], gal(["Beechcraft Bonanza", "piston", "avgas", 105, 950, 170, 8000, 175, 80, 25, 15, 11, 74, 1.5])],
  [["BE58"], gal(["Beechcraft Baron", "piston", "avgas", 110, 1200, 195, 8000, 200, 95, 34, 28, 20, 194, 2.5])],
  [["DA40"], gal(["Diamond DA40", "piston", "jet", 75, 750, 140, 8000, 145, 70, 7.5, 6, 4.5, 28, 0.6])],
  [["DA42"], gal(["Diamond DA42", "piston", "jet", 80, 1100, 160, 8000, 165, 80, 14, 11, 8, 50, 1.0])],
  [["DA62"], gal(["Diamond DA62", "piston", "jet", 90, 1100, 170, 10000, 175, 85, 18, 14, 10, 86, 1.2])],
  [["DHC2"], gal(["de Havilland Beaver", "piston", "avgas", 85, 900, 120, 5000, 125, 60, 30, 23, 18, 95, 1.5])],
  [["ICON", "A5"], gal(["ICON A5", "piston", "avgas", 65, 600, 85, 3000, 90, 50, 5.5, 4.5, 3.5, 20, 0.4])],
  [["E300", "E330", "EA50"], gal(["Extra 300/330", "piston", "avgas", 100, 2500, 165, 6000, 170, 80, 22, 15, 10, 39, 0.8])],
  [["VL3"], gal(["JMB VL-3", "piston", "avgas", 80, 1200, 150, 6000, 155, 60, 6, 5, 4, 32, 0.3])],
  // Turboprops (pounds).
  [["C208"], G(["Cessna Caravan", "turboprop", "jet", 105, 900, 175, 10000, 180, 85, 420, 360, 250, 2224, 40])],
  [["KODI"], G(["Kodiak 100", "turboprop", "jet", 100, 1100, 174, 12000, 180, 80, 380, 320, 220, 2141, 40])],
  [["TBM7", "TBM8", "TBM9"], G(["Daher TBM 900 series", "turboprop", "jet", 124, 1500, 310, 28000, 260, 90, 520, 380, 250, 1950, 40])],
  [["PC12"], G(["Pilatus PC-12", "turboprop", "jet", 130, 1500, 280, 27000, 250, 95, 520, 380, 250, 2704, 40])],
  [["B350", "BE35"], G(["King Air 350", "turboprop", "jet", 160, 1700, 300, 27000, 270, 110, 900, 700, 450, 3611, 60])],
  [["BE20", "BE30"], G(["King Air 200/300", "turboprop", "jet", 150, 1700, 280, 25000, 250, 105, 800, 600, 400, 3645, 60])],
  [["AT72", "AT75", "AT76"], G(["ATR 72", "turboprop", "jet", 170, 1600, 275, 20000, 260, 110, 2000, 1400, 800, 11000, 100])],
  [["AT45", "AT46"], G(["ATR 42", "turboprop", "jet", 160, 1600, 265, 20000, 250, 105, 1700, 1200, 700, 9900, 90])],
  [["DH8D"], G(["Dash 8-400", "turboprop", "jet", 190, 2000, 330, 25000, 300, 120, 3000, 2300, 1200, 11700, 120])],
  // Business jets.
  [["SF50"], G(["Cirrus Vision Jet", "jet", "jet", 160, 1500, 300, 28000, 260, 90, 650, 450, 300, 1980, 50])],
  [["HDJT"], G(["HondaJet", "jet", "jet", 200, 2500, 420, 41000, 330, 110, 1200, 700, 400, 2860, 60])],
  [["C25C", "C25B", "C25A"], G(["Cessna Citation CJ4", "jet", "jet", 240, 2500, 430, 41000, 340, 120, 2000, 1100, 600, 5828, 80])],
  [["E55P"], G(["Embraer Phenom 300", "jet", "jet", 230, 2800, 440, 43000, 340, 120, 2000, 1150, 600, 5300, 80])],
  [["C700"], G(["Cessna Citation Longitude", "jet", "jet", 250, 3000, 470, 43000, 360, 125, 3200, 1700, 900, 14500, 150])],
  [["GLF6", "GLF5", "GL7T", "GLEX"], G(["Large business jet", "jet", "jet", 270, 3000, 480, 45000, 370, 130, 5500, 3200, 1500, 40000, 250])],
  // Airliners.
  [["CRJ7", "CRJ9", "CRJX", "CRJ2"], G(["Bombardier CRJ", "jet", "jet", 270, 2500, 440, 35000, 330, 135, 5000, 2800, 1200, 19600, 250])],
  [["E170", "E75L", "E75S"], G(["Embraer 170/175", "jet", "jet", 270, 2500, 430, 37000, 330, 130, 5500, 3000, 1300, 20000, 250])],
  [["E190", "E195", "E290", "E295"], G(["Embraer 190/195", "jet", "jet", 280, 2400, 440, 37000, 330, 135, 6500, 3800, 1600, 28000, 300])],
  [["BCS1", "BCS3"], G(["Airbus A220", "jet", "jet", 290, 2400, 445, 37000, 330, 130, 7000, 4200, 1700, 37000, 350])],
  [["A318", "A319", "A19N"], G(["Airbus A319", "jet", "jet", 280, 2300, 450, 37000, 330, 130, 8000, 5000, 2000, 41000, 400])],
  [["A320", "A20N"], G(["Airbus A320", "jet", "jet", 290, 2200, 450, 37000, 330, 135, 8500, 5300, 2000, 41000, 450])],
  [["A321", "A21N"], G(["Airbus A321", "jet", "jet", 300, 2000, 450, 35000, 330, 140, 9500, 6100, 2200, 51000, 450])],
  [["B736", "B737", "B38M", "B37M"], G(["Boeing 737-700/MAX 8", "jet", "jet", 290, 2300, 455, 37000, 330, 140, 8500, 5000, 1900, 46000, 400])],
  [["B738", "B739", "B39M"], G(["Boeing 737-800/900", "jet", "jet", 290, 2200, 455, 37000, 330, 145, 9000, 5500, 2000, 46000, 400])],
  [["B752", "B753"], G(["Boeing 757", "jet", "jet", 290, 2500, 460, 37000, 340, 135, 12000, 7500, 3000, 75000, 500])],
  [["A310", "A306"], G(["Airbus A310/A300", "jet", "jet", 300, 2200, 460, 35000, 340, 140, 14000, 10000, 4000, 110000, 700])],
  [["B762", "B763", "B764"], G(["Boeing 767", "jet", "jet", 300, 2200, 460, 37000, 340, 140, 15000, 10500, 4000, 150000, 800])],
  [["A332", "A333", "A338", "A339"], G(["Airbus A330", "jet", "jet", 300, 2000, 470, 37000, 340, 140, 18000, 12500, 4500, 245000, 1000])],
  [["A359", "A35K"], G(["Airbus A350", "jet", "jet", 300, 2000, 488, 39000, 340, 140, 20000, 13000, 5000, 240000, 1000])],
  [["B788"], G(["Boeing 787-8", "jet", "jet", 300, 2200, 488, 39000, 340, 140, 18000, 10500, 4000, 223000, 1000])],
  [["B789", "B78X"], G(["Boeing 787-9/10", "jet", "jet", 300, 2100, 488, 39000, 340, 145, 19000, 11500, 4200, 223000, 1000])],
  [["B772", "B77L", "B77W", "B778", "B779"], G(["Boeing 777", "jet", "jet", 310, 2000, 490, 35000, 340, 150, 26000, 16000, 6000, 320000, 1500])],
  [["B744", "B748"], G(["Boeing 747-400/8", "jet", "jet", 310, 1800, 490, 35000, 340, 155, 33000, 22000, 8000, 380000, 1500])],
  [["MD11"], G(["McDonnell Douglas MD-11", "jet", "jet", 310, 2000, 480, 35000, 340, 155, 30000, 18000, 6000, 260000, 1300])],
  [["A388"], G(["Airbus A380", "jet", "jet", 310, 1800, 490, 37000, 340, 145, 36000, 25000, 9000, 560000, 2500])],
  // Helicopters.
  [["R22"], gal(["Robinson R22", "heli", "avgas", 60, 1000, 88, 1500, 85, 50, 10, 9, 8, 25, 0.5])],
  [["R44"], gal(["Robinson R44", "heli", "avgas", 65, 1000, 110, 1500, 105, 55, 17, 15, 13, 46.5, 0.8])],
  [["AS50", "EC30", "H125"], G(["Airbus H125", "heli", "jet", 70, 1500, 125, 3000, 120, 60, 340, 300, 250, 950, 20])],
  [["EC35", "H135"], G(["Airbus H135", "heli", "jet", 70, 1500, 125, 3000, 120, 60, 480, 420, 350, 1240, 25])],
  [["B407", "B06"], G(["Bell 407", "heli", "jet", 70, 1300, 120, 3000, 115, 60, 420, 380, 300, 830, 20])],
];

/** Common sim titles to a designator, where the sim gives no type. */
const TITLES: [RegExp, string][] = [
  [/\b152\b/, "C152"],
  [/skyhawk|\b172\b/i, "C172"],
  [/skylane|\b182\b/i, "C182"],
  [/caravan|\b208\b/i, "C208"],
  [/longitude/i, "C700"],
  [/\bcj4\b|citation cj/i, "C25C"],
  [/vision jet|sf50/i, "SF50"],
  [/hondajet/i, "HDJT"],
  [/tbm/i, "TBM9"],
  [/pc-?12/i, "PC12"],
  [/king ?air.*350|\b350i?\b/i, "B350"],
  [/kodiak/i, "KODI"],
  [/bonanza/i, "BE36"],
  [/baron/i, "BE58"],
  [/\bsr22/i, "SR22"],
  [/\bsr20/i, "SR20"],
  [/da ?62/i, "DA62"],
  [/da ?42/i, "DA42"],
  [/da ?40/i, "DA40"],
  [/beaver/i, "DHC2"],
  [/\bcub\b|j-?3/i, "J3"],
  [/icon a5/i, "ICON"],
  [/extra 3/i, "E330"],
  [/a320/i, "A20N"],
  [/a321/i, "A21N"],
  [/a319/i, "A319"],
  [/a310/i, "A310"],
  [/a330/i, "A339"],
  [/a350/i, "A359"],
  [/a380/i, "A388"],
  [/737.?max/i, "B38M"],
  [/737/i, "B738"],
  [/747/i, "B748"],
  [/757/i, "B752"],
  [/767/i, "B763"],
  [/777/i, "B77W"],
  [/787-?10/i, "B78X"],
  [/787-?9/i, "B789"],
  [/787/i, "B788"],
  [/crj/i, "CRJ7"],
  [/e17|e175|e170/i, "E75L"],
  [/e19/i, "E190"],
  [/atr ?72/i, "AT76"],
  [/atr ?42/i, "AT46"],
  [/dash ?8|q400/i, "DH8D"],
  [/md-?11/i, "MD11"],
  [/h125|as350|ecureuil/i, "AS50"],
  [/h135|ec135/i, "EC35"],
  [/\br22\b/i, "R22"],
  [/\br44\b/i, "R44"],
  [/bell 407/i, "B407"],
];

export interface AircraftHint {
  icao: string;
  title: string;
  engineType: string;
  engineCount: number;
  design?: { fuel_capacity_gal: number | null; fuel_lb_per_gal: number | null; cruise_kt: number | null; cruise_alt_ft: number | null } | null;
}

/** The performance to plan with: the type's own row, or one built from
 * the sim's design figures and the kind of engine. None for gliders. */
export function perfFor(a: AircraftHint): (Perf & { source: "type" | "design" }) | null {
  let code = a.icao.trim().toUpperCase();
  let row = TYPES.find(([codes]) => codes.includes(code))?.[1];
  if (!row) {
    code = TITLES.find(([re]) => re.test(a.title))?.[1] ?? "";
    row = TYPES.find(([codes]) => codes.includes(code))?.[1];
  }
  const d = a.design ?? null;
  if (row) {
    // The sim's own tank size, when it gives one.
    const usable = d?.fuel_capacity_gal ? d.fuel_capacity_gal * (d.fuel_lb_per_gal ?? LB_PER_GAL[row.fuel]) : row.usable;
    return { ...row, usable, source: "type" };
  }
  return generic(a);
}

/** A rough performance from the kind of engine and the sim's design figures. */
function generic(a: AircraftHint): (Perf & { source: "design" }) | null {
  const t = a.engineType;
  if (t === "glider" || t === "electric") return null;
  const n = Math.max(1, a.engineCount || 1);
  const d = a.design ?? null;
  const kind: Kind = t === "jet" ? "jet" : t === "turboprop" ? "turboprop" : t === "helo" ? "heli" : "piston";
  const fuel: Fuel = kind === "piston" ? "avgas" : "jet";
  const lbGal = d?.fuel_lb_per_gal ?? LB_PER_GAL[fuel];
  const capLb = d?.fuel_capacity_gal ? d.fuel_capacity_gal * lbGal : null;
  const alt = d?.cruise_alt_ft ?? (kind === "jet" ? 37000 : kind === "turboprop" ? 22000 : kind === "heli" ? 2500 : 6000);
  // The design cruise speed is indicated: about 2% more true per 1,000 ft.
  const ias = d?.cruise_kt ?? (kind === "jet" ? 280 : kind === "turboprop" ? 200 : kind === "heli" ? 115 : 120);
  const tas = kind === "jet" ? Math.min(490, ias * (1 + 0.02 * Math.min(alt, 36000) / 1000)) : ias * (1 + 0.02 * alt / 1000);
  let ffCruise: number;
  if (kind === "piston") ffCruise = Math.max(4, 8.5 + (tas - 122) * 0.19) * LB_PER_GAL.avgas * (n > 1 ? 1.7 : 1);
  else if (kind === "turboprop") ffCruise = tas * 1.3 * n * 0.9;
  else if (kind === "heli") ffCruise = 330 * (n > 1 ? 1.3 : 1);
  else ffCruise = capLb ? 1.84 * capLb ** 0.75 : 1500 * n;
  const usable = capLb ?? ffCruise * (kind === "jet" ? 5 : 4.5);
  const name = a.icao || a.title || "this aircraft";
  return {
    name,
    kind,
    fuel,
    climbKt: kind === "jet" ? 270 : Math.round(ias * 0.7),
    climbFpm: kind === "jet" ? 2200 : kind === "turboprop" ? 1500 : kind === "heli" ? 1200 : 700,
    cruiseKt: Math.round(tas),
    cruiseFt: alt,
    descentKt: kind === "jet" ? 330 : Math.round(tas),
    approachKt: kind === "jet" ? 135 : kind === "turboprop" ? 100 : kind === "heli" ? 55 : Math.round(ias * 0.55),
    ffClimb: ffCruise * (kind === "jet" ? 1.6 : 1.3),
    ffCruise,
    ffDescent: ffCruise * (kind === "jet" ? 0.4 : 0.7),
    usable,
    taxi: kind === "jet" ? Math.max(100, ffCruise * 0.08) : kind === "piston" ? 6 : 40,
    source: "design",
  };
}

// --- The flight ------------------------------------------------------------------

export interface LegPt {
  lat: number;
  lon: number;
}

export interface Estimate {
  /** Minutes in the air, wheels up to wheels down. */
  ete: number;
  climbMin: number;
  cruiseMin: number;
  descentMin: number;
  /** The cruise altitude flown (lower than planned on a short route). */
  cruiseFt: number;
  /** Fuel, lb: taxi, trip, contingency, alternate, final reserve, and the sum. */
  taxi: number;
  trip: number;
  contingency: number;
  alternate: number;
  reserve: number;
  reserveMin: number;
  block: number;
  usable: number;
  /** Average head (+) or tail (−) wind on the route, kt (null: no winds). */
  headwind: number | null;
}

const rad = (d: number) => (d * Math.PI) / 180;
function gcNm(a: LegPt, b: LegPt): number {
  const h = Math.sin(rad(b.lat - a.lat) / 2) ** 2 + Math.cos(rad(a.lat)) * Math.cos(rad(b.lat)) * Math.sin(rad(b.lon - a.lon) / 2) ** 2;
  return 2 * 3440.065 * Math.asin(Math.min(1, Math.sqrt(h)));
}
function bearing(a: LegPt, b: LegPt): number {
  const y = Math.sin(rad(b.lon - a.lon)) * Math.cos(rad(b.lat));
  const x = Math.cos(rad(a.lat)) * Math.sin(rad(b.lat)) - Math.sin(rad(a.lat)) * Math.cos(rad(b.lat)) * Math.cos(rad(b.lon - a.lon));
  return Math.atan2(y, x);
}

/**
 * The estimate for a route (origin, points, destination) at a cruise
 * altitude. `wind` gives the wind at cruise level ([east, north] kt).
 * `rules` decides the final reserve for light aircraft (VFR 30 min, IFR
 * 45 min, as the US rules ask; helicopters 20 min VFR); jets carry 5%
 * contingency and 30 minutes' final reserve, as ICAO advises.
 */
export function estimate(
  p: Perf,
  route: LegPt[],
  cruiseFt: number,
  opts: { rules: "VFR" | "IFR"; alternateNm?: number; wind?: ((lat: number, lon: number) => [number, number]) | null },
): Estimate | null {
  if (route.length < 2) return null;
  const legs = route.slice(1).map((b, i) => ({ a: route[i], b, nm: gcNm(route[i], b), brg: bearing(route[i], b) }));
  const total = legs.reduce((s, l) => s + l.nm, 0);
  if (total < 0.5) return null;

  // Climb and descent for a cruise altitude; a short route turns back
  // down before reaching it.
  const shape = (alt: number) => {
    const rate = p.climbFpm * Math.min(1.3, Math.max(0.5, 1.3 - (0.3 * alt) / p.cruiseFt));
    const climbTas = p.climbKt * (1 + (0.01 * alt) / 1000);
    const climbMin = alt / rate;
    const climbNm = (climbMin / 60) * climbTas;
    const slow = p.kind === "jet" && alt > 11000 ? 5 : 0;
    const descNm = (Math.max(0, alt - 1500) / 1000) * 3 + slow;
    const apprNm = p.kind === "heli" ? 2 : 6;
    return { climbTas, climbNm, descNm, apprNm };
  };
  let alt = Math.max(500, cruiseFt);
  let s = shape(alt);
  for (let i = 0; i < 20 && s.climbNm + s.descNm + s.apprNm > total && alt > 600; i++) {
    alt *= Math.max(0.5, total / (s.climbNm + s.descNm + s.apprNm));
    s = shape(alt);
  }
  // Cruise speed and flow off the type's usual altitude: turbines are
  // slower and thirstier low down; pistons hardly change.
  const f = Math.min(1, alt / p.cruiseFt);
  let cruiseTas = p.kind === "jet" ? p.cruiseKt * (0.7 + 0.3 * f) : p.kind === "turboprop" ? p.cruiseKt * (0.8 + 0.2 * f) : p.cruiseKt;
  // Below 10,000 ft: 250 kt indicated at most.
  if (alt < 10000) cruiseTas = Math.min(cruiseTas, 250 * (1 + (0.02 * alt) / 1000));
  const ffCruise = p.kind === "jet" ? p.ffCruise * (1 + 0.35 * (1 - f)) : p.kind === "turboprop" ? p.ffCruise * (1 + 0.2 * (1 - f)) : p.ffCruise;

  // Along the route a few nm at a time: phase, true airspeed, the wind's
  // part (full at cruise, half climbing and descending), ground speed.
  let mins = 0;
  let lb = 0;
  let climbMin = 0;
  let cruiseMin = 0;
  let descentMin = 0;
  let headSum = 0;
  let headN = 0;
  let d = 0;
  for (const l of legs) {
    const n = Math.max(1, Math.ceil(l.nm / 4));
    const step = l.nm / n;
    for (let i = 0; i < n; i++) {
      const mid = d + step / 2;
      const t = (i + 0.5) / n;
      const lat = l.a.lat + (l.b.lat - l.a.lat) * t;
      const lon = l.a.lon + (l.b.lon - l.a.lon) * t;
      let tas: number;
      let ff: number;
      let w: number;
      let phase: "c" | "r" | "d";
      if (mid < s.climbNm) [tas, ff, w, phase] = [s.climbTas, p.ffClimb, 0.5, "c"];
      else if (mid > total - s.apprNm) [tas, ff, w, phase] = [p.approachKt, p.ffDescent * 1.3, 0.2, "d"];
      else if (mid > total - s.apprNm - s.descNm) [tas, ff, w, phase] = [Math.min(p.descentKt, cruiseTas * 1.05), p.ffDescent, 0.5, "d"];
      else [tas, ff, w, phase] = [cruiseTas, ffCruise, 1, "r"];
      let gs = tas;
      if (opts.wind) {
        const [u, v] = opts.wind(lat, lon);
        const tail = (u * Math.sin(l.brg) + v * Math.cos(l.brg)) * w;
        const cross = (u * Math.cos(l.brg) - v * Math.sin(l.brg)) * w;
        gs = Math.max(tas * 0.3, Math.sqrt(Math.max(0, tas * tas - cross * cross)) + tail);
        if (phase === "r") {
          headSum -= tail;
          headN++;
        }
      }
      const m = (step / gs) * 60;
      mins += m;
      lb += (m / 60) * ff;
      if (phase === "c") climbMin += m;
      else if (phase === "r") cruiseMin += m;
      else descentMin += m;
      d += step;
    }
  }
  // Take-off and departure: a minute at climb power.
  mins += 1;
  lb += p.ffClimb / 60;

  const turbineRules = p.kind === "jet";
  const contingency = turbineRules ? lb * 0.05 : 0;
  const reserveMin = turbineRules ? 30 : p.kind === "heli" && opts.rules === "VFR" ? 20 : opts.rules === "IFR" ? 45 : 30;
  // Holding for jets is slower and leaner than cruise.
  const reserve = (reserveMin / 60) * (turbineRules ? ffCruise * 0.8 : ffCruise);
  const alternate = opts.alternateNm && opts.alternateNm > 0 ? (opts.alternateNm / (cruiseTas * 0.85) + 10 / 60) * ffCruise * 1.05 : 0;
  const block = p.taxi + lb + contingency + alternate + reserve;
  return {
    ete: Math.round(mins),
    climbMin,
    cruiseMin,
    descentMin,
    cruiseFt: Math.round(alt / 100) * 100,
    taxi: p.taxi,
    trip: lb,
    contingency,
    alternate,
    reserve,
    reserveMin,
    block,
    usable: p.usable,
    headwind: headN ? headSum / headN : null,
  };
}

/** Fuel as the pilot of this kind of aircraft reads it. */
export function fuelText(p: Perf, lb: number): { main: string; sub: string } {
  const n = (v: number) => Math.round(v).toLocaleString("en-US");
  if (p.kind === "piston" || (p.kind === "heli" && p.fuel === "avgas")) {
    const g = lb / LB_PER_GAL[p.fuel];
    return { main: `${g < 100 ? g.toFixed(1) : n(g)} gal`, sub: `${n(lb)} lb` };
  }
  return { main: `${n(lb)} lb`, sub: `${n(lb * 0.45359237)} kg` };
}
