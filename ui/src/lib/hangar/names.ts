// Turn whatever the sim calls an aircraft ("PMDG 737-700 Ryanair (EI-SEV)",
// "FenixA320_ITYEIDTE", "BlueSim C172-2024") into a plain model name such
// as "Boeing 737-700", "Airbus A320" or "Cessna 172 Skyhawk".

/** ICAO type designators (ICAO Doc 8643) and their usual names. */
export const TYPES: Record<string, string> = {
  // Airbus
  A124: "Antonov An-124", A139: "AgustaWestland AW139", A169: "Leonardo AW169", A189: "Leonardo AW189",
  A19N: "Airbus A319neo", A20N: "Airbus A320neo", A21N: "Airbus A321neo",
  A306: "Airbus A300-600", A30B: "Airbus A300B4", A310: "Airbus A310", A318: "Airbus A318", A319: "Airbus A319",
  A320: "Airbus A320", A321: "Airbus A321", A332: "Airbus A330-200", A333: "Airbus A330-300",
  A337: "Airbus A330 Beluga XL", A338: "Airbus A330-800", A339: "Airbus A330-900", A342: "Airbus A340-200",
  A343: "Airbus A340-300", A345: "Airbus A340-500", A346: "Airbus A340-600", A359: "Airbus A350-900",
  A35K: "Airbus A350-1000", A388: "Airbus A380-800", A3ST: "Airbus Beluga", A400: "Airbus A400M",
  BCS1: "Airbus A220-100", BCS3: "Airbus A220-300",
  // Boeing
  B712: "Boeing 717", B721: "Boeing 727-100", B722: "Boeing 727-200", B731: "Boeing 737-100",
  B732: "Boeing 737-200", B733: "Boeing 737-300", B734: "Boeing 737-400", B735: "Boeing 737-500",
  B736: "Boeing 737-600", B737: "Boeing 737-700", B738: "Boeing 737-800", B739: "Boeing 737-900",
  B37M: "Boeing 737 MAX 7", B38M: "Boeing 737 MAX 8", B39M: "Boeing 737 MAX 9", B3XM: "Boeing 737 MAX 10",
  B741: "Boeing 747-100", B742: "Boeing 747-200", B743: "Boeing 747-300", B744: "Boeing 747-400",
  B748: "Boeing 747-8", B74S: "Boeing 747SP", B752: "Boeing 757-200", B753: "Boeing 757-300",
  B762: "Boeing 767-200", B763: "Boeing 767-300", B764: "Boeing 767-400", B772: "Boeing 777-200",
  B77L: "Boeing 777-200LR", B773: "Boeing 777-300", B77W: "Boeing 777-300ER", B778: "Boeing 777-8",
  B779: "Boeing 777-9", B788: "Boeing 787-8", B789: "Boeing 787-9", B78X: "Boeing 787-10",
  B52: "Boeing B-52", B17: "Boeing B-17", B29: "Boeing B-29", C17: "Boeing C-17 Globemaster", P8: "Boeing P-8 Poseidon",
  K35R: "Boeing KC-135", E3TF: "Boeing E-3 Sentry", B377: "Boeing 377 Stratocruiser", B307: "Boeing 307 Stratoliner",
  // McDonnell Douglas, Lockheed
  DC3: "Douglas DC-3", DC4: "Douglas DC-4", DC6: "Douglas DC-6", DC10: "McDonnell Douglas DC-10",
  MD11: "McDonnell Douglas MD-11", MD82: "McDonnell Douglas MD-82", MD83: "McDonnell Douglas MD-83",
  MD88: "McDonnell Douglas MD-88", MD90: "McDonnell Douglas MD-90", DC93: "Douglas DC-9-30",
  L101: "Lockheed L-1011 TriStar", C130: "Lockheed C-130 Hercules", C5M: "Lockheed C-5 Galaxy",
  CONI: "Lockheed Constellation", L049: "Lockheed Constellation", P38: "Lockheed P-38 Lightning",
  // Regional
  AT43: "ATR 42-300", AT45: "ATR 42-500", AT46: "ATR 42-600", AT72: "ATR 72", AT75: "ATR 72-500", AT76: "ATR 72-600",
  DH8A: "De Havilland Dash 8-100", DH8B: "De Havilland Dash 8-200", DH8C: "De Havilland Dash 8-300",
  DH8D: "De Havilland Dash 8-400", DHC6: "De Havilland Twin Otter", DHC2: "De Havilland Beaver",
  DHC3: "De Havilland Otter", DHC7: "De Havilland Dash 7", DHC1: "De Havilland Chipmunk", DH82: "De Havilland Tiger Moth",
  CRJ1: "Bombardier CRJ100", CRJ2: "Bombardier CRJ200", CRJ7: "Bombardier CRJ700", CRJ9: "Bombardier CRJ900",
  CRJX: "Bombardier CRJ1000", E135: "Embraer ERJ 135", E145: "Embraer ERJ 145", E170: "Embraer E170",
  E175: "Embraer E175", E190: "Embraer E190", E195: "Embraer E195", E290: "Embraer E190-E2", E295: "Embraer E195-E2",
  E110: "Embraer Bandeirante", E120: "Embraer Brasilia", SF34: "Saab 340", SB20: "Saab 2000",
  JS31: "BAe Jetstream 31", JS32: "BAe Jetstream 32", JS41: "BAe Jetstream 41", B461: "BAe 146-100",
  B462: "BAe 146-200", B463: "BAe 146-300", RJ85: "Avro RJ85", RJ1H: "Avro RJ100", F50: "Fokker 50",
  F70: "Fokker 70", F100: "Fokker 100", F28: "Fokker F28", D328: "Dornier 328", J328: "Dornier 328JET",
  D228: "Dornier 228", SU95: "Sukhoi Superjet 100", L410: "Let L-410", B190: "Beechcraft 1900", A748: "Hawker Siddeley 748",
  // Business jets
  C25A: "Cessna Citation CJ2", C25B: "Cessna Citation CJ3", C25C: "Cessna Citation CJ4", C525: "Cessna CitationJet",
  C510: "Cessna Citation Mustang", C560: "Cessna Citation V", C56X: "Cessna Citation Excel", C680: "Cessna Citation Sovereign",
  C68A: "Cessna Citation Latitude", C700: "Cessna Citation Longitude", C750: "Cessna Citation X",
  E50P: "Embraer Phenom 100", E55P: "Embraer Phenom 300", E545: "Embraer Praetor 500", E550: "Embraer Praetor 600",
  E35L: "Embraer Legacy 600", GLF4: "Gulfstream IV", GLF5: "Gulfstream V", GLF6: "Gulfstream G650", GA6C: "Gulfstream G600",
  GLEX: "Bombardier Global Express", GL5T: "Bombardier Global 5000", GL7T: "Bombardier Global 7500",
  CL30: "Bombardier Challenger 300", CL35: "Bombardier Challenger 350", CL60: "Bombardier Challenger 600",
  LJ35: "Learjet 35", LJ45: "Learjet 45", LJ75: "Learjet 75", F2TH: "Dassault Falcon 2000", FA7X: "Dassault Falcon 7X",
  FA8X: "Dassault Falcon 8X", FA50: "Dassault Falcon 50", FA20: "Dassault Falcon 20", H25B: "Hawker 800",
  HDJT: "Honda HondaJet", SF50: "Cirrus Vision Jet", PC24: "Pilatus PC-24", EA50: "Eclipse 500", CONC: "Aerospatiale Concorde",
  // Turboprops
  B350: "Beechcraft King Air 350", BE20: "Beechcraft King Air 200", BE9L: "Beechcraft King Air 90",
  BE99: "Beechcraft 99", PC12: "Pilatus PC-12", PC6T: "Pilatus PC-6 Porter", PC21: "Pilatus PC-21", PC7: "Pilatus PC-7",
  PC9: "Pilatus PC-9", TBM7: "Daher TBM 700", TBM8: "Daher TBM 850", TBM9: "Daher TBM 930", TBM910: "Daher TBM 910",
  C208: "Cessna 208 Caravan", C08T: "Cessna 408 SkyCourier", KODI: "Quest Kodiak 100", M600: "Piper M600",
  PAY3: "Piper Cheyenne III", P46T: "Piper Malibu Meridian", AN2: "Antonov An-2", GA8: "GippsAero Airvan 8",
  // Piston singles and twins
  C150: "Cessna 150", C152: "Cessna 152", C162: "Cessna 162 Skycatcher", C170: "Cessna 170", C172: "Cessna 172 Skyhawk",
  C177: "Cessna 177 Cardinal", C182: "Cessna 182 Skylane", C206: "Cessna 206 Stationair", C210: "Cessna 210 Centurion",
  C310: "Cessna 310", C337: "Cessna 337 Skymaster", C340: "Cessna 340", C414: "Cessna 414", C421: "Cessna 421",
  C140: "Cessna 140", C195: "Cessna 195",
  PA18: "Piper PA-18 Super Cub", PA28: "Piper PA-28 Cherokee", P28A: "Piper PA-28 Cherokee", P28B: "Piper PA-28 Cherokee",
  P28R: "Piper PA-28R Arrow", PA24: "Piper PA-24 Comanche", PA32: "Piper PA-32 Saratoga", PA34: "Piper PA-34 Seneca",
  PA44: "Piper PA-44 Seminole", PA46: "Piper PA-46 Malibu", PA31: "Piper PA-31 Navajo", PA38: "Piper PA-38 Tomahawk",
  J3: "Piper J-3 Cub", PA11: "Piper PA-11 Cub Special", PA12: "Piper PA-12 Super Cruiser",
  BE33: "Beechcraft Debonair", BE35: "Beechcraft Bonanza V35", BE36: "Beechcraft Bonanza G36", BE55: "Beechcraft Baron 55",
  BE58: "Beechcraft Baron G58", BE76: "Beechcraft Duchess", BE18: "Beechcraft Model 18", BE17: "Beechcraft Staggerwing",
  SR20: "Cirrus SR20", SR22: "Cirrus SR22", S22T: "Cirrus SR22T", DA20: "Diamond DA20 Katana", DA40: "Diamond DA40",
  DA42: "Diamond DA42 Twin Star", DA50: "Diamond DA50", DA62: "Diamond DA62", DV20: "Diamond DV20",
  M20P: "Mooney M20", M20T: "Mooney M20 Turbo", M20: "Mooney M20", COL3: "Cessna 350 Corvalis", COL4: "Cessna 400 Corvalis TT",
  P210: "Cessna P210", AA5: "Grumman Tiger", RV7: "Van's RV-7", RV8: "Van's RV-8", RV10: "Van's RV-10", RV14: "Van's RV-14",
  CUB: "Piper Cub", CH60: "Zenith CH 601", CH70: "Zenith CH 701", CH75: "Zenith CH 750", SAVG: "Savage Cub",
  SHRK: "Shark Aero Shark", JAB2: "Jabiru J230", VL3: "JMB VL-3", BRIS: "BRM Bristell", P208: "Tecnam P2008",
  P92: "Tecnam P92", P212: "Tecnam P2012 Traveller", TB20: "Socata TB-20 Trinidad", TB10: "Socata TB-10 Tobago",
  DR40: "Robin DR400", DR10: "Robin DR400", AT3: "Aero AT-3", C42: "Ikarus C42", ICA5: "Icon A5",
  E330: "Extra 330", E300: "Extra 300", PTS1: "Pitts S-1", PTS2: "Pitts S-2", CAP2: "Mudry CAP 10", CP10: "Mudry CAP 10",
  SU26: "Sukhoi Su-26", SU31: "Sukhoi Su-31", YK52: "Yakovlev Yak-52", YK18: "Yakovlev Yak-18T", ZLIN: "Zlin Z-242",
  BL8: "Bellanca Decathlon", CITA: "Bellanca Citabria", HUSK: "Aviat Husky", A1: "Aviat Husky", PIVI: "Pipistrel Virus",
  SNTS: "Pipistrel Velis", JUNK: "Junkers F 13", JU52: "Junkers Ju 52", VE30: "Volocopter", SPIT: "Supermarine Spitfire",
  P51: "North American P-51 Mustang", T6: "North American T-6 Texan", B25: "North American B-25 Mitchell",
  CORS: "Vought F4U Corsair", F4U: "Vought F4U Corsair", DR1: "Fokker Dr.I", WACO: "Waco YMF", GEE: "Gee Bee R-Z",
  VAMP: "De Havilland Vampire", L39: "Aero L-39 Albatros", T38: "Northrop T-38 Talon", F18: "Boeing F/A-18 Super Hornet",
  F18S: "Boeing F/A-18 Super Hornet", F14: "Grumman F-14 Tomcat", F16: "General Dynamics F-16", F35: "Lockheed Martin F-35",
  EUFI: "Eurofighter Typhoon", MIR2: "Dassault Mirage 2000", HAWK: "BAe Hawk", A10: "Fairchild A-10",
  CL41: "Canadair Tutor", CL2T: "Canadair CL-415", CL21: "Canadair CL-215", SHAR: "Short Sunderland", PBY: "Consolidated PBY Catalina",
  // Helicopters
  AS50: "Airbus H125", AS55: "Eurocopter AS355", AS65: "Airbus AS365 Dauphin", EC30: "Airbus H130", EC35: "Airbus H135",
  EC45: "Airbus H145", EC55: "Airbus H155", EC75: "Airbus H175", EC25: "Airbus H225", H160: "Airbus H160",
  B06: "Bell 206 JetRanger", B407: "Bell 407", B412: "Bell 412", B429: "Bell 429", B505: "Bell 505", B47G: "Bell 47",
  B212: "Bell 212", UH1: "Bell UH-1 Huey", R22: "Robinson R22", R44: "Robinson R44", R66: "Robinson R66",
  G2CA: "Guimbal Cabri G2", S76: "Sikorsky S-76", S92: "Sikorsky S-92", H60: "Sikorsky UH-60 Black Hawk",
  CH47: "Boeing CH-47 Chinook", H47: "Boeing CH-47 Chinook", MI8: "Mil Mi-8", A109: "Leonardo AW109", A119: "Leonardo AW119",
  // Gliders and others
  DG80: "DG Flugzeugbau DG-808", LS8: "Schempp-Hirth LS8", DISC: "Schempp-Hirth Discus", VENT: "Schempp-Hirth Ventus",
  ARCP: "Schempp-Hirth Arcus", ASK21: "Schleicher ASK 21", AS21: "Schleicher ASK 21", AS33: "Schleicher AS 33",
  GLID: "Glider", BALL: "Hot air balloon", ZEPL: "Zeppelin NT", ULAC: "Ultralight",
};

/**
 * Name patterns found in sim titles, most specific first. Each returns the
 * model name, or an ICAO type designator to look up.
 */
const PATTERNS: [RegExp, (m: RegExpMatchArray) => string][] = [
  // Boeing families with their variant
  [/\b(?:B-?)?737[- ]?MAX[- ]?(7|8|9|10)\b/i, (m) => `Boeing 737 MAX ${m[1]}`],
  [/(?:\b|B)737[- ]?([1-9])00(ER)?/i, (m) => `Boeing 737-${m[1]}00${m[2] ? "ER" : ""}`],
  [/(?:\b|B)737[- ]?MAX/i, () => "Boeing 737 MAX 8"],
  [/(?:\b|B)747[- ]?(8)(?:i|F)?\b/i, () => "Boeing 747-8"],
  [/(?:\b|B)747[- ]?([1-4])00(F|ER)?/i, (m) => `Boeing 747-${m[1]}00${m[2] ? m[2].toUpperCase() : ""}`],
  [/(?:\b|B)747[- ]?SP/i, () => "Boeing 747SP"],
  [/(?:\b|B)757[- ]?([23])00/i, (m) => `Boeing 757-${m[1]}00`],
  [/(?:\b|B)767[- ]?([234])00(ER|F)?/i, (m) => `Boeing 767-${m[1]}00${m[2] ? m[2].toUpperCase() : ""}`],
  [/(?:\b|B)777[- ]?([23])00[- ]?(ER|LR|F)?/i, (m) => `Boeing 777-${m[1]}00${m[2] ? m[2].toUpperCase() : ""}`],
  [/(?:\b|B)777[- ]?(F|X|8|9)\b/i, (m) => (m[1].toUpperCase() === "F" ? "Boeing 777F" : `Boeing 777-${m[1].toUpperCase()}`)],
  [/(?:\b|B)787[- ]?(8|9|10)\b/i, (m) => `Boeing 787-${m[1]}`],
  [/(?:\b|B)727[- ]?([12])00/i, (m) => `Boeing 727-${m[1]}00`],
  [/(?:\b|B)717\b/i, () => "Boeing 717"],
  // Airbus families
  [/A?220[- ]?([13])00/i, (m) => `Airbus A220-${m[1]}00`],
  [/A32NX/i, () => "Airbus A320neo"],
  [/A339/i, () => "Airbus A330-900"],
  [/A3(19|20|21)[- _]?neo/i, (m) => `Airbus A3${m[1]}neo`],
  [/A3(19|20|21)N\b/i, (m) => `Airbus A3${m[1]}neo`],
  [/A3(18|19|20|21)(?!\d)/i, (m) => `Airbus A3${m[1]}`],
  [/A330[- ]?(2|3|8|9)00/i, (m) => `Airbus A330-${m[1]}00`],
  [/A330[- ]?MRTT/i, () => "Airbus A330 MRTT"],
  [/A340[- ]?([2356])00/i, (m) => `Airbus A340-${m[1]}00`],
  [/A350[- ]?(9|10)00/i, (m) => `Airbus A350-${m[1]}00`],
  [/A35K\b/i, () => "Airbus A350-1000"],
  [/A350/i, () => "Airbus A350"],
  [/A330/i, () => "Airbus A330"],
  [/A340/i, () => "Airbus A340"],
  [/A380/i, () => "Airbus A380-800"],
  [/A300/i, () => "Airbus A300"],
  [/A310/i, () => "Airbus A310"],
  [/A400M?/i, () => "Airbus A400M"],
  [/BelugaXL|Beluga XL/i, () => "Airbus A330 Beluga XL"],
  // Others by name
  [/MD[- ]?11/i, () => "McDonnell Douglas MD-11"],
  [/MD[- ]?8([0-8])/i, (m) => `McDonnell Douglas MD-8${m[1]}`],
  [/DC[- ]?10/i, () => "McDonnell Douglas DC-10"],
  [/DC[- ]?([3469])\b/i, (m) => `Douglas DC-${m[1]}`],
  [/\bC-?47\b/i, () => "Douglas C-47 Skytrain"],
  [/Concorde/i, () => "Aerospatiale Concorde"],
  [/ATR[- ]?72[- ]?600|AT76/i, () => "ATR 72-600"],
  [/ATR[- ]?72/i, () => "ATR 72"],
  [/ATR[- ]?42/i, () => "ATR 42"],
  [/Q400|Dash[- ]?8[- ]?400|DHC[- ]?8[- ]?400/i, () => "De Havilland Dash 8-400"],
  [/Dash[- ]?8[- ]?([123])00/i, (m) => `De Havilland Dash 8-${m[1]}00`],
  [/Dash[- ]?7/i, () => "De Havilland Dash 7"],
  [/Twin[- ]?Otter|DHC[- ]?6/i, () => "De Havilland Twin Otter"],
  [/Beaver|DHC[- ]?2/i, () => "De Havilland Beaver"],
  [/Tiger[- ]?Moth/i, () => "De Havilland Tiger Moth"],
  [/CRJ[- ]?(1|2|7|9|10)00/i, (m) => `Bombardier CRJ${m[1]}00`],
  [/CRJ[- ]?(7|9)\b/i, (m) => `Bombardier CRJ${m[1]}00`],
  [/E(19[05])[- ]?E2/i, (m) => `Embraer E${m[1]}-E2`],
  [/E[- ]?Jet[- ]?(1[79]5|170|190)|\bE(1[79][05])\b|Embraer[- ]?(1[79][05])/i, (m) => `Embraer E${m[1] ?? m[2] ?? m[3]}`],
  [/ERJ[- ]?1(35|40|45)/i, (m) => `Embraer ERJ 1${m[1]}`],
  [/Phenom[- ]?(100|300)/i, (m) => `Embraer Phenom ${m[1]}`],
  [/Praetor[- ]?(500|600)/i, (m) => `Embraer Praetor ${m[1]}`],
  [/Saab[- ]?340|SF34/i, () => "Saab 340"],
  [/Saab[- ]?2000/i, () => "Saab 2000"],
  [/Jetstream[- ]?(31|32|41)/i, (m) => `BAe Jetstream ${m[1]}`],
  [/(?:BAe|Avro)[- ]?146|RJ[- ]?(70|85|100)/i, () => "BAe 146 / Avro RJ"],
  [/Fokker[- ]?(50|70|100|28)/i, (m) => `Fokker ${m[1]}`],
  [/Superjet|SSJ[- ]?100/i, () => "Sukhoi Superjet 100"],
  [/Citation[- ]?CJ([1-4])/i, (m) => `Cessna Citation CJ${m[1]}`],
  [/\bCJ4\b/i, () => "Cessna Citation CJ4"],
  [/Longitude/i, () => "Cessna Citation Longitude"],
  [/Latitude/i, () => "Cessna Citation Latitude"],
  [/Citation[- ]?X\b/i, () => "Cessna Citation X"],
  [/Citation[- ]?Mustang|\bC510\b/i, () => "Cessna Citation Mustang"],
  [/Challenger[- ]?(300|350|600|650)/i, (m) => `Bombardier Challenger ${m[1]}`],
  [/Global[- ]?(5000|6000|7500|8000|Express)/i, (m) => `Bombardier Global ${m[1]}`],
  [/Learjet[- ]?(\d{2})/i, (m) => `Learjet ${m[1]}`],
  [/Falcon[- ]?(7X|8X|10X|900|2000|50|20)/i, (m) => `Dassault Falcon ${m[1].toUpperCase()}`],
  [/Gulfstream[- ]?(G?\d{3,4}|IV|V)|\bG(450|500|550|600|650|700|800)\b/i, (m) => `Gulfstream ${m[1] ? m[1].toUpperCase().replace(/^(\d)/, "G$1") : `G${m[2]}`}`],
  [/HondaJet|HDJT/i, () => "Honda HondaJet"],
  [/Vision[- ]?Jet|SF50/i, () => "Cirrus Vision Jet"],
  [/PC[- ]?12/i, () => "Pilatus PC-12"],
  [/PC[- ]?24/i, () => "Pilatus PC-24"],
  [/\bPC-?6\b|Porter/i, () => "Pilatus PC-6 Porter"],
  [/PC[- ]?21/i, () => "Pilatus PC-21"],
  [/TBM[- ]?(700|850|900|910|930|940|960)/i, (m) => `Daher TBM ${m[1]}`],
  [/King[- ]?Air[- ]?(350|260|250|200|C90|90)/i, (m) => `Beechcraft King Air ${m[1].toUpperCase()}`],
  [/King[- ]?Air/i, () => "Beechcraft King Air"],
  [/Kodiak/i, () => "Quest Kodiak 100"],
  [/(?:Grand[- ]?)?Caravan|C208/i, (m) => (/grand/i.test(m[0]) ? "Cessna 208B Grand Caravan" : "Cessna 208 Caravan")],
  [/SkyCourier|C408/i, () => "Cessna 408 SkyCourier"],
  [/Skyhawk|C[- ]?172|Cessna[- ]?172/i, () => "Cessna 172 Skyhawk"],
  [/Skylane|C[- ]?182/i, () => "Cessna 182 Skylane"],
  [/Stationair|C[- ]?206/i, () => "Cessna 206 Stationair"],
  [/Centurion|C[- ]?210/i, () => "Cessna 210 Centurion"],
  [/Cardinal|C[- ]?177/i, () => "Cessna 177 Cardinal"],
  [/C[- ]?152|Cessna[- ]?152/i, () => "Cessna 152"],
  [/C[- ]?150|Cessna[- ]?150/i, () => "Cessna 150"],
  [/C[- ]?170\b|Cessna[- ]?170/i, () => "Cessna 170"],
  [/C[- ]?195\b|Cessna[- ]?195/i, () => "Cessna 195"],
  [/C[- ]?310\b|Cessna[- ]?310/i, () => "Cessna 310"],
  [/C[- ]?337|Skymaster/i, () => "Cessna 337 Skymaster"],
  [/C[- ]?414|Chancellor/i, () => "Cessna 414"],
  [/Corvalis/i, () => "Cessna Corvalis"],
  [/Baron/i, () => "Beechcraft Baron G58"],
  [/Bonanza/i, () => "Beechcraft Bonanza G36"],
  [/Duchess/i, () => "Beechcraft Duchess"],
  [/Staggerwing/i, () => "Beechcraft Staggerwing"],
  [/Beech(?:craft)?[- ]?(?:Model[- ])?18\b/i, () => "Beechcraft Model 18"],
  [/\b(?:Beech(?:craft)?[- ]?)?1900D?\b/i, () => "Beechcraft 1900"],
  [/Seminole|PA[- ]?44/i, () => "Piper PA-44 Seminole"],
  [/Seneca|PA[- ]?34/i, () => "Piper PA-34 Seneca"],
  [/Comanche|PA[- ]?24/i, () => "Piper PA-24 Comanche"],
  [/Arrow|PA[- ]?28R/i, () => "Piper PA-28R Arrow"],
  [/Warrior|Archer|Cherokee|PA[- ]?28/i, (m) => `Piper PA-28 ${m[0].match(/warrior|archer/i)?.[0].replace(/^./, (c) => c.toUpperCase()) ?? "Cherokee"}`],
  [/Malibu|M350|M500/i, () => "Piper PA-46 Malibu"],
  [/M600/i, () => "Piper M600"],
  [/Navajo|PA[- ]?31/i, () => "Piper PA-31 Navajo"],
  [/Saratoga|PA[- ]?32/i, () => "Piper PA-32 Saratoga"],
  [/Super[- ]?Cub|PA[- ]?18/i, () => "Piper PA-18 Super Cub"],
  [/Savage[- ]?(Cub|Shock|Carbon|Norden)?/i, () => "Savage Cub"],
  [/J[- ]?3[- ]?Cub|Piper Cub/i, () => "Piper J-3 Cub"],
  [/Top[- ]?Cub|Carbon[- ]?Cub|XCub|\bCub\b/i, () => "CubCrafters Cub"],
  [/SR[- ]?22T|SR22 ?Turbo/i, () => "Cirrus SR22T"],
  [/SR[- ]?22/i, () => "Cirrus SR22"],
  [/SR[- ]?20/i, () => "Cirrus SR20"],
  [/\bDA-?(20|40|42|50|62)\b/, (m) => TYPES[`DA${m[1]}`] ?? `Diamond DA${m[1]}`],
  [/DV[- ]?20|Katana/i, () => "Diamond DV20 Katana"],
  [/Mooney|\bM20/i, () => "Mooney M20"],
  [/RV[- ]?(4|6|7|8|10|12|14)/i, (m) => `Van's RV-${m[1]}`],
  [/Extra[- ]?(300|330)/i, (m) => `Extra ${m[1]}`],
  [/Pitts/i, () => "Pitts Special"],
  [/Husky/i, () => "Aviat Husky"],
  [/Icon[- ]?A5/i, () => "Icon A5"],
  [/Robin[- ]?DR[- ]?400|DR[- ]?400/i, () => "Robin DR400"],
  [/Robin[- ]?(?:Cap|CAP)[- ]?10|\bCAP[- ]?10/i, () => "Mudry CAP 10"],
  [/Tecnam[- ]?P[- ]?2008|P2008/i, () => "Tecnam P2008"],
  [/P2012|Traveller/i, () => "Tecnam P2012 Traveller"],
  [/Velis|Virus|Pipistrel/i, (m) => (/velis/i.test(m[0]) ? "Pipistrel Velis Electro" : "Pipistrel Virus")],
  [/Zlin|Savage Norden/i, () => "Zlin Z-242"],
  [/Bristell|BRM/i, () => "BRM Bristell"],
  [/Shark/i, () => "Shark Aero Shark"],
  [/\bVL-?3\b/, () => "JMB VL-3"],
  [/Zenith|CH[- ]?(601|701|750)/i, (m) => `Zenith CH ${m[1] ?? "701"}`],
  [/Junkers[- ]?F[- ]?13/i, () => "Junkers F 13"],
  [/Ju[- ]?52/i, () => "Junkers Ju 52"],
  [/Spitfire/i, () => "Supermarine Spitfire"],
  [/\bP-?51|Mustang/i, () => "North American P-51 Mustang"],
  [/\bT-?6\b|Texan/i, () => "North American T-6 Texan"],
  [/Corsair|F4U/i, () => "Vought F4U Corsair"],
  [/Super[- ]?Hornet|F[-/ ]?A?[- ]?18/i, () => "Boeing F/A-18 Super Hornet"],
  [/Catalina|PBY/i, () => "Consolidated PBY Catalina"],
  [/Gee[- ]?Bee/i, () => "Gee Bee R-Z"],
  [/Waco/i, () => "Waco YMF"],
  [/Hercules|C[- ]?130/i, () => "Lockheed C-130 Hercules"],
  [/Constellation|Connie/i, () => "Lockheed Constellation"],
  [/\bAn-?2\b/, () => "Antonov An-2"],
  [/An[- ]?225/i, () => "Antonov An-225"],
  [/An[- ]?124/i, () => "Antonov An-124"],
  [/CL[- ]?415|Canadair/i, () => "Canadair CL-415"],
  // Helicopters
  [/\bH-?125\b|\bAS-?350\b|Ecureuil/i, () => "Airbus H125"],
  [/\bH-?135\b|\bEC-?135\b/i, () => "Airbus H135"],
  [/\bH-?145\b|\bEC-?145\b/i, () => "Airbus H145"],
  [/\bH-?160\b/i, () => "Airbus H160"],
  [/Bell[- ]?(407|412|429|505|206|47|212)/i, (m) => TYPES[m[1] === "206" ? "B06" : m[1] === "47" ? "B47G" : `B${m[1]}`] ?? `Bell ${m[1]}`],
  [/JetRanger/i, () => "Bell 206 JetRanger"],
  [/Huey|UH[- ]?1/i, () => "Bell UH-1 Huey"],
  [/\bR(22|44|66)\b|Robinson/, (m) => `Robinson R${m[1] ?? "44"}`],
  [/Cabri/i, () => "Guimbal Cabri G2"],
  [/\bS-?76\b/i, () => "Sikorsky S-76"],
  [/Black[- ]?Hawk|UH[- ]?60/i, () => "Sikorsky UH-60 Black Hawk"],
  [/Chinook|CH[- ]?47/i, () => "Boeing CH-47 Chinook"],
  [/AW[- ]?(109|119|139|169|189)/i, (m) => `Leonardo AW${m[1]}`],
  // Gliders
  [/DG[- ]?808/i, () => "DG Flugzeugbau DG-808"],
  [/\bLS-?8\b/, () => "Schempp-Hirth LS8"],
  [/Discus/i, () => "Schempp-Hirth Discus"],
  [/Ventus/i, () => "Schempp-Hirth Ventus"],
  [/Arcus/i, () => "Schempp-Hirth Arcus"],
  [/ASK[- ]?21/i, () => "Schleicher ASK 21"],
  [/\bAS-?33\b/, () => "Schleicher AS 33"],
];

/** A type code from a messy sim value such as "ATCCOM.AC_MODEL_B738.0.text". */
export function cleanType(raw: string): string {
  const t = (raw || "").trim().toUpperCase();
  const m = t.match(/AC_MODEL[ _]?([A-Z0-9]{2,4})/);
  if (m) return m[1];
  return /^[A-Z0-9]{2,4}$/.test(t) ? t : "";
}

/** Type codes long enough to find inside a title without false hits. */
const CODES = Object.keys(TYPES)
  .filter((k) => k.length === 4 && /\d/.test(k) && /[A-Z]/.test(k))
  .sort((a, b) => b.length - a.length);

/** The model name for an aircraft: the pilot's own name if set, else what
 * the sim's title and type code say it is. */
export function modelName(a: { icao_type: string; title: string; model?: string | null }): string {
  if (a.model?.trim()) return a.model.trim();
  const title = (a.title || "").replace(/[_]+/g, " ");
  for (const [re, f] of PATTERNS) {
    const m = title.match(re);
    if (m) return f(m);
  }
  const code = cleanType(a.icao_type);
  if (code && TYPES[code]) return TYPES[code];
  const upper = title.toUpperCase();
  const found = CODES.find((c) => new RegExp(`(?:^|[^A-Z0-9])${c}(?:$|[^0-9])`).test(upper) || upper.includes(c));
  if (found) return TYPES[found];
  if (code) return code;
  // Last resort: the title without livery details in brackets.
  const plain = (a.title || "").replace(/\s*[([].*?[)\]]\s*/g, " ").trim();
  return plain || "Unknown aircraft";
}

/** Suggestions for a model field: names matching what was typed. */
export function suggestModels(q: string, n = 8): { code: string; name: string }[] {
  const s = q.trim().toLowerCase();
  if (s.length < 2) return [];
  const out: { code: string; name: string }[] = [];
  const seen = new Set<string>();
  for (const [code, name] of Object.entries(TYPES)) {
    if (seen.has(name)) continue;
    if (name.toLowerCase().includes(s) || code.toLowerCase().startsWith(s)) {
      out.push({ code, name });
      seen.add(name);
      if (out.length >= n) break;
    }
  }
  return out;
}

/** The ICAO type code for a model name, if it is a known one. */
export function codeFor(name: string): string {
  const n = name.trim().toLowerCase();
  const hit = Object.entries(TYPES).find(([, v]) => v.toLowerCase() === n);
  if (hit) return hit[0];
  const parsed = modelName({ icao_type: "", title: name });
  const back = Object.entries(TYPES).find(([, v]) => v === parsed);
  return back ? back[0] : "";
}
