//! How the aircraft was handled in the air: time spent in each band of
//! G load, bank and pitch. Kept as raw seconds so the pilot rating can be
//! re-scored when its thresholds are tuned.

use serde::{Deserialize, Serialize};
use telemetry::Telemetry;

/// G load band edges. Bands: below the first edge, between each pair, above
/// the last: 11 bands. Normal flight sits in 0.5 to 1.6 G.
pub const G_EDGES: [f64; 10] = [-3.0, -1.0, 0.0, 0.5, 1.6, 2.0, 2.5, 3.5, 4.5, 6.0];
/// Bank angle band edges (either wing down), degrees: 6 bands.
pub const BANK_EDGES: [f64; 5] = [30.0, 45.0, 60.0, 75.0, 90.0];
/// Pitch band edges (nose up or down), degrees: 5 bands. Kept for flights
/// recorded before nose up and nose down were told apart.
pub const PITCH_EDGES: [f64; 4] = [20.0, 30.0, 40.0, 50.0];
/// Nose-up pitch band edges, degrees: 4 bands. Beyond 25 is excessive.
pub const PITCH_UP_EDGES: [f64; 3] = [25.0, 35.0, 45.0];
/// Nose-down pitch band edges, degrees: 4 bands. Beyond 15 is excessive.
pub const PITCH_DOWN_EDGES: [f64; 3] = [15.0, 25.0, 35.0];

/// Bank and pitch are also kept in 5-degree steps, so their limits can be
/// changed and every flight scored again. Bins: 0-5, 5-10, ... and one for
/// everything past the last.
pub const FINE_STEP_DEG: f64 = 5.0;
/// Bank bins up to 90 degrees, then one beyond.
pub const FINE_BANK_BINS: usize = 19;
/// Pitch bins (nose up, nose down) up to 60 degrees, then one beyond.
pub const FINE_PITCH_BINS: usize = 13;

fn fine(v: f64, bins: usize) -> usize {
    ((v.max(0.0) / FINE_STEP_DEG) as usize).min(bins - 1)
}

/// Smoothness: how fast bank, pitch and G change, measured once a second
/// from one-second averages. Bins: roll rate 1 deg/s, pitch rate 0.5 deg/s,
/// G change 0.05 G/s; 31 each, the last open-ended.
pub const RATE_BINS: usize = 31;
pub const ROLL_RATE_STEP: f64 = 1.0;
pub const PITCH_RATE_STEP: f64 = 0.5;
pub const G_RATE_STEP: f64 = 0.05;

/// The speed limit: 250 knots indicated below 10,000 feet. Time over it is
/// kept in 5-knot steps of excess; 21 bins, the last open-ended.
pub const SPEED_LIMIT_KT: f64 = 250.0;
pub const SPEED_LIMIT_BELOW_FT: f64 = 10_000.0;
pub const SPEED_STEP_KT: f64 = 5.0;
pub const SPEED_BINS: usize = 21;

/// The approach: the last stretch below this height before touchdown, at
/// most this long.
pub const APPROACH_FROM_FT: f64 = 1000.0;
pub const APPROACH_MAX_S: f64 = 180.0;

fn bin(v: f64, step: f64, bins: usize) -> usize {
    ((v.max(0.0) / step) as usize).min(bins - 1)
}

/// One second of the approach.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ApproachPoint {
    /// Seconds before touchdown (negative).
    pub t_s: f32,
    pub agl_ft: f32,
    pub vs_fpm: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_deg: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ias_kt: Option<f32>,
}

/// The final approach of a flight: about one point a second from the last
/// descent through 1,000 ft above the ground to touchdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Approach {
    pub points: Vec<ApproachPoint>,
}

impl Approach {
    /// From points with absolute times, oldest first, ending at the touchdown
    /// at `td_ms`: the last stretch below 1,000 ft, at most three minutes.
    pub fn from_points(points: &[(i64, ApproachPoint)], td_ms: i64) -> Option<Approach> {
        let start = points.iter().rposition(|(_, p)| p.agl_ft as f64 > APPROACH_FROM_FT).map_or(0, |i| i + 1);
        let out: Vec<ApproachPoint> = points[start..]
            .iter()
            .filter(|(t, _)| td_ms - t <= (APPROACH_MAX_S * 1000.0) as i64 && *t <= td_ms)
            .map(|(t, p)| ApproachPoint { t_s: ((t - td_ms) as f64 / 1000.0) as f32, ..*p })
            .collect();
        (!out.is_empty()).then_some(Approach { points: out })
    }
}

/// Running one-second averages for the smoothness rates.
#[derive(Debug, Clone, Default, PartialEq)]
struct Second {
    start: i64,
    n: u32,
    bank: f64,
    pitch: f64,
    g: f64,
    /// The previous second's averages: (mid time, bank, pitch, G).
    prev: Option<(i64, f64, f64, f64)>,
}

/// G is smoothed over about a second, so a single bump of turbulence
/// weighs far less than a sustained pull.
const G_SMOOTH_S: f64 = 1.0;
/// Longer gaps between samples (a stutter, a pause) are not counted.
const MAX_STEP_S: f64 = 2.0;

/// Where a flight's handling figures came from.
pub mod source {
    /// FlightDeck recorded every sample (G, bank and pitch).
    pub const RECORDED: &str = "recorded";
    /// Another tracker's positions, with the attitude it logged.
    pub const POSITIONS: &str = "positions";
    /// Worked out from the track alone: bank from the rate of turn, pitch
    /// from the climb or descent angle. No G.
    pub const TRACK: &str = "track";
    /// Too little data to judge the flight's handling.
    pub const NONE: &str = "none";
}

fn recorded() -> String {
    source::RECORDED.into()
}

/// One position of a flight for estimating its handling afterwards.
/// Attitude values are `None` when the source did not log them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sample {
    pub t_ms: i64,
    pub lat: f64,
    pub lon: f64,
    pub on_ground: bool,
    pub gs_kt: f64,
    pub vs_fpm: f64,
    pub hdg_deg: f64,
    pub bank_deg: Option<f64>,
    pub pitch_deg: Option<f64>,
    pub g_load: Option<f64>,
    /// Altitude above sea level and above the ground, and indicated
    /// airspeed, when the source logged them.
    pub alt_ft: Option<f64>,
    pub agl_ft: Option<f64>,
    pub ias_kt: Option<f64>,
}

/// Positions further apart than this say too little about what happened
/// between them.
const MAX_GAP_S: f64 = 60.0;
/// To judge a flight from positions: at least this long in the air...
const MIN_AIRBORNE_S: f64 = 120.0;
/// ...with positions this close together, as a rule (the median gap).
const MAX_MEDIAN_GAP_S: f64 = 30.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Handling {
    /// See [`source`].
    #[serde(default = "recorded")]
    pub source: String,
    /// Time in the air, seconds, not counting pauses.
    pub airborne_s: f64,
    /// Empty when G was not recorded.
    pub g_s: Vec<f64>,
    pub bank_s: Vec<f64>,
    pub pitch_s: Vec<f64>,
    /// Time nose up and nose down in each band; empty on flights recorded
    /// before they were kept apart.
    #[serde(default)]
    pub pitch_up_s: Vec<f64>,
    #[serde(default)]
    pub pitch_down_s: Vec<f64>,
    /// Seconds in each 5-degree step of bank, nose up and nose down; empty
    /// on flights recorded before they were kept.
    #[serde(default)]
    pub bank_5: Vec<f64>,
    #[serde(default)]
    pub pitch_up_5: Vec<f64>,
    #[serde(default)]
    pub pitch_down_5: Vec<f64>,
    /// Seconds at each roll rate, pitch rate and rate of G change (see
    /// RATE_BINS); empty when not recorded.
    #[serde(default)]
    pub roll_rate_s: Vec<f64>,
    #[serde(default)]
    pub pitch_rate_s: Vec<f64>,
    #[serde(default)]
    pub g_rate_s: Vec<f64>,
    #[serde(default)]
    pub roll_rate_max: f64,
    #[serde(default)]
    pub pitch_rate_max: f64,
    #[serde(default)]
    pub g_rate_max: f64,
    /// Seconds over 250 kt below 10,000 ft, by 5-knot step of excess; empty
    /// when the airspeed was not recorded.
    #[serde(default)]
    pub speed_over_s: Vec<f64>,
    /// Fastest indicated airspeed below 10,000 ft.
    #[serde(default)]
    pub speed_max_low: f64,
    /// The final approach, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approach: Option<Approach>,
    /// Extremes of the smoothed G and of bank and pitch.
    pub g_max: f64,
    pub g_min: f64,
    pub bank_max: f64,
    pub pitch_max: f64,
    pub pitch_min: f64,
    #[serde(skip)]
    g_smooth: Option<f64>,
    #[serde(skip)]
    last_t: Option<i64>,
    #[serde(skip)]
    second: Second,
}

impl Default for Handling {
    fn default() -> Self {
        Handling {
            source: recorded(),
            airborne_s: 0.0,
            g_s: vec![0.0; G_EDGES.len() + 1],
            bank_s: vec![0.0; BANK_EDGES.len() + 1],
            pitch_s: vec![0.0; PITCH_EDGES.len() + 1],
            pitch_up_s: vec![0.0; PITCH_UP_EDGES.len() + 1],
            pitch_down_s: vec![0.0; PITCH_DOWN_EDGES.len() + 1],
            bank_5: vec![0.0; FINE_BANK_BINS],
            pitch_up_5: vec![0.0; FINE_PITCH_BINS],
            pitch_down_5: vec![0.0; FINE_PITCH_BINS],
            roll_rate_s: vec![0.0; RATE_BINS],
            pitch_rate_s: vec![0.0; RATE_BINS],
            g_rate_s: vec![0.0; RATE_BINS],
            roll_rate_max: 0.0,
            pitch_rate_max: 0.0,
            g_rate_max: 0.0,
            speed_over_s: vec![0.0; SPEED_BINS],
            speed_max_low: 0.0,
            approach: None,
            g_max: 1.0,
            g_min: 1.0,
            bank_max: 0.0,
            pitch_max: 0.0,
            pitch_min: 0.0,
            g_smooth: None,
            last_t: None,
            second: Second::default(),
        }
    }
}

/// Which band a value falls in.
pub fn band(edges: &[f64], v: f64) -> usize {
    edges.iter().take_while(|&&e| v >= e).count()
}

impl Sample {
    pub fn from_track(p: &telemetry::TrackPoint) -> Self {
        Sample {
            t_ms: p.t_ms,
            lat: p.lat,
            lon: p.lon,
            on_ground: p.on_ground,
            gs_kt: p.gs_kt as f64,
            vs_fpm: p.vs_fpm as f64,
            hdg_deg: p.hdg_deg as f64,
            alt_ft: Some(p.alt_ft as f64),
            ..Default::default()
        }
    }
}

impl Handling {
    /// Count one airborne sample.
    pub fn add(&mut self, s: &Telemetry) {
        let dt = match self.last_t.replace(s.t_ms) {
            Some(t) => ((s.t_ms - t) as f64 / 1000.0).clamp(0.0, MAX_STEP_S),
            None => 0.0,
        };
        let g = match self.g_smooth {
            Some(prev) => prev + (s.g_load - prev) * (1.0 - (-dt / G_SMOOTH_S).exp()),
            None => s.g_load,
        };
        self.g_smooth = Some(g);
        if dt <= 0.0 {
            return;
        }
        self.airborne_s += dt;
        self.g_s[band(&G_EDGES, g)] += dt;
        self.add_bank(s.bank_deg.abs(), dt);
        self.add_pitch(s.pitch_deg, dt);
        self.g_max = self.g_max.max(g);
        self.g_min = self.g_min.min(g);
        self.bank_max = self.bank_max.max(s.bank_deg.abs());
        self.pitch_max = self.pitch_max.max(s.pitch_deg);
        self.pitch_min = self.pitch_min.min(s.pitch_deg);
        if s.alt_msl_ft < SPEED_LIMIT_BELOW_FT {
            self.add_speed(s.ias_kt, dt);
        }
        self.add_second(s.t_ms, s.bank_deg, s.pitch_deg, s.g_load);
    }

    fn add_speed(&mut self, ias: f64, dt: f64) {
        self.speed_max_low = self.speed_max_low.max(ias);
        if ias > SPEED_LIMIT_KT {
            self.speed_over_s[bin(ias - SPEED_LIMIT_KT, SPEED_STEP_KT, SPEED_BINS)] += dt;
        }
    }

    /// Fold a sample into the current second; when it is complete, count
    /// how fast bank, pitch and G changed since the second before.
    fn add_second(&mut self, t: i64, bank: f64, pitch: f64, g: f64) {
        let sec = &mut self.second;
        if sec.n == 0 {
            sec.start = t;
        }
        sec.n += 1;
        sec.bank += bank;
        sec.pitch += pitch;
        sec.g += g;
        if t - sec.start < 1000 {
            return;
        }
        let n = sec.n as f64;
        let now = ((sec.start + t) / 2, sec.bank / n, sec.pitch / n, sec.g / n);
        let prev = sec.prev.replace(now);
        *sec = Second { prev: sec.prev, ..Default::default() };
        let Some((t0, b0, p0, g0)) = prev else { return };
        let dt = (now.0 - t0) as f64 / 1000.0;
        if !(0.5..=2.5).contains(&dt) {
            return;
        }
        let (roll, pitch_rate, g_rate) = ((now.1 - b0).abs() / dt, (now.2 - p0).abs() / dt, (now.3 - g0).abs() / dt);
        self.roll_rate_s[bin(roll, ROLL_RATE_STEP, RATE_BINS)] += dt;
        self.pitch_rate_s[bin(pitch_rate, PITCH_RATE_STEP, RATE_BINS)] += dt;
        self.g_rate_s[bin(g_rate, G_RATE_STEP, RATE_BINS)] += dt;
        self.roll_rate_max = self.roll_rate_max.max(roll);
        self.pitch_rate_max = self.pitch_rate_max.max(pitch_rate);
        self.g_rate_max = self.g_rate_max.max(g_rate);
    }

    fn add_bank(&mut self, bank: f64, dt: f64) {
        self.bank_s[band(&BANK_EDGES, bank)] += dt;
        self.bank_5[fine(bank, FINE_BANK_BINS)] += dt;
    }

    /// Nose up is positive.
    fn add_pitch(&mut self, pitch: f64, dt: f64) {
        self.pitch_s[band(&PITCH_EDGES, pitch.abs())] += dt;
        if pitch >= 0.0 {
            self.pitch_up_s[band(&PITCH_UP_EDGES, pitch)] += dt;
            self.pitch_down_s[0] += dt;
            self.pitch_up_5[fine(pitch, FINE_PITCH_BINS)] += dt;
            self.pitch_down_5[0] += dt;
        } else {
            self.pitch_down_s[band(&PITCH_DOWN_EDGES, -pitch)] += dt;
            self.pitch_up_s[0] += dt;
            self.pitch_down_5[fine(-pitch, FINE_PITCH_BINS)] += dt;
            self.pitch_up_5[0] += dt;
        }
    }

    /// The aircraft left the air (or the sim paused): the next sample
    /// starts a new stretch.
    pub fn gap(&mut self) {
        self.last_t = None;
        self.second = Second::default();
    }

    /// No handling figures could be worked out for a flight.
    pub fn none() -> Self {
        Handling {
            source: source::NONE.into(),
            g_s: vec![],
            bank_s: vec![],
            pitch_s: vec![],
            pitch_up_s: vec![],
            pitch_down_s: vec![],
            bank_5: vec![],
            pitch_up_5: vec![],
            pitch_down_5: vec![],
            roll_rate_s: vec![],
            pitch_rate_s: vec![],
            g_rate_s: vec![],
            speed_over_s: vec![],
            ..Default::default()
        }
    }

    pub fn is_known(&self) -> bool {
        self.source != source::NONE && self.airborne_s > 0.0
    }

    /// The final approach from positions: the last stretch below 1,000 ft
    /// above the ground before the last touchdown. Height above the ground
    /// is the logged one, or the altitude less `ground_ft` (the arrival
    /// airport's elevation). `None` without heights, or when the positions
    /// are too far apart to follow it (mostly more than 5 s apart, or under
    /// 20 s of it).
    pub fn estimate_approach(samples: &[Sample], ground_ft: Option<f64>) -> Option<Approach> {
        let td = samples.windows(2).rposition(|w| !w[0].on_ground && w[1].on_ground)? + 1;
        let td_ms = samples[td].t_ms;
        let pts: Vec<(i64, ApproachPoint)> = samples[..td]
            .iter()
            .filter(|s| !s.on_ground)
            .filter_map(|s| {
                let agl = s.agl_ft.or_else(|| Some(s.alt_ft? - ground_ft?))?;
                Some((
                    s.t_ms,
                    ApproachPoint {
                        t_s: 0.0,
                        agl_ft: agl.max(0.0) as f32,
                        vs_fpm: s.vs_fpm as f32,
                        bank_deg: s.bank_deg.map(|b| b as f32),
                        ias_kt: s.ias_kt.map(|v| v as f32),
                    },
                ))
            })
            .collect();
        let a = Approach::from_points(&pts, td_ms)?;
        let first = a.points.first()?.t_s;
        let mut gaps: Vec<f32> = a.points.windows(2).map(|w| w[1].t_s - w[0].t_s).collect();
        gaps.push(-a.points.last()?.t_s);
        gaps.sort_by(f32::total_cmp);
        (-first >= 20.0 && gaps[gaps.len() / 2] <= 5.0).then_some(a)
    }

    /// Work out the handling of a flight from its positions (imported, or
    /// recorded before FlightDeck kept handling). Where the positions carry
    /// bank and pitch they are used; otherwise bank comes from the rate of
    /// turn (a coordinated turn: tan(bank) = speed x turn rate / g) and pitch
    /// from the climb or descent angle. Returns `None` when there is not
    /// enough to go on: under two minutes in the air, or positions mostly
    /// more than half a minute apart.
    pub fn estimate(samples: &[Sample]) -> Option<Handling> {
        Self::estimate_with(samples, false)
    }

    /// The same for a track FlightDeck simplified when saving it: points on
    /// a straight line were dropped, so long gaps are straight flight and
    /// still count.
    pub fn estimate_simplified(samples: &[Sample]) -> Option<Handling> {
        Self::estimate_with(samples, true)
    }

    fn estimate_with(samples: &[Sample], simplified: bool) -> Option<Handling> {
        let max_gap = if simplified { 900.0 } else { MAX_GAP_S };
        let with_attitude = samples.iter().filter(|s| !s.on_ground).any(|s| s.bank_deg.is_some() && s.pitch_deg.is_some());
        let with_g = samples.iter().filter(|s| !s.on_ground).any(|s| s.g_load.is_some_and(|g| g != 0.0));
        let with_ias = samples.iter().filter(|s| !s.on_ground).any(|s| s.ias_kt.is_some() && s.alt_ft.is_some());
        let mut h = Handling {
            source: if with_attitude { source::POSITIONS } else { source::TRACK }.into(),
            g_s: if with_g { vec![0.0; G_EDGES.len() + 1] } else { vec![] },
            // Positions are too far apart to judge smoothness.
            roll_rate_s: vec![],
            pitch_rate_s: vec![],
            g_rate_s: vec![],
            speed_over_s: if with_ias { vec![0.0; SPEED_BINS] } else { vec![] },
            ..Default::default()
        };
        let mut gaps = Vec::new();
        for w in samples.windows(2) {
            let (a, b) = (w[0], w[1]);
            let dt = (b.t_ms - a.t_ms) as f64 / 1000.0;
            if a.on_ground || b.on_ground || dt <= 0.0 || dt > max_gap {
                continue;
            }
            gaps.push(dt);
            let (bank, pitch) = match (a.bank_deg, b.bank_deg, a.pitch_deg, b.pitch_deg) {
                (Some(b0), Some(b1), Some(p0), Some(p1)) if with_attitude => ((b0.abs() + b1.abs()) / 2.0, (p0 + p1) / 2.0),
                _ => {
                    let turn = telemetry::geo::heading_diff(a.hdg_deg, b.hdg_deg).to_radians() / dt;
                    let v = (a.gs_kt + b.gs_kt) / 2.0 * 0.514_444;
                    let bank = (v * turn / 9.806_65).atan().to_degrees().abs();
                    let climb = (a.vs_fpm + b.vs_fpm) / 2.0 / 60.0;
                    let pitch = climb.atan2(((a.gs_kt + b.gs_kt) / 2.0 * 1.687_81).max(1.0)).to_degrees();
                    (bank, pitch)
                }
            };
            h.airborne_s += dt;
            if with_ias {
                if let (Some(alt), Some(i0), Some(i1)) = (a.alt_ft, a.ias_kt, b.ias_kt) {
                    if alt < SPEED_LIMIT_BELOW_FT {
                        h.add_speed((i0 + i1) / 2.0, dt);
                    }
                }
            }
            h.add_bank(bank, dt);
            h.add_pitch(pitch, dt);
            h.bank_max = h.bank_max.max(bank);
            h.pitch_max = h.pitch_max.max(pitch);
            h.pitch_min = h.pitch_min.min(pitch);
            if with_g {
                let g = match (a.g_load, b.g_load) {
                    (Some(x), Some(y)) if x != 0.0 && y != 0.0 => (x + y) / 2.0,
                    _ => 1.0,
                };
                h.g_s[band(&G_EDGES, g)] += dt;
                h.g_max = h.g_max.max(g);
                h.g_min = h.g_min.min(g);
            }
        }
        if h.airborne_s < MIN_AIRBORNE_S || gaps.is_empty() {
            return None;
        }
        gaps.sort_by(f64::total_cmp);
        (simplified || gaps[gaps.len() / 2] <= MAX_MEDIAN_GAP_S).then_some(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands() {
        assert_eq!(band(&G_EDGES, 1.0), 4);
        assert_eq!(band(&G_EDGES, -5.0), 0);
        assert_eq!(band(&G_EDGES, 7.0), 10);
        assert_eq!(band(&BANK_EDGES, 25.0), 0);
        assert_eq!(band(&BANK_EDGES, 50.0), 2);
        assert_eq!(band(&PITCH_EDGES, 55.0), 4);
    }

    #[test]
    fn a_brief_bump_is_smoothed_away() {
        let mut h = Handling::default();
        let mut s = Telemetry { g_load: 1.0, ..Default::default() };
        for i in 0..100 {
            s.t_ms = i * 100;
            // One 0.1 s spike of 2.8 G.
            s.g_load = if i == 50 { 2.8 } else { 1.0 };
            h.add(&s);
        }
        assert!((h.airborne_s - 9.9).abs() < 1e-6);
        assert!(h.g_max < 1.6, "{}", h.g_max);
        assert_eq!(h.g_s[4], h.airborne_s);
    }

    fn cruise(n: usize, every_s: i64) -> Vec<Sample> {
        (0..n)
            .map(|i| Sample { t_ms: i as i64 * every_s * 1000, gs_kt: 250.0, hdg_deg: 90.0, ..Default::default() })
            .collect()
    }

    #[test]
    fn a_steep_turn_shows_in_the_track() {
        // 250 kt, turning 6 degrees a second for a minute: about 53 degrees of bank.
        let mut s = cruise(200, 2);
        for (i, p) in s.iter_mut().enumerate().skip(50).take(31) {
            p.hdg_deg = 90.0 + (i - 50) as f64 * 12.0;
        }
        let h = Handling::estimate(&s).unwrap();
        assert_eq!(h.source, source::TRACK);
        assert!(h.g_s.is_empty());
        assert!((h.bank_max - 53.0).abs() < 2.0, "{}", h.bank_max);
        assert!((h.bank_s[2] - 60.0).abs() < 0.1, "{:?}", h.bank_s);
    }

    #[test]
    fn logged_attitude_is_used_as_is() {
        let mut s = cruise(100, 5);
        for p in &mut s {
            p.bank_deg = Some(-20.0);
            p.pitch_deg = Some(3.0);
        }
        let h = Handling::estimate(&s).unwrap();
        assert_eq!(h.source, source::POSITIONS);
        assert_eq!(h.bank_s[0], h.airborne_s);
        assert_eq!(h.pitch_max, 3.0);
    }

    #[test]
    fn too_little_to_go_on() {
        // A minute in the air.
        assert!(Handling::estimate(&cruise(13, 5)).is_none());
        // Positions two minutes apart.
        assert!(Handling::estimate(&cruise(30, 120)).is_none());
        // Old recordings without the field are FlightDeck's own.
        let h: Handling = serde_json::from_str(r#"{"airborne_s":10,"g_s":[],"bank_s":[],"pitch_s":[],"g_max":1,"g_min":1,"bank_max":0,"pitch_max":0,"pitch_min":0}"#).unwrap();
        assert_eq!(h.source, source::RECORDED);
        assert!(!Handling::none().is_known());
    }

    #[test]
    fn nose_up_and_nose_down_are_kept_apart() {
        let mut h = Handling::default();
        let mut s = Telemetry { g_load: 1.0, pitch_deg: 20.0, ..Default::default() };
        for i in 0..=100 {
            s.t_ms = i * 100;
            // 20 degrees up is fine; 20 degrees down is not.
            s.pitch_deg = if i <= 50 { 20.0 } else { -20.0 };
            h.add(&s);
        }
        assert!((h.pitch_up_s[0] - 10.0).abs() < 1e-6, "{:?}", h.pitch_up_s);
        assert!((h.pitch_down_s[1] - 5.0).abs() < 1e-6, "{:?}", h.pitch_down_s);
        assert_eq!((h.pitch_max, h.pitch_min), (20.0, -20.0));
        // 20 degrees: the 20-25 step either way.
        assert!((h.pitch_up_5[4] - 5.0).abs() < 1e-6 && (h.pitch_down_5[4] - 5.0).abs() < 1e-6);
    }

    #[test]
    fn an_approach_from_positions() {
        // Descending at 900 fpm from 1,800 ft to touchdown, a position every
        // 4 s; heights from the altitude and a 400 ft airport.
        let mut s: Vec<Sample> = (0..30)
            .map(|i| Sample { t_ms: i * 4000, alt_ft: Some(2200.0 - i as f64 * 60.0), vs_fpm: -900.0, ..Default::default() })
            .collect();
        s.push(Sample { t_ms: 120_000, on_ground: true, alt_ft: Some(400.0), ..Default::default() });
        let a = Handling::estimate_approach(&s, Some(400.0)).unwrap();
        assert!(a.points.iter().all(|p| p.agl_ft <= 1000.0));
        assert!(a.points.len() >= 15, "{}", a.points.len());
        // No airport elevation and no logged heights: nothing to go on.
        assert!(Handling::estimate_approach(&s, None).is_none());
        // Positions 20 s apart are too far apart.
        let sparse: Vec<Sample> = s.iter().step_by(5).copied().chain([s[30]]).collect();
        assert!(Handling::estimate_approach(&sparse, Some(400.0)).is_none());
    }

    #[test]
    fn a_sustained_steep_turn_counts() {
        let mut h = Handling::default();
        let mut s = Telemetry { g_load: 2.0, bank_deg: -62.0, ..Default::default() };
        for i in 0..=200 {
            s.t_ms = i * 100;
            h.add(&s);
        }
        assert!((h.bank_s[3] - 20.0).abs() < 1e-6);
        assert!(h.g_s[6] > 19.0);
        assert_eq!(h.bank_max, 62.0);
    }
}
