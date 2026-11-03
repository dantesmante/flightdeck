//! Pilot rating.
//!
//! Every eligible flight gets a performance score out of 100: half Landing
//! Quality (touchdown rate, touchdown G, bounces), half Flight
//! Stability (time spent at high G, steep bank and steep pitch). The score
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
use tracker::{handling_source, Approach, ApproachPoint, Handling, LandingReport};

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
    /// None when not recorded (imported and older flights) and for
    /// aerobatic aircraft.
    pub smoothness: Option<Part>,
    /// None when the approach is not known.
    pub approach: Option<Part>,
    /// None when the airspeed was not recorded.
    pub speed: Option<Part>,
    pub roll_rate_max: f64,
    pub pitch_rate_max: f64,
    pub g_rate_max: f64,
    /// On the approach: fastest descent, most bank from 500 ft, and the
    /// widest airspeed swing from 500 ft to the flare.
    pub approach_sink_max: f64,
    pub approach_bank_max: f64,
    pub approach_speed_dev: f64,
    /// Fastest indicated airspeed below 10,000 ft.
    pub speed_max_low: f64,
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
    /// Landing and stability together, 0 to 100. None when not scored.
    pub performance: Option<f64>,
    pub landing: Option<LandingScore>,
    pub safety: Option<SafetyScore>,
    /// Why a flight was not rated, in a few words.
    pub unrated: Option<String>,
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
    whole(score)
}

/// Scores are whole numbers. Anything short of perfect is at most 99, so
/// a small deduction always shows.
pub fn whole(v: f64) -> f64 {
    if v >= 100.0 {
        100.0
    } else {
        v.round().clamp(0.0, 99.0)
    }
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
    // Landing well in a crosswind is harder: each part earns one percent
    // more per knot across, up to a quarter more, never past 100.
    let crosswind_kt = l.crosswind_kt.abs();
    let crosswind_bonus = crosswind_kt.min(scales::CROSSWIND_MAX_KT) / 100.0;
    let part = |raw: f64, weight: f64| Part { score: whole(raw * (1.0 + crosswind_bonus)), weight };
    let rate_raw = curve(l.vs_fpm.abs(), scales::RATE);
    let g_raw = (l.g_peak > 0.0).then(|| curve(l.g_peak, scales::TOUCHDOWN_G));
    let bounce_raw = (!NO_BOUNCES.contains(&l.vs_source.as_str()) || l.bounces > 0).then(|| curve(l.bounces as f64, scales::BOUNCES));
    let (w_rate, w_g, w_bounce) = scales::LANDING_WEIGHTS;
    let mut rate = part(rate_raw, w_rate);
    let mut g = g_raw.map(|v| part(v, w_g));
    let mut bounce = bounce_raw.map(|v| part(v, w_bounce));
    let score = combine(&mut [Some(&mut rate), g.as_mut(), bounce.as_mut()]);
    // The same parts without the bonus, for the breakdown.
    let mut plain = [
        Some(Part { score: whole(rate_raw), weight: w_rate }),
        g_raw.map(|v| Part { score: whole(v), weight: w_g }),
        bounce_raw.map(|v| Part { score: whole(v), weight: w_bounce }),
    ];
    let base_score = combine(&mut plain.iter_mut().map(Option::as_mut).collect::<Vec<_>>());
    Some(LandingScore {
        score,
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

/// Points lost for time past a limit. `bins` are (middle of the bin in
/// degrees, seconds in it). The further past the limit, the more each
/// second costs: it grows with the square of the excess, so a brief
/// slightly steep moment costs little and a sustained extreme one a lot.
fn over_limit(bins: impl Iterator<Item = (f64, f64)>, limit: f64, total_s: f64) -> f64 {
    bins.filter(|(mid, _)| *mid > limit)
        .map(|(mid, secs)| {
            let excess = mid - limit;
            scales::PER_SECOND_PER_DEG2 * excess * excess * secs + scales::PER_PERCENT_PER_DEG * excess * 100.0 * secs / total_s
        })
        .sum()
}

/// 5-degree bins as (middle, seconds).
fn fine_bins(secs: &[f64]) -> impl Iterator<Item = (f64, f64)> + '_ {
    secs.iter().enumerate().map(|(i, s)| (i as f64 * 5.0 + 2.5, *s))
}

/// Coarse bins of older recordings as (middle, seconds).
fn coarse_bins<'a>(secs: &'a [f64], mids: &'a [f64]) -> impl Iterator<Item = (f64, f64)> + 'a {
    mids.iter().copied().zip(secs.iter().copied())
}

/// Points lost for time past a rate limit: bins of `step`, (middle,
/// seconds), cost per second per unit of excess squared.
fn over_rate(secs: &[f64], step: f64, limit: f64, cost: f64) -> f64 {
    secs.iter()
        .enumerate()
        .map(|(i, s)| ((i as f64 + 0.5) * step, *s))
        .filter(|(mid, _)| *mid > limit)
        .map(|(mid, s)| cost * (mid - limit).powi(2) * s)
        .sum()
}

/// What the approach did, and the points it lost.
struct ApproachJudged {
    lost: f64,
    sink_max: f64,
    bank_max: f64,
    speed_dev: f64,
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

/// The median speed of the points (time, speed) within `half` seconds of `t`.
fn speed_around(pts: &[(f64, f64)], t: f64, half: f64) -> f64 {
    median(pts.iter().filter(|p| (p.0 - t).abs() <= half).map(|p| p.1).collect())
}

fn judge_approach(a: &Approach) -> Option<ApproachJudged> {
    let pts = &a.points;
    if pts.is_empty() {
        return None;
    }
    // Each point stands for the time until the next one (or touchdown).
    let dur = |i: usize| -> f64 {
        let next = pts.get(i + 1).map_or(0.0, |p| p.t_s);
        ((next - pts[i].t_s) as f64).clamp(0.0, 5.0)
    };
    let mut j = ApproachJudged { lost: 0.0, sink_max: 0.0, bank_max: 0.0, speed_dev: 0.0 };
    let in_gate = |p: &ApproachPoint| {
        let agl = p.agl_ft as f64;
        (scales::APPROACH_FLARE_FT..=scales::APPROACH_GATE_FT).contains(&agl)
    };
    let descending = |p: &ApproachPoint| (p.vs_fpm as f64) < scales::APPROACH_DESCENDING_FPM;
    // The speed trend from 500 ft to the flare, while descending.
    let speeds: Vec<(f64, f64)> = pts
        .iter()
        .filter(|p| in_gate(p) && descending(p))
        .filter_map(|p| p.ias_kt.map(|v| (p.t_s as f64, v as f64)))
        .collect();
    let judge_speed = speeds.len() >= scales::APPROACH_SPEED_MIN_POINTS;
    let mut speed_lost = 0.0;
    for (i, p) in pts.iter().enumerate() {
        let dt = dur(i);
        let sink = -(p.vs_fpm as f64);
        j.sink_max = j.sink_max.max(sink);
        if sink > scales::APPROACH_SINK_LIMIT_FPM {
            j.lost += scales::APPROACH_SINK_COST * ((sink - scales::APPROACH_SINK_LIMIT_FPM) / 100.0).powi(2) * dt;
        }
        if (p.agl_ft as f64) <= scales::APPROACH_GATE_FT {
            if let Some(b) = p.bank_deg {
                let b = (b as f64).abs();
                j.bank_max = j.bank_max.max(b);
                if b > scales::APPROACH_BANK_LIMIT_DEG {
                    j.lost += scales::APPROACH_BANK_COST * (b - scales::APPROACH_BANK_LIMIT_DEG).powi(2) * dt;
                }
            }
        }
        if let (Some(v), true) = (p.ias_kt, judge_speed) {
            if in_gate(p) && descending(p) {
                // Compared with the speed around that moment, so a steady
                // slow-down is no deviation but a surge or a sag is.
                let typical = speed_around(&speeds, p.t_s as f64, scales::APPROACH_SPEED_WINDOW_S);
                let dev = (v as f64 - typical).abs();
                j.speed_dev = j.speed_dev.max(dev);
                if dev > scales::APPROACH_SPEED_BAND_KT {
                    speed_lost += scales::APPROACH_SPEED_COST * (dev - scales::APPROACH_SPEED_BAND_KT).powi(2) * dt;
                }
            }
        }
    }
    if judge_speed {
        // How much the speed changes over the final, start to end.
        let (t0, t1) = (speeds[0].0, speeds[speeds.len() - 1].0);
        let w = scales::APPROACH_SPEED_WINDOW_S;
        let change = speed_around(&speeds, t1 - w / 2.0, w / 2.0) - speed_around(&speeds, t0 + w / 2.0, w / 2.0);
        if -change > scales::APPROACH_SLOWDOWN_KT {
            speed_lost += scales::APPROACH_TREND_COST * (-change - scales::APPROACH_SLOWDOWN_KT);
        } else if change > scales::APPROACH_SPEEDUP_KT {
            speed_lost += scales::APPROACH_TREND_COST * (change - scales::APPROACH_SPEEDUP_KT);
        }
    }
    j.lost += speed_lost.min(scales::APPROACH_SPEED_MAX_LOSS);
    Some(j)
}

pub fn score_safety(h: &Handling, aerobatic: bool) -> Option<SafetyScore> {
    if !h.is_known() || h.airborne_s < 60.0 {
        return None;
    }
    let total = h.airborne_s;
    let [w_g, w_bank, w_pitch, w_smooth, w_approach, w_speed] = scales::STABILITY_WEIGHTS;
    let score = |lost: f64, weight: f64| Part { score: whole(100.0 - lost), weight };
    let mut g = (!h.g_s.is_empty()).then(|| {
        score(penalty(&h.g_s, total, if aerobatic { scales::G_AEROBATIC } else { scales::G_NORMAL }), w_g)
    });
    let mut bank = (!aerobatic).then(|| {
        let lost = if !h.bank_5.is_empty() {
            over_limit(fine_bins(&h.bank_5), scales::BANK_LIMIT_DEG, total)
        } else {
            // Recorded before the 5-degree steps: bands 0-30, 30-45, 45-60,
            // 60-75, 75-90, beyond.
            over_limit(coarse_bins(&h.bank_s, &[15.0, 37.5, 52.5, 67.5, 82.5, 95.0]), scales::BANK_LIMIT_DEG, total)
        };
        score(lost, w_bank)
    });
    let mut pitch = (!aerobatic).then(|| {
        let lost = if !h.pitch_up_5.is_empty() {
            over_limit(fine_bins(&h.pitch_up_5), scales::PITCH_UP_LIMIT_DEG, total)
                + over_limit(fine_bins(&h.pitch_down_5), scales::PITCH_DOWN_LIMIT_DEG, total)
        } else if !h.pitch_up_s.is_empty() {
            // Up: 0-25, 25-35, 35-45, beyond; down: 0-15, 15-25, 25-35, beyond.
            over_limit(coarse_bins(&h.pitch_up_s, &[12.5, 30.0, 40.0, 50.0]), scales::PITCH_UP_LIMIT_DEG, total)
                + over_limit(coarse_bins(&h.pitch_down_s, &[7.5, 20.0, 30.0, 40.0]), scales::PITCH_DOWN_LIMIT_DEG, total)
        } else {
            // Either way: 0-20, 20-30, 30-40, 40-50, beyond.
            over_limit(coarse_bins(&h.pitch_s, &[10.0, 25.0, 35.0, 45.0, 55.0]), scales::PITCH_UP_LIMIT_DEG, total)
        };
        score(lost, w_pitch)
    });
    let mut smoothness = (!aerobatic && !h.roll_rate_s.is_empty()).then(|| {
        use tracker::handling::{G_RATE_STEP, PITCH_RATE_STEP, ROLL_RATE_STEP};
        let lost = over_rate(&h.roll_rate_s, ROLL_RATE_STEP, scales::ROLL_RATE_LIMIT, scales::ROLL_RATE_COST)
            + over_rate(&h.pitch_rate_s, PITCH_RATE_STEP, scales::PITCH_RATE_LIMIT, scales::PITCH_RATE_COST)
            + over_rate(&h.g_rate_s, G_RATE_STEP, scales::G_RATE_LIMIT, scales::G_RATE_COST);
        score(lost, w_smooth)
    });
    let judged = h.approach.as_ref().and_then(judge_approach);
    let mut approach = judged.as_ref().map(|j| score(j.lost, w_approach));
    let mut speed = (!h.speed_over_s.is_empty()).then(|| {
        let lost: f64 = h
            .speed_over_s
            .iter()
            .enumerate()
            .map(|(i, s)| scales::SPEED_COST * ((i as f64 + 0.5) * tracker::handling::SPEED_STEP_KT).powi(2) * s)
            .sum();
        score(lost, w_speed)
    });
    if g.is_none() && bank.is_none() && approach.is_none() && speed.is_none() {
        // An aerobatic aircraft with nothing else recorded: nothing to judge.
        return None;
    }
    let total_score =
        combine(&mut [g.as_mut(), bank.as_mut(), pitch.as_mut(), smoothness.as_mut(), approach.as_mut(), speed.as_mut()]);
    Some(SafetyScore {
        score: total_score,
        g,
        bank,
        pitch,
        smoothness,
        approach,
        speed,
        roll_rate_max: h.roll_rate_max,
        pitch_rate_max: h.pitch_rate_max,
        g_rate_max: h.g_rate_max,
        approach_sink_max: judged.as_ref().map_or(0.0, |j| j.sink_max),
        approach_bank_max: judged.as_ref().map_or(0.0, |j| j.bank_max),
        approach_speed_dev: judged.as_ref().map_or(0.0, |j| j.speed_dev),
        speed_max_low: h.speed_max_low,
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
            let p = whole(l.score * 0.5 + sf.score * 0.5);
            let e = expected_for(rating);
            // Every rated flight counts in full, whatever it was rated on.
            let k = k_for(rated);
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
