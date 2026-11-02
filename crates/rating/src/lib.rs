//! Pilot rating.
//!
//! Every eligible flight gets a performance score out of 100: half Landing
//! Quality (touchdown rate, touchdown G, bounces), half Flight
//! Safety (time spent at high G, steep bank and steep pitch). The score
//! moves the Pilot Rating up or down against what the current rating
//! expects, iRating style. Pilot XP only ever grows: each flight earns its
//! score plus a bonus for time in the air, and XP sets the rank.
//!
//! Thresholds are starting values, to be tuned with real flights. Scores
//! are computed from the raw numbers saved with each flight, so retuning
//! re-scores the whole history.

mod scales;

use serde::{Deserialize, Serialize};
use store::pilot::RatingFlight;
use tracker::{handling_source, Handling, LandingReport};

pub use scales::{rank_for, Insignia, Rank, RANKS};

pub const START_RATING: f64 = 1000.0;
/// XP for a completed flight that FlightDeck could not score (imported,
/// entered by hand, or recorded before the rating existed).
pub const UNSCORED_XP: f64 = 50.0;

/// One part of a category score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Part {
    pub score: f64,
    /// Share of the category, after leaving out parts that do not apply.
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LandingScore {
    pub score: f64,
    pub rate: Part,
    /// None when the touchdown G was not recorded.
    pub g: Option<Part>,
    /// None when bounces were not recorded.
    pub bounce: Option<Part>,
    /// Where the touchdown figures came from: "sim", "derived", "volanta",
    /// "elevatex", or "track" when worked out from the positions.
    pub source: String,
    pub vs_fpm: f64,
    pub g_peak: f64,
    pub bounces: u32,
    /// Crosswind across the aircraft at touchdown, and the share it added
    /// to the landing score (0.12 = +12%).
    pub crosswind_kt: f64,
    pub crosswind_bonus: f64,
    /// The landing score before the crosswind bonus.
    pub base_score: f64,
    /// Earlier landings left out: go-arounds and touch-and-goes.
    pub discarded: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SafetyScore {
    pub score: f64,
    /// None when G was not recorded.
    pub g: Option<Part>,
    /// None for aerobatic and fighter aircraft, flown hard on purpose.
    pub bank: Option<Part>,
    pub pitch: Option<Part>,
    pub aerobatic: bool,
    /// See tracker::handling::source.
    pub source: String,
    pub g_max: f64,
    pub g_min: f64,
    pub bank_max: f64,
    pub pitch_max: f64,
    pub pitch_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlightScore {
    pub flight_id: i64,
    pub start_ms: i64,
    pub dep_icao: String,
    pub arr_icao: String,
    pub icao_type: String,
    /// Landing and safety together, 0 to 100. None when not scored.
    pub performance: Option<f64>,
    pub landing: Option<LandingScore>,
    pub safety: Option<SafetyScore>,
    /// Why a flight was not rated, in a few words.
    pub unrated: Option<String>,
    /// How much of the flight was measured rather than worked out: 1 for a
    /// flight FlightDeck recorded, less for older and imported flights. It
    /// scales how far the flight moves the rating.
    pub confidence: f64,
    /// What was worked out rather than measured, in plain words.
    pub estimated: Vec<String>,
    pub airborne_min: f64,
    pub xp_base: f64,
    pub xp_bonus: f64,
    pub xp: f64,
    pub rating_before: Option<f64>,
    pub rating_after: Option<f64>,
    pub expected: Option<f64>,
    pub k: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RatingPoint {
    pub t_ms: i64,
    pub rating: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Profile {
    pub rating: f64,
    pub rating_peak: f64,
    /// What the current rating expects of the next flight.
    pub expected: f64,
    pub rated_flights: usize,
    pub xp: f64,
    pub rank: Rank,
    pub next_rank: Option<Rank>,
    pub avg_performance: Option<f64>,
    pub avg_landing: Option<f64>,
    pub avg_safety: Option<f64>,
    /// Starting value, then after each rated flight.
    pub history: Vec<RatingPoint>,
    /// Newest first.
    pub flights: Vec<FlightScore>,
}

/// Straight lines between the given points; flat beyond both ends.
pub fn curve(x: f64, pts: &[(f64, f64)]) -> f64 {
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    for w in pts.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    last.1
}

/// Weighted average of the parts that apply, their weights rescaled to 1.
fn combine(parts: &mut [Option<&mut Part>]) -> f64 {
    let total: f64 = parts.iter().flatten().map(|p| p.weight).sum();
    let mut score = 0.0;
    for p in parts.iter_mut().flatten() {
        p.weight /= total;
        score += p.score * p.weight;
    }
    score
}

/// The landing that counts. Earlier landings a minute or more before it
/// were go-arounds or touch-and-goes and are left out; one less than a
/// minute before it was the same landing, bounced high.
fn the_landing(landings: &[LandingReport]) -> Option<(LandingReport, u32)> {
    let fin = landings.iter().rposition(|l| !l.touch_and_go).or(landings.len().checked_sub(1))?;
    let mut first = fin;
    while first > 0 && landings[fin].t_ms - landings[first - 1].t_ms < 60_000 {
        first -= 1;
    }
    let mut l = landings[first].clone();
    for later in &landings[first + 1..=fin] {
        l.g_peak = l.g_peak.max(later.g_peak);
        l.bounces += 1 + later.bounces;
    }
    Some((l, first as u32))
}

/// Touchdown sources that do not count bounces.
const NO_BOUNCES: &[&str] = &["volanta", "track"];

pub fn score_landing(landings: &[LandingReport]) -> Option<LandingScore> {
    let (l, discarded) = the_landing(landings)?;
    let mut rate = Part { score: curve(l.vs_fpm.abs(), scales::RATE), weight: 0.35 };
    let mut g = (l.g_peak > 0.0).then(|| Part { score: curve(l.g_peak, scales::TOUCHDOWN_G), weight: 0.20 });
    let mut bounce = (!NO_BOUNCES.contains(&l.vs_source.as_str()) || l.bounces > 0)
        .then(|| Part { score: curve(l.bounces as f64, scales::BOUNCES), weight: 0.20 });
    let base_score = combine(&mut [Some(&mut rate), g.as_mut(), bounce.as_mut()]);
    // Landing well in a crosswind is harder: up to a quarter more, one
    // percent per knot, never past 100.
    let crosswind_kt = l.crosswind_kt.abs();
    let crosswind_bonus = crosswind_kt.min(scales::CROSSWIND_MAX_KT) / 100.0;
    Some(LandingScore {
        score: (base_score * (1.0 + crosswind_bonus)).min(100.0),
        base_score,
        crosswind_kt,
        crosswind_bonus,
        rate,
        g,
        bounce,
        source: l.vs_source.clone(),
        vs_fpm: l.vs_fpm,
        g_peak: l.g_peak,
        bounces: l.bounces,
        discarded,
    })
}

/// Penalty for time in each band: points per percent of the flight, plus
/// points per second.
fn penalty(seconds: &[f64], total_s: f64, table: &[(f64, f64)]) -> f64 {
    seconds.iter().zip(table).map(|(s, (per_pct, per_s))| per_pct * 100.0 * s / total_s + per_s * s).sum()
}

pub fn score_safety(h: &Handling, aerobatic: bool) -> Option<SafetyScore> {
    if !h.is_known() || h.airborne_s < 60.0 {
        return None;
    }
    let part = |secs: &[f64], table: &[(f64, f64)], weight: f64| Part {
        score: (100.0 - penalty(secs, h.airborne_s, table)).clamp(0.0, 100.0),
        weight,
    };
    let mut g = (!h.g_s.is_empty()).then(|| part(&h.g_s, if aerobatic { scales::G_AEROBATIC } else { scales::G_NORMAL }, 0.50));
    let mut bank = (!aerobatic).then(|| part(&h.bank_s, scales::BANK, 0.25));
    let mut pitch = (!aerobatic).then(|| {
        if h.pitch_up_s.is_empty() {
            // Recorded before nose up and nose down were kept apart.
            part(&h.pitch_s, scales::PITCH_EITHER, 0.25)
        } else {
            let lost = penalty(&h.pitch_up_s, h.airborne_s, scales::PITCH_UP)
                + penalty(&h.pitch_down_s, h.airborne_s, scales::PITCH_DOWN);
            Part { score: (100.0 - lost).clamp(0.0, 100.0), weight: 0.25 }
        }
    });
    if g.is_none() && bank.is_none() {
        // An aerobatic aircraft without G: nothing left to judge.
        return None;
    }
    let score = combine(&mut [g.as_mut(), bank.as_mut(), pitch.as_mut()]);
    Some(SafetyScore {
        score,
        g,
        bank,
        pitch,
        aerobatic,
        source: h.source.clone(),
        g_max: h.g_max,
        g_min: h.g_min,
        bank_max: h.bank_max,
        pitch_max: h.pitch_max,
        pitch_min: h.pitch_min,
    })
}

/// Aerobatic, racing and fighter aircraft are flown hard on purpose.
pub fn is_aerobatic(icao_type: &str, title: &str) -> bool {
    let t = icao_type.trim().to_ascii_uppercase();
    if scales::AEROBATIC_TYPES.contains(&t.as_str()) {
        return true;
    }
    let title = title.to_ascii_lowercase();
    scales::AEROBATIC_WORDS.iter().any(|w| title.contains(w))
}

/// What a rating expects of a flight, out of 100.
pub fn expected_for(rating: f64) -> f64 {
    scales::EXPECTED.iter().rev().find(|(min, _)| rating >= *min).map_or(50.0, |(_, e)| *e)
}

/// How far one flight can move the rating: more while it is new.
pub fn k_for(rated_before: usize) -> f64 {
    match rated_before {
        0..=4 => 48.0,
        5..=19 => 40.0,
        20..=49 => 32.0,
        _ => 24.0,
    }
}

/// XP for time in the air, with diminishing returns.
pub fn length_bonus(airborne_min: f64) -> f64 {
    scales::LENGTH_BONUS.iter().rev().find(|(min, _)| airborne_min >= *min).map_or(0.0, |(_, xp)| *xp)
}

fn unrated_reason(f: &RatingFlight) -> Option<&'static str> {
    Some(match () {
        _ if f.sim == "demo" => "Demo flight",
        _ if f.sim == "manual" || f.source == "manual" => "Entered by hand",
        _ if f.status == "partial" => "Ended before landing",
        _ => return None,
    })
}

/// How far to trust each kind of data, 0 to 1.
fn trust(source: &str) -> f64 {
    match source {
        handling_source::RECORDED | "sim" | "derived" | "volanta" | "elevatex" => 1.0,
        handling_source::POSITIONS => 0.75,
        _ => 0.5,
    }
}

/// What was worked out rather than measured, for the pilot to read.
fn estimates(l: &LandingScore, s: &SafetyScore) -> Vec<String> {
    let mut out = vec![];
    if l.source == "track" {
        out.push("Touchdown rate worked out from the positions".to_string());
    }
    let mut missing = vec![];
    if l.g.is_none() {
        missing.push("touchdown G");
    }
    if l.bounce.is_none() {
        missing.push("bounces");
    }
    if s.g.is_none() && !s.aerobatic {
        missing.push("G-force in flight");
    }
    match s.source.as_str() {
        handling_source::POSITIONS => out.push("Bank and pitch from the positions the flight was logged with".into()),
        handling_source::TRACK => out.push("Bank worked out from the rate of turn, pitch from the climb and descent".into()),
        _ => {}
    }
    if !missing.is_empty() {
        out.push(format!("Not recorded, so left out: {}", missing.join(", ")));
    }
    out
}

/// Time in the air for the length bonus: from wheels up to touchdown only,
/// so time spent parked, taxiing or starting in the air never counts. A
/// flight FlightDeck recorded counts its airborne time without pauses;
/// others count from takeoff to landing. No takeoff, no bonus.
pub fn airborne_minutes(f: &RatingFlight) -> f64 {
    let (Some(up), Some(down)) = (f.takeoff_ms, f.landing_ms) else { return 0.0 };
    match &f.handling {
        Some(h) if h.source == handling_source::RECORDED && h.airborne_s > 0.0 => h.airborne_s / 60.0,
        _ => ((down - up).max(0)) as f64 / 60_000.0,
    }
}

/// Score every flight (oldest first in, newest first out) and work out the
/// pilot's rating, XP and rank.
pub fn profile(flights: &[RatingFlight]) -> Profile {
    let mut rating = START_RATING;
    let mut peak = rating;
    let mut rated = 0usize;
    let mut xp = 0.0;
    let mut history = vec![RatingPoint { t_ms: flights.first().map_or(0, |f| f.start_ms), rating }];
    let (mut sum_p, mut sum_l, mut sum_s) = (0.0, 0.0, 0.0);
    let mut out = Vec::with_capacity(flights.len());
    for f in flights {
        let airborne_min = airborne_minutes(f);
        let mut reason = unrated_reason(f).map(str::to_string);
        let landings: Vec<LandingReport> = match (&f.estimated_landing, f.landings.is_empty()) {
            (Some(l), true) => vec![l.clone()],
            _ => f.landings.clone(),
        };
        let landing = if reason.is_none() { score_landing(&landings) } else { None };
        let safety = match (&reason, &f.handling) {
            (None, Some(h)) => score_safety(h, is_aerobatic(&f.icao_type, &f.title)),
            _ => None,
        };
        if reason.is_none() {
            reason = match (&landing, &safety) {
                (None, _) => Some("Not enough data: no touchdown rate".into()),
                (_, None) if f.handling.as_ref().is_some_and(|h| h.is_known()) => Some("Too short to rate".into()),
                (_, None) => Some("Not enough data: too few positions in the air".into()),
                _ => None,
            };
        }
        let mut s = FlightScore {
            flight_id: f.id,
            start_ms: f.start_ms,
            dep_icao: f.dep_icao.clone(),
            arr_icao: f.arr_icao.clone(),
            icao_type: f.icao_type.clone(),
            performance: None,
            landing: None,
            safety: None,
            unrated: reason.clone(),
            confidence: 0.0,
            estimated: vec![],
            airborne_min,
            xp_base: 0.0,
            xp_bonus: 0.0,
            xp: 0.0,
            rating_before: None,
            rating_after: None,
            expected: None,
            k: None,
        };
        let finished = f.status != "partial";
        if let (None, Some(l), Some(sf)) = (&reason, landing, safety) {
            let p = l.score * 0.5 + sf.score * 0.5;
            let e = expected_for(rating);
            let confidence = trust(&l.source).min(trust(&sf.source));
            // Worked-out flights move the rating less.
            let k = k_for(rated) * confidence;
            s.confidence = confidence;
            s.estimated = estimates(&l, &sf);
            let after = rating + k * (p - e) / 100.0;
            sum_p += p;
            sum_l += l.score;
            sum_s += sf.score;
            s.performance = Some(p);
            s.landing = Some(l);
            s.safety = Some(sf);
            s.rating_before = Some(rating);
            s.rating_after = Some(after);
            s.expected = Some(e);
            s.k = Some(k);
            s.xp_base = p.round();
            rating = after;
            peak = peak.max(rating);
            rated += 1;
            history.push(RatingPoint { t_ms: f.start_ms, rating });
        } else if finished && f.sim != "demo" {
            s.xp_base = UNSCORED_XP;
        }
        if s.xp_base > 0.0 {
            s.xp_bonus = length_bonus(airborne_min);
        }
        s.xp = s.xp_base + s.xp_bonus;
        xp += s.xp;
        out.push(s);
    }
    out.reverse();
    let (rank, next_rank) = rank_for(xp);
    let avg = |sum: f64| (rated > 0).then(|| sum / rated as f64);
    Profile {
        rating,
        rating_peak: peak,
        expected: expected_for(rating),
        rated_flights: rated,
        xp,
        rank,
        next_rank,
        avg_performance: avg(sum_p),
        avg_landing: avg(sum_l),
        avg_safety: avg(sum_s),
        history,
        flights: out,
    }
}

#[cfg(test)]
mod tests;
