//! Every threshold the rating uses, in one place for tuning.

use serde::Serialize;

/// Touchdown rate (ft/min, either sign) to score.
pub const RATE: &[(f64, f64)] = &[
    (60.0, 100.0),
    (180.0, 90.0),
    (300.0, 70.0),
    (450.0, 45.0),
    (600.0, 20.0),
    (800.0, 5.0),
    (1000.0, 0.0),
];

/// Peak G in the touchdown window to score.
pub const TOUCHDOWN_G: &[(f64, f64)] = &[(1.2, 100.0), (1.4, 85.0), (1.7, 55.0), (2.0, 25.0), (2.5, 0.0)];

/// Distance from the centerline, as a share of half the runway's width.
/// 1.0 is the runway edge.
pub const CENTERLINE: &[(f64, f64)] = &[
    (0.05, 100.0),
    (0.15, 90.0),
    (0.30, 70.0),
    (0.50, 40.0),
    (0.80, 10.0),
    (1.00, 0.0),
];

pub const BOUNCES: &[(f64, f64)] = &[(0.0, 100.0), (1.0, 60.0), (2.0, 25.0), (3.0, 0.0)];

// Flight safety: for each band, (points per percent of the time in the
// air, points per second). Bands match tracker::handling.

/// G bands: < -3, -3..-1, -1..0, 0..0.5, 0.5..1.6 (normal), 1.6..2, 2..2.5,
/// 2.5..3.5, 3.5..4.5, 4.5..6, > 6.
pub const G_NORMAL: &[(f64, f64)] = &[
    (8.0, 1.0),
    (8.0, 1.0),
    (4.0, 0.5),
    (1.0, 0.05),
    (0.0, 0.0),
    (0.5, 0.02),
    (2.0, 0.1),
    (5.0, 0.4),
    (8.0, 1.0),
    (8.0, 1.0),
    (8.0, 1.0),
];
/// Aerobatic aircraft: only beyond about +6 and -3 G.
pub const G_AEROBATIC: &[(f64, f64)] = &[
    (8.0, 1.0),
    (2.0, 0.2),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (1.0, 0.05),
    (6.0, 0.5),
];
/// Bank bands: 0..30, 30..45, 45..60, 60..75, 75..90, beyond 90 degrees.
pub const BANK: &[(f64, f64)] = &[(0.0, 0.0), (0.0, 0.0), (1.0, 0.02), (2.5, 0.1), (5.0, 0.3), (8.0, 0.6)];
/// Pitch bands: 0..20, 20..30, 30..40, 40..50, beyond 50 degrees.
pub const PITCH: &[(f64, f64)] = &[(0.0, 0.0), (0.0, 0.0), (1.0, 0.05), (3.0, 0.2), (6.0, 0.5)];

/// Rating from which each expected performance applies.
pub const EXPECTED: &[(f64, f64)] = &[
    (0.0, 50.0),
    (500.0, 60.0),
    (750.0, 70.0),
    (1000.0, 75.0),
    (1250.0, 80.0),
    (1500.0, 85.0),
    (1750.0, 90.0),
    (2000.0, 95.0),
];

/// Minutes in the air from which each XP bonus applies.
pub const LENGTH_BONUS: &[(f64, f64)] = &[
    (15.0, 2.0),
    (30.0, 5.0),
    (60.0, 8.0),
    (120.0, 12.0),
    (240.0, 15.0),
    (360.0, 18.0),
    (480.0, 20.0),
];

/// ICAO types of aerobatic, racing and fighter aircraft.
pub const AEROBATIC_TYPES: &[&str] = &[
    "E300", "E330", "E200", "E230", "E260", "EA30", "PTS1", "PTS2", "S1", "S2", "CAP1", "CAP2", "CP10", "CP23", "SU26",
    "SU29", "SU31", "YK52", "YK50", "ZLIN", "Z42", "Z50", "MX2", "EDGE", "GB1", "GEE", "F16", "F35", "F18",
    "F18S", "F14", "F15", "F22", "EUFI", "RFAL", "TOR", "GRIF", "MIR2", "F4", "F5", "T38", "L39", "M346", "HAWK",
    "PC21", "T6", "TEX2", "PC7", "PC9", "CL41", "A4", "AV8B", "HAR", "SPIT", "P51", "CORS", "F4U", "HURI", "BF09",
    "FW90", "P40", "YK18", "VAMP", "MG29", "SU27", "SU30", "SU35", "J39", "J35", "F104",
];

/// Words in an aircraft's title that mark it as aerobatic or military.
pub const AEROBATIC_WORDS: &[&str] = &[
    "extra 3", "extra 2", "pitts", "aerobatic", "racer", "race ", "reno", "fighter", "hornet", "falcon f-16",
    "f-16", "f-18", "f/a-18", "f-14", "f-15", "f-22", "f-35", "eurofighter", "typhoon", "rafale", "gripen",
    "mustang p-51", "p-51", "spitfire", "corsair", "zivko", "edge 540", "gee bee", "cap 10", "cap 232", "sukhoi",
    "yak-52", "red bull", "l-39", "texan ii", "t-6", "pc-21",
];

/// How the epaulettes look at a rank.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Insignia {
    pub stripes: u8,
    /// Thin stripes rather than full ones.
    pub thin: bool,
    /// "bronze", "silver" or "gold" wings, if any.
    pub wings: Option<&'static str>,
    pub oak_leaf: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rank {
    pub level: u8,
    pub title: &'static str,
    pub min_xp: f64,
    pub insignia: Insignia,
}

const fn rank(level: u8, title: &'static str, min_xp: f64, stripes: u8, thin: bool, wings: Option<&'static str>, oak_leaf: bool) -> Rank {
    Rank { level, title, min_xp, insignia: Insignia { stripes, thin, wings, oak_leaf } }
}

pub const RANKS: &[Rank] = &[
    rank(1, "Student Pilot", 0.0, 0, false, None, false),
    rank(2, "Solo Pilot", 500.0, 1, true, None, false),
    rank(3, "Private Pilot", 1_500.0, 1, false, Some("bronze"), false),
    rank(4, "Line Pilot", 3_500.0, 2, true, None, false),
    rank(5, "Cadet Pilot", 5_000.0, 2, true, Some("silver"), false),
    rank(6, "Second Officer", 7_500.0, 2, false, None, false),
    rank(7, "First Officer", 10_000.0, 3, false, None, false),
    rank(8, "Senior First Officer", 12_000.0, 3, false, Some("gold"), false),
    rank(9, "Captain", 15_000.0, 4, false, None, false),
    rank(10, "Fleet Captain", 30_000.0, 4, false, None, true),
];

/// The rank reached with this much XP, and the next one.
pub fn rank_for(xp: f64) -> (Rank, Option<Rank>) {
    let i = RANKS.iter().rposition(|r| xp >= r.min_xp).unwrap_or(0);
    (RANKS[i], RANKS.get(i + 1).copied())
}
