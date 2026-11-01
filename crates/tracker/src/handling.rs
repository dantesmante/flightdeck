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
/// Pitch band edges (nose up or down), degrees: 5 bands.
pub const PITCH_EDGES: [f64; 4] = [20.0, 30.0, 40.0, 50.0];

/// G is smoothed over about a second, so a single bump of turbulence
/// weighs far less than a sustained pull.
const G_SMOOTH_S: f64 = 1.0;
/// Longer gaps between samples (a stutter, a pause) are not counted.
const MAX_STEP_S: f64 = 2.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Handling {
    /// Time in the air, seconds, not counting pauses.
    pub airborne_s: f64,
    pub g_s: Vec<f64>,
    pub bank_s: Vec<f64>,
    pub pitch_s: Vec<f64>,
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
}

impl Default for Handling {
    fn default() -> Self {
        Handling {
            airborne_s: 0.0,
            g_s: vec![0.0; G_EDGES.len() + 1],
            bank_s: vec![0.0; BANK_EDGES.len() + 1],
            pitch_s: vec![0.0; PITCH_EDGES.len() + 1],
            g_max: 1.0,
            g_min: 1.0,
            bank_max: 0.0,
            pitch_max: 0.0,
            pitch_min: 0.0,
            g_smooth: None,
            last_t: None,
        }
    }
}

/// Which band a value falls in.
pub fn band(edges: &[f64], v: f64) -> usize {
    edges.iter().take_while(|&&e| v >= e).count()
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
        self.bank_s[band(&BANK_EDGES, s.bank_deg.abs())] += dt;
        self.pitch_s[band(&PITCH_EDGES, s.pitch_deg.abs())] += dt;
        self.g_max = self.g_max.max(g);
        self.g_min = self.g_min.min(g);
        self.bank_max = self.bank_max.max(s.bank_deg.abs());
        self.pitch_max = self.pitch_max.max(s.pitch_deg);
        self.pitch_min = self.pitch_min.min(s.pitch_deg);
    }

    /// The aircraft left the air (or the sim paused): the next sample
    /// starts a new stretch.
    pub fn gap(&mut self) {
        self.last_t = None;
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
