//! Touchdown analysis.

use serde::{Deserialize, Serialize};
use telemetry::Telemetry;

/// Everything recorded about one touchdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandingReport {
    pub t_ms: i64,
    pub lat: f64,
    pub lon: f64,
    /// Vertical speed at touchdown, negative = descending.
    pub vs_fpm: f64,
    /// "sim" when the simulator reported it, "derived" when computed here.
    pub vs_source: String,
    /// Peak G within 1.5 s of touchdown.
    pub g_peak: f64,
    pub pitch_deg: f64,
    pub bank_deg: f64,
    pub gs_kt: f64,
    pub ias_kt: f64,
    pub hdg_deg: f64,
    pub headwind_kt: f64,
    /// Positive = wind from the right.
    pub crosswind_kt: f64,
    pub bounces: u32,
    /// True when the aircraft took off again (touch-and-go).
    pub touch_and_go: bool,
}

impl LandingReport {
    /// Build the report at the first on-ground sample. `recent` holds the last
    /// airborne samples, oldest first.
    pub fn at_touchdown(s: &Telemetry, recent: &[Telemetry]) -> Self {
        let tail: Vec<f64> = recent.iter().rev().take(3).map(|r| r.vs_fpm).collect();
        let vs = if tail.is_empty() {
            s.vs_fpm
        } else {
            tail.iter().sum::<f64>() / tail.len() as f64
        };
        let rel = (s.wind_dir_deg - s.hdg_true_deg).to_radians();
        // Prefer the sim's own touchdown rate when it arrives on this sample.
        let (vs, source) = match s.touchdown_vs_fpm {
            Some(v) if v != 0.0 => (-v.abs(), "sim"),
            _ => (vs.min(0.0), "derived"),
        };
        LandingReport {
            t_ms: s.t_ms,
            lat: s.lat,
            lon: s.lon,
            vs_fpm: vs,
            vs_source: source.into(),
            g_peak: s.g_load,
            pitch_deg: s.pitch_deg,
            bank_deg: s.bank_deg,
            gs_kt: s.gs_kt,
            ias_kt: s.ias_kt,
            hdg_deg: s.hdg_true_deg,
            headwind_kt: s.wind_kt * rel.cos(),
            crosswind_kt: s.wind_kt * rel.sin(),
            bounces: 0,
            touch_and_go: false,
        }
    }

    /// A short verdict for the landing card.
    pub fn grade(&self) -> &'static str {
        grade_for(self.vs_fpm)
    }
}

pub fn grade_for(vs_fpm: f64) -> &'static str {
    match vs_fpm.abs() {
        v if v <= 60.0 => "Butter",
        v if v <= 180.0 => "Smooth",
        v if v <= 300.0 => "Firm",
        v if v <= 600.0 => "Hard",
        _ => "Very hard",
    }
}
