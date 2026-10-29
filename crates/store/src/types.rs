//! Aircraft category and class for the logbook, from the ICAO type
//! designator: SEL / MEL (single or multi engine land), SES / MES (on
//! floats), RH (helicopter) or GL (glider). The table decides; the
//! engine count the sim reports is only used for types not in it.

use telemetry::{AircraftInfo, EngineType};

/// Single-engine aeroplanes.
const SINGLE: &[&str] = &[
    // Cessna
    "C120", "C140", "C150", "C152", "C162", "C170", "C172", "C175", "C177", "C180", "C182", "C185", "C188", "C195", "C205",
    "C206", "C207", "C208", "C210", "P210", "C72R", "C77R", "C82R", "COL3", "COL4", "T206", "T210",
    // Piper
    "J3", "J5", "PA11", "PA12", "PA18", "PA20", "PA22", "PA24", "PA25", "PA28", "P28A", "P28B", "P28R", "P28T", "PA32", "P32R",
    "P32T", "PA38", "PA46", "P46T", "M600", "PA36",
    // Beechcraft, Cirrus, Diamond, Mooney and other light singles
    "BE17", "BE19", "BE23", "BE24", "BE33", "BE35", "BE36", "SR20", "SR22", "S22T", "SF50", "DA20", "DA40", "DA50", "DV20",
    "M20P", "M20T", "M20", "M20J", "M20K", "M20R", "AA1", "AA5", "GLAS", "LANC", "RV4", "RV6", "RV7", "RV8", "RV9", "RV10",
    "RV12", "RV14", "CUB", "CH60", "CH70", "CH75", "SAVG", "SHRK", "JAB2", "VL3", "BRIS", "P208", "P92", "TB9", "TB10",
    "TB20", "TB21", "DR40", "DR10", "AT3", "C42", "ICA5", "E300", "E330", "E200", "PTS1", "PTS2", "CAP2", "CP10", "SU26",
    "SU29", "SU31", "YK52", "YK18", "ZLIN", "Z42", "BL8", "CITA", "HUSK", "A1", "PIVI", "SNTS", "VIRU", "FDCT", "TOBA",
    "GA8", "KODI", "AN2", "DHC1", "DH82", "DHC2", "DHC3", "PC6T", "PC6P", "PC7", "PC9", "PC21", "PC12", "TBM7", "TBM8",
    "TBM9", "TBM910", "T6", "TEX2", "P51", "SPIT", "CORS", "F4U", "DR1", "WACO", "GEE", "JUNK", "STAR", "VAMP", "L39",
    "F16", "F35", "MIR2", "HAWK", "CL41", "M346", "AT6T", "AT8T", "AT5T", "AT02", "C82T", "S108", "A210", "BCUB", "DEC",
];

/// Multi-engine aeroplanes (twins, airliners, business and military jets).
const MULTI: &[&str] = &[
    // Piston and turboprop twins
    "BE18", "BE55", "BE58", "BE60", "BE76", "BE95", "BE9L", "BE9T", "BE10", "BE20", "BE30", "B350", "BE99", "B190", "PA23",
    "PA27", "PA30", "PA31", "PA34", "PA44", "PAY1", "PAY2", "PAY3", "PAY4", "P68", "DA42", "DA62", "C303", "C310", "C320",
    "C335", "C336", "C337", "C340", "C402", "C404", "C411", "C414", "C421", "C425", "C441", "C08T", "AC50", "AC90",
    "MU2", "P180", "P212", "DHC4", "DHC6", "DHC7", "D228", "L410", "BN2P", "TRIS", "E110", "E120", "SF34", "SB20",
    "JS31", "JS32", "JS41", "D328", "F27", "F50", "A748", "AN24", "AN26", "AN12", "C130", "C30J", "C160", "CN35",
    "C295", "DC3", "DC4", "DC6", "DC7", "CONI", "L049", "L188", "B17", "B25", "B29", "B52", "P38", "JU52", "PBY",
    "CL2T", "CL21", "AT43", "AT44", "AT45", "AT46", "AT72", "AT73", "AT75", "AT76", "DH8A", "DH8B", "DH8C", "DH8D",
    // Business jets
    "C25A", "C25B", "C25C", "C25M", "C500", "C501", "C510", "C525", "C550", "C551", "C55B", "C560", "C56X", "C650", "C680",
    "C68A", "C700", "C750", "E50P", "E55P", "E545", "E550", "E35L", "EA50", "PC24", "HDJT", "GLF2", "GLF3", "GLF4",
    "GLF5", "GLF6", "GA5C", "GA6C", "GA7C", "GA8C", "G150", "G280", "GLEX", "GL5T", "GL7T", "GL8T", "CL30", "CL35",
    "CL60", "LJ23", "LJ24", "LJ25", "LJ31", "LJ35", "LJ40", "LJ45", "LJ55", "LJ60", "LJ70", "LJ75", "FA10", "FA20",
    "FA50", "FA7X", "FA8X", "F900", "F2TH", "FA6X", "H25A", "H25B", "H25C", "BE40", "PRM1", "SJ30",
    // Airliners
    "A124", "A225", "A306", "A30B", "A310", "A318", "A319", "A320", "A321", "A19N", "A20N", "A21N", "A332", "A333",
    "A337", "A338", "A339", "A342", "A343", "A345", "A346", "A359", "A35K", "A388", "A3ST", "A400", "BCS1", "BCS3",
    "B712", "B721", "B722", "B727", "B731", "B732", "B733", "B734", "B735", "B736", "B737", "B738", "B739", "B37M",
    "B38M", "B39M", "B3XM", "B741", "B742", "B743", "B744", "B748", "B74S", "B752", "B753", "B762", "B763", "B764",
    "B772", "B77L", "B773", "B77W", "B778", "B779", "B788", "B789", "B78X", "DC85", "DC86", "DC87", "DC91", "DC93",
    "DC94", "DC95", "DC10", "MD11", "MD81", "MD82", "MD83", "MD87", "MD88", "MD90", "L101", "B461", "B462", "B463",
    "RJ70", "RJ85", "RJ1H", "F28", "F70", "F100", "CRJ1", "CRJ2", "CRJ7", "CRJ9", "CRJX", "E135", "E145", "E170",
    "E175", "E190", "E195", "E290", "E295", "E75L", "E75S", "J328", "SU95", "CONC", "T134", "T154", "T204", "IL62",
    "IL76", "IL96", "C17", "C5M", "K35R", "KC10", "E3TF", "P8", "C2", "E2",
    // Military twins
    "F18", "F18S", "F14", "F15", "F22", "EUFI", "A10", "T38", "F4", "F5", "F117",
];

/// Helicopters.
const HELI: &[&str] = &[
    "AS50", "AS55", "AS65", "EC20", "EC30", "EC35", "EC45", "EC55", "EC75", "EC25", "H160", "B06", "B06T", "B407", "B412",
    "B429", "B505", "B47G", "B212", "UH1", "R22", "R44", "R66", "G2CA", "S76", "S92", "H60", "CH47", "H47", "MI8", "A109",
    "A119", "A139", "A169", "A189", "H500", "MD52", "MD60", "EXPL", "BK17", "AS32", "S61", "V22",
];

/// Gliders.
const GLIDER: &[&str] = &["DG80", "LS8", "DISC", "VENT", "ARCP", "ASK21", "AS21", "AS33", "GLID", "DG1T", "DUOD", "ASW2", "LS4"];

/// The ICAO type designator from a sim value, which may be a localization
/// key such as "ATCCOM.AC_MODEL_B738.0.text".
pub fn clean_type(raw: &str) -> String {
    let t = raw.trim().to_uppercase();
    if let Some(i) = t.find("AC_MODEL") {
        let rest = t[i + 8..].trim_start_matches(['_', ' ']);
        return rest.chars().take_while(|c| c.is_ascii_alphanumeric()).take(4).collect();
    }
    if (2..=6).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric()) {
        t
    } else {
        String::new()
    }
}

/// Floats or amphibian gear, from the sim's title.
fn on_floats(title: &str) -> bool {
    let t = title.to_lowercase();
    ["float", "amphib", "seaplane", "pontoon", "on water", "flying boat"].iter().any(|w| t.contains(w))
}

/// Category and class: SEL, MEL, SES, MES, RH or GL.
pub fn class_for(icao_type: &str, title: &str, engine_count: u8, engine_type: EngineType) -> &'static str {
    let code = clean_type(icao_type);
    let has = |list: &[&str]| !code.is_empty() && list.contains(&code.as_str());
    if engine_type == EngineType::Helo || has(HELI) {
        return "RH";
    }
    if engine_type == EngineType::Glider || has(GLIDER) {
        return "GL";
    }
    let multi = if has(SINGLE) {
        false
    } else if has(MULTI) {
        true
    } else {
        engine_count > 1
    };
    match (multi, on_floats(title)) {
        (false, false) => "SEL",
        (true, false) => "MEL",
        (false, true) => "SES",
        (true, true) => "MES",
    }
}

pub fn class_of(a: &AircraftInfo) -> &'static str {
    class_for(&a.icao_type, &a.title, a.engine_count, a.engine_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_come_from_the_type() {
        // Sims sometimes report the wrong engine count; the type decides.
        assert_eq!(class_for("C172", "Cessna Skyhawk", 2, EngineType::Piston), "SEL");
        assert_eq!(class_for("B738", "PMDG 737", 1, EngineType::Jet), "MEL");
        assert_eq!(class_for("ATCCOM.AC_MODEL_A20N.0.text", "FlyByWire", 0, EngineType::Unknown), "MEL");
        assert_eq!(class_for("SF50", "Cirrus Vision Jet", 1, EngineType::Jet), "SEL");
        assert_eq!(class_for("DHC2", "Beaver on floats", 1, EngineType::Piston), "SES");
        assert_eq!(class_for("DHC6", "Twin Otter Amphibian", 2, EngineType::Turboprop), "MES");
        assert_eq!(class_for("AS50", "H125", 1, EngineType::Turboprop), "RH");
        assert_eq!(class_for("LS8", "LS8", 0, EngineType::Glider), "GL");
        // Unknown type: the engine count decides.
        assert_eq!(class_for("ZZZZ", "Homebuilt", 2, EngineType::Piston), "MEL");
        assert_eq!(class_for("", "Homebuilt", 1, EngineType::Piston), "SEL");
    }
}

/// An engine type stored as text ("piston", "jet", ...).
pub fn engine_from_str(s: &str) -> EngineType {
    match s {
        "piston" => EngineType::Piston,
        "jet" => EngineType::Jet,
        "turboprop" => EngineType::Turboprop,
        "helo" => EngineType::Helo,
        "glider" => EngineType::Glider,
        "electric" => EngineType::Electric,
        _ => EngineType::Unknown,
    }
}
