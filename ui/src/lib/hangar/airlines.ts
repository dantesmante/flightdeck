// Airlines the hangar knows: ICAO and IATA codes, name and livery colours
// (main colour, accent). Used to tint an aircraft's registration strip.
// Airlines not listed still work: they get a neutral colour of their own.

export interface Airline {
  icao: string;
  iata: string;
  name: string;
  colors: [string, string];
}

const LIST: [string, string, string, string, string][] = [
  ["AAL", "AA", "American Airlines", "#0078d2", "#c30019"],
  ["DAL", "DL", "Delta Air Lines", "#003366", "#c8102e"],
  ["UAL", "UA", "United Airlines", "#002244", "#0a5ec5"],
  ["SWA", "WN", "Southwest Airlines", "#304cb2", "#f9b612"],
  ["ASA", "AS", "Alaska Airlines", "#01426a", "#48c0a8"],
  ["JBU", "B6", "JetBlue", "#003876", "#0033a0"],
  ["NKS", "NK", "Spirit Airlines", "#ffec00", "#000000"],
  ["FFT", "F9", "Frontier Airlines", "#00764b", "#9bd3ae"],
  ["HAL", "HA", "Hawaiian Airlines", "#4b116f", "#e0157c"],
  ["SKW", "OO", "SkyWest Airlines", "#1b3664", "#8aa6c8"],
  ["RPA", "YX", "Republic Airways", "#002f6c", "#9aa3ad"],
  ["ENY", "MQ", "Envoy Air", "#0078d2", "#7fb6e6"],
  ["EDV", "9E", "Endeavor Air", "#003366", "#c8102e"],
  ["AAY", "G4", "Allegiant Air", "#003a70", "#f6a01a"],
  ["SCX", "SY", "Sun Country Airlines", "#f26722", "#002855"],
  ["FDX", "FX", "FedEx Express", "#4d148c", "#ff6600"],
  ["UPS", "5X", "UPS Airlines", "#351c15", "#ffb500"],
  ["GTI", "5Y", "Atlas Air", "#0b2d6b", "#c8102e"],
  ["ABX", "GB", "ABX Air", "#1b365d", "#7a99ac"],
  ["ACA", "AC", "Air Canada", "#d22630", "#1a1a1a"],
  ["WJA", "WS", "WestJet", "#00aaa5", "#0e2a47"],
  ["TSC", "TS", "Air Transat", "#00287a", "#00b3e3"],
  ["POE", "PD", "Porter Airlines", "#002244", "#6f7b85"],
  ["JZA", "QK", "Jazz Aviation", "#d22630", "#f08c00"],
  ["AMX", "AM", "Aeromexico", "#0b2343", "#0091d5"],
  ["VOI", "Y4", "Volaris", "#a6007e", "#00a4e4"],
  ["VIV", "VB", "Viva Aerobus", "#00a650", "#ffffff"],
  ["AVA", "AV", "Avianca", "#da291c", "#1a1a1a"],
  ["CMP", "CM", "Copa Airlines", "#0032a0", "#a7a8aa"],
  ["LAN", "LA", "LATAM", "#1b0088", "#e8114b"],
  ["TAM", "JJ", "LATAM Brasil", "#1b0088", "#e8114b"],
  ["GLO", "G3", "Gol", "#ff6200", "#4a4a4a"],
  ["AZU", "AD", "Azul", "#0033a0", "#00a3e0"],
  ["ARG", "AR", "Aerolineas Argentinas", "#00a1de", "#1a3a6b"],
  ["BAW", "BA", "British Airways", "#075aaa", "#eb2226"],
  ["VIR", "VS", "Virgin Atlantic", "#da0530", "#5a0024"],
  ["EZY", "U2", "easyJet", "#ff6600", "#ffffff"],
  ["RYR", "FR", "Ryanair", "#073590", "#f1c933"],
  ["EXS", "LS", "Jet2", "#e21a23", "#1a1a1a"],
  ["TOM", "BY", "TUI Airways", "#70cbf4", "#d40e14"],
  ["LOG", "LM", "Loganair", "#002b5c", "#8a1538"],
  ["EIN", "EI", "Aer Lingus", "#006272", "#7ac143"],
  ["DLH", "LH", "Lufthansa", "#05164d", "#ffad00"],
  ["CLH", "CL", "Lufthansa CityLine", "#05164d", "#ffad00"],
  ["EWG", "EW", "Eurowings", "#a3006b", "#1b1b4b"],
  ["CFG", "DE", "Condor", "#fdc500", "#1a1a1a"],
  ["TUI", "X3", "TUIfly", "#70cbf4", "#d40e14"],
  ["SWR", "LX", "Swiss", "#e2001a", "#ffffff"],
  ["AUA", "OS", "Austrian Airlines", "#d81e05", "#ffffff"],
  ["BEL", "SN", "Brussels Airlines", "#00205b", "#e2001a"],
  ["AFR", "AF", "Air France", "#002157", "#e30613"],
  ["KLM", "KL", "KLM", "#00a1de", "#003145"],
  ["TRA", "HV", "Transavia", "#00d66c", "#1a1a1a"],
  ["IBE", "IB", "Iberia", "#d7192d", "#fcb817"],
  ["VLG", "VY", "Vueling", "#ffcc00", "#4a4a4a"],
  ["AEA", "UX", "Air Europa", "#003a70", "#0090d0"],
  ["TAP", "TP", "TAP Air Portugal", "#00a650", "#e2001a"],
  ["ITY", "AZ", "ITA Airways", "#002f87", "#0073cf"],
  ["SAS", "SK", "SAS", "#000a64", "#c8102e"],
  ["NAX", "DY", "Norwegian", "#d81939", "#003251"],
  ["FIN", "AY", "Finnair", "#0b1560", "#ffffff"],
  ["ICE", "FI", "Icelandair", "#003a7d", "#f6b800"],
  ["LOT", "LO", "LOT Polish Airlines", "#11397e", "#a7a9ac"],
  ["WZZ", "W6", "Wizz Air", "#c6007e", "#06038d"],
  ["CSA", "OK", "Czech Airlines", "#d7192d", "#003f87"],
  ["AEE", "A3", "Aegean Airlines", "#0c2e5a", "#4a9be0"],
  ["THY", "TK", "Turkish Airlines", "#c70a0c", "#ffffff"],
  ["PGT", "PC", "Pegasus", "#fdc400", "#c4161c"],
  ["SXS", "XQ", "SunExpress", "#ffcc00", "#e63312"],
  ["AFL", "SU", "Aeroflot", "#02458d", "#e40613"],
  ["UAE", "EK", "Emirates", "#d71921", "#c5a466"],
  ["QTR", "QR", "Qatar Airways", "#5c0632", "#a5a5a5"],
  ["ETD", "EY", "Etihad Airways", "#bd8b13", "#3c3c3c"],
  ["FDB", "FZ", "flydubai", "#004a8f", "#f26522"],
  ["SVA", "SV", "Saudia", "#006c35", "#00a1de"],
  ["GFA", "GF", "Gulf Air", "#c39a4b", "#9f1d35"],
  ["OMA", "WY", "Oman Air", "#8a6f3d", "#03838c"],
  ["RJA", "RJ", "Royal Jordanian", "#0c0c0c", "#c8a046"],
  ["ELY", "LY", "El Al", "#0f3e8a", "#ffffff"],
  ["MSR", "MS", "EgyptAir", "#00265c", "#d8a700"],
  ["RAM", "AT", "Royal Air Maroc", "#c1272d", "#006233"],
  ["ETH", "ET", "Ethiopian Airlines", "#00843d", "#fcdd09"],
  ["KQA", "KQ", "Kenya Airways", "#c8102e", "#000000"],
  ["SAA", "SA", "South African Airways", "#00205b", "#e03c31"],
  ["AIC", "AI", "Air India", "#da0e29", "#7d1d2e"],
  ["IGO", "6E", "IndiGo", "#001b94", "#ffffff"],
  ["SIA", "SQ", "Singapore Airlines", "#1d3d77", "#f9a61a"],
  ["TGW", "TR", "Scoot", "#ffcc00", "#000000"],
  ["MAS", "MH", "Malaysia Airlines", "#003e7e", "#d71921"],
  ["AXM", "AK", "AirAsia", "#ff0000", "#ffffff"],
  ["THA", "TG", "Thai Airways", "#5c2d91", "#c8a2c8"],
  ["GIA", "GA", "Garuda Indonesia", "#00597c", "#2db5e5"],
  ["PAL", "PR", "Philippine Airlines", "#002d72", "#c8102e"],
  ["CEB", "5J", "Cebu Pacific", "#fdbb30", "#0b5ea8"],
  ["HVN", "VN", "Vietnam Airlines", "#00646f", "#d4a017"],
  ["CPA", "CX", "Cathay Pacific", "#005d63", "#a6a6a6"],
  ["CCA", "CA", "Air China", "#d71920", "#a6a6a6"],
  ["CES", "MU", "China Eastern", "#1b2f6e", "#c8102e"],
  ["CSN", "CZ", "China Southern", "#1a6eb5", "#c8102e"],
  ["CHH", "HU", "Hainan Airlines", "#c8102e", "#d4a017"],
  ["EVA", "BR", "EVA Air", "#00703c", "#f18c00"],
  ["CAL", "CI", "China Airlines", "#003a70", "#e8a5b7"],
  ["KAL", "KE", "Korean Air", "#00256c", "#8bc5e8"],
  ["AAR", "OZ", "Asiana Airlines", "#2c2b6e", "#c8102e"],
  ["JAL", "JL", "Japan Airlines", "#c00000", "#ffffff"],
  ["ANA", "NH", "All Nippon Airways", "#13448f", "#00a1e9"],
  ["QFA", "QF", "Qantas", "#e40000", "#ffffff"],
  ["JST", "JQ", "Jetstar", "#ff5a00", "#1a1a1a"],
  ["VOZ", "VA", "Virgin Australia", "#e10a0a", "#5b0c3d"],
  ["ANZ", "NZ", "Air New Zealand", "#000000", "#00b2a9"],
  ["FJI", "FJ", "Fiji Airways", "#0d3a6f", "#a87e3d"],
  ["CLX", "CV", "Cargolux", "#e2001a", "#1d3f87"],
  ["BOX", "3S", "AeroLogic", "#05164d", "#ffad00"],
  ["GEC", "LH", "Lufthansa Cargo", "#05164d", "#ffad00"],
  ["DHK", "D0", "DHL Air", "#ffcc00", "#d40511"],
  ["BCS", "QY", "European Air Transport (DHL)", "#ffcc00", "#d40511"],
  ["CKS", "K4", "Kalitta Air", "#1b3664", "#c8102e"],
  ["NCA", "KZ", "Nippon Cargo Airlines", "#0b2d6b", "#e60012"],
  ["NJE", "NJ", "NetJets Europe", "#1a1a1a", "#c0c0c0"],
  ["EJA", "1I", "NetJets", "#1a1a1a", "#c0c0c0"],
];

export const AIRLINES: Airline[] = LIST.map(([icao, iata, name, a, b]) => ({ icao, iata, name, colors: [a, b] }));

/** An airline from a code (ICAO or IATA) or its name. */
export function findAirline(q: string | null | undefined): Airline | null {
  const s = (q ?? "").trim();
  if (!s) return null;
  const up = s.toUpperCase();
  return (
    AIRLINES.find((a) => a.icao === up) ??
    AIRLINES.find((a) => a.iata === up) ??
    AIRLINES.find((a) => a.name.toUpperCase() === up) ??
    AIRLINES.find((a) => a.name.toUpperCase().startsWith(up) && up.length >= 3) ??
    null
  );
}

/** Airlines matching what was typed, for the airline field. */
export function suggestAirlines(q: string, n = 8): Airline[] {
  const s = q.trim().toUpperCase();
  if (!s) return [];
  return AIRLINES.filter((a) => a.icao.startsWith(s) || a.iata === s || a.name.toUpperCase().includes(s)).slice(0, n);
}

/** Display name and colours for whatever the pilot typed as the airline. */
export function airlineLook(q: string | null | undefined): { name: string; colors: [string, string] } | null {
  const s = (q ?? "").trim();
  if (!s) return null;
  const a = findAirline(s);
  if (a) return { name: a.name, colors: a.colors };
  // Unknown airline: a steady colour from its name.
  let h = 0;
  for (const c of s.toUpperCase()) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  const hue = h % 360;
  return { name: s, colors: [`hsl(${hue} 45% 32%)`, `hsl(${(hue + 40) % 360} 55% 55%)`] };
}
