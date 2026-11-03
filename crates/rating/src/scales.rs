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

/// Landing quality: touchdown rate, touchdown G, bounces.
pub const LANDING_WEIGHTS: (f64, f64, f64) = (0.50, 0.25, 0.25);

/// Crosswind at touchdown beyond this earns no more bonus. Each knot up to
/// it adds 1% to each part of the landing score.
pub const CROSSWIND_MAX_KT: f64 = 25.0;

pub const BOUNCES: &[(f64, f64)] = &[(0.0, 100.0), (1.0, 60.0), (2.0, 25.0), (3.0, 0.0)];

// Flight stability. G: for each band, (points per percent of the time in
// the air, points per second). Bands match tracker::handling.

/// G bands: < -3, -3..-1, -1..0, 0..0.5, 0.5..1.6 (normal), 1.6..2, 2..2.5,
/// 2.5..3.5, 3.5..4.5, 4.5..6, > 6.
pub const G_NORMAL: &[(f64, f64)] = &[
    (8.0, 2.5),
    (8.0, 2.5),
    (4.0, 1.25),
    (1.0, 0.125),
    (0.0, 0.0),
    (0.5, 0.05),
    (2.0, 0.25),
    (5.0, 1.0),
    (8.0, 2.5),
    (8.0, 2.5),
    (8.0, 2.5),
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
/// Bank and pitch limits, degrees. Time beyond them loses points.
pub const BANK_LIMIT_DEG: f64 = 30.0;
pub const PITCH_UP_LIMIT_DEG: f64 = 20.0;
pub const PITCH_DOWN_LIMIT_DEG: f64 = 10.0;
/// Points lost per second past a limit, per degree of excess squared (in
/// 5-degree steps, from the middle of the step): ten seconds at 35-40
/// degrees of bank costs about 5, a minute there about 28.
pub const PER_SECOND_PER_DEG2: f64 = 0.008;
/// Plus, per percent of the flight spent past it, per degree of excess.
pub const PER_PERCENT_PER_DEG: f64 = 0.1;

/// Flight stability is made of these parts, out of 1: G, bank, pitch,
/// smoothness, stable approach, speed limit. Parts a flight has no data
/// for are left out and the rest share their weight.
pub const STABILITY_WEIGHTS: [f64; 6] = [0.20, 0.15, 0.15, 0.20, 0.20, 0.10];

/// Smoothness: how fast bank, pitch and G change. Beyond these rates each
/// second costs points per unit of excess squared.
pub const ROLL_RATE_LIMIT: f64 = 8.0; // degrees a second
pub const PITCH_RATE_LIMIT: f64 = 3.0; // degrees a second
pub const G_RATE_LIMIT: f64 = 0.5; // G a second
pub const ROLL_RATE_COST: f64 = 0.08;
pub const PITCH_RATE_COST: f64 = 0.4;
pub const G_RATE_COST: f64 = 30.0;

/// Stable approach, from the last descent through 1,000 ft above the
/// ground to touchdown: no faster descent than this...
pub const APPROACH_SINK_LIMIT_FPM: f64 = 1000.0;
/// ...points per second per (100 fpm of excess) squared...
pub const APPROACH_SINK_COST: f64 = 0.2;
/// ...and, from 500 ft down, no more bank than this...
pub const APPROACH_GATE_FT: f64 = 500.0;
pub const APPROACH_BANK_LIMIT_DEG: f64 = 15.0;
pub const APPROACH_BANK_COST: f64 = 0.05;
/// ...and, from 500 ft down to the flare (50 ft), airspeed within this of
/// the typical speed around that moment (the median over this many seconds
/// either side), counting only while descending: level stretches, such as
/// a circuit flown low, are left out, and a steady slow-down toward the
/// runway is no deviation.
pub const APPROACH_FLARE_FT: f64 = 50.0;
pub const APPROACH_SPEED_BAND_KT: f64 = 10.0;
pub const APPROACH_SPEED_COST: f64 = 0.1;
pub const APPROACH_SPEED_WINDOW_S: f64 = 10.0;
/// A point counts as descending below this vertical speed.
pub const APPROACH_DESCENDING_FPM: f64 = -100.0;
/// Fewer descending points than this: the speed is not judged.
pub const APPROACH_SPEED_MIN_POINTS: usize = 10;
/// From the start of the final to the flare the speed may drop by this much
/// (the first and last 10 seconds compared), or rise by this much; beyond,
/// each knot costs APPROACH_TREND_COST points.
pub const APPROACH_SLOWDOWN_KT: f64 = 25.0;
pub const APPROACH_SPEEDUP_KT: f64 = 10.0;
pub const APPROACH_TREND_COST: f64 = 2.0;
/// Speed can cost at most this much of the approach score.
pub const APPROACH_SPEED_MAX_LOSS: f64 = 50.0;

/// Speed limit (250 kt below 10,000 ft): points per second per knot of
/// excess squared, in 5-knot steps. Ten seconds at 260 kt costs 13.
pub const SPEED_COST: f64 = 0.008;

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
