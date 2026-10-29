//! Flight tracking engine.
//!
//! Feed it [`Telemetry`] samples with [`Tracker::update`]. It works out the
//! flight phase, records a thinned track, analyses every touchdown and, when
//! the aircraft is parked again, hands back a complete [`FlightSummary`].
//!
//! Phases: Preflight → TaxiOut → TakeoffRoll → Airborne → LandingRollout →
//! TaxiIn → Arrived. Every transition must hold for a short debounce window so
//! a bounce or a gust never splits a flight in two.

mod landing;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use telemetry::{geo, AircraftInfo, SimKind, Telemetry, TrackPoint};

pub use landing::{grade_for, LandingReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Preflight,
    TaxiOut,
    TakeoffRoll,
    Airborne,
    LandingRollout,
    TaxiIn,
    Arrived,
}

impl Phase {
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Preflight => "Preflight",
            Phase::TaxiOut => "Taxi out",
            Phase::TakeoffRoll => "Takeoff roll",
            Phase::Airborne => "Airborne",
            Phase::LandingRollout => "Landing rollout",
            Phase::TaxiIn => "Taxi in",
            Phase::Arrived => "Arrived",
        }
    }
}

/// Thresholds. Starting values, to be tuned against recorded flights.
#[derive(Debug, Clone)]
pub struct Config {
    pub taxi_gs_kt: f64,
    pub takeoff_gs_kt: f64,
    pub rejected_gs_kt: f64,
    pub taxi_in_gs_kt: f64,
    pub park_gs_kt: f64,
    /// After landing, standing still this long counts as parked even if the
    /// sim never reports engines off or the parking brake set.
    pub park_fallback_ms: i64,
    /// How long a ground condition must hold before the phase changes.
    pub debounce_ms: i64,
    /// How long the aircraft must be off the ground to count as airborne.
    pub airborne_ms: i64,
    /// Off the ground for less than this after a touchdown = a bounce.
    pub bounce_window_ms: i64,
    /// Shorter lifts than this are wheel-contact flicker, not bounces.
    pub bounce_min_ms: i64,
    /// A jump of more than `teleport_min_nm` faster than this = teleport.
    pub teleport_nm_per_s: f64,
    pub teleport_min_nm: f64,
    pub track_interval_ms: i64,
    pub track_hdg_change_deg: f64,
    pub track_alt_change_ft: f64,
    /// Below this height on descent every sample is kept.
    pub hires_agl_ft: f64,
    pub hires_after_touchdown_ms: i64,
    /// Peak G is searched for this long after touchdown.
    pub touchdown_window_ms: i64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            taxi_gs_kt: 3.0,
            takeoff_gs_kt: 40.0,
            rejected_gs_kt: 25.0,
            taxi_in_gs_kt: 30.0,
            park_gs_kt: 1.0,
            park_fallback_ms: 60_000,
            debounce_ms: 3000,
            airborne_ms: 2000,
            bounce_window_ms: 10_000,
            bounce_min_ms: 500,
            teleport_nm_per_s: 5.0,
            teleport_min_nm: 5.0,
            track_interval_ms: 1000,
            track_hdg_change_deg: 3.0,
            track_alt_change_ft: 100.0,
            hires_agl_ft: 100.0,
            hires_after_touchdown_ms: 30_000,
            touchdown_window_ms: 1500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightStatus {
    /// Flown from block-out (or air start) to parked.
    Complete,
    /// Ended early: sim closed, crash, teleport or ended by hand.
    Partial,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightEvent {
    pub t_ms: i64,
    pub kind: String,
    pub data: serde_json::Value,
}

/// Everything known about a finished flight. Saved by the store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightSummary {
    pub sim: SimKind,
    pub aircraft: AircraftInfo,
    pub status: FlightStatus,
    pub end_reason: String,
    pub started_airborne: bool,
    pub start_ms: i64,
    pub end_ms: i64,
    pub block_out_ms: Option<i64>,
    pub takeoff_ms: Option<i64>,
    pub landing_ms: Option<i64>,
    pub block_in_ms: Option<i64>,
    pub dep_lat: f64,
    pub dep_lon: f64,
    pub arr_lat: f64,
    pub arr_lon: f64,
    pub distance_nm: f64,
    pub max_alt_ft: f64,
    pub fuel_used_kg: f64,
    pub paused_ms: i64,
    pub max_sim_rate: f64,
    pub landings: Vec<LandingReport>,
    pub track: Vec<TrackPoint>,
    pub events: Vec<FlightEvent>,
}

impl FlightSummary {
    /// The landing that ended the flight (the last full stop), if any.
    pub fn final_landing(&self) -> Option<&LandingReport> {
        self.landings.iter().rev().find(|l| !l.touch_and_go).or(self.landings.last())
    }

    /// Block time in minutes, excluding time paused.
    pub fn block_minutes(&self) -> Option<f64> {
        let (a, b) = (self.block_out_ms?, self.block_in_ms?);
        Some(((b - a - self.paused_ms).max(0)) as f64 / 60_000.0)
    }

    /// Airborne time in minutes (takeoff to final landing).
    pub fn air_minutes(&self) -> Option<f64> {
        let (a, b) = (self.takeoff_ms?, self.landing_ms?);
        Some(((b - a).max(0)) as f64 / 60_000.0)
    }

    /// Best available duration for the logbook: block time, else start to end.
    pub fn total_minutes(&self) -> f64 {
        self.block_minutes()
            .unwrap_or_else(|| ((self.end_ms - self.start_ms - self.paused_ms).max(0)) as f64 / 60_000.0)
    }
}

/// What the tracker reports after each sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrackerOutput {
    PhaseChanged { from: Option<Phase>, to: Phase, t_ms: i64 },
    /// Sent once the touchdown window has closed and peak G is known.
    Touchdown(LandingReport),
    TrackPoint(TrackPoint),
    Completed(Box<FlightSummary>),
}

/// A cut-down view of the flight in progress, for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFlight {
    pub phase: Phase,
    pub start_ms: i64,
    pub block_out_ms: Option<i64>,
    pub takeoff_ms: Option<i64>,
    pub distance_nm: f64,
    pub max_alt_ft: f64,
    pub fuel_used_kg: f64,
    pub dep_lat: f64,
    pub dep_lon: f64,
    pub last_landing: Option<LandingReport>,
    pub track_points: usize,
}

/// Waits for a condition to hold continuously, then reports when it began.
#[derive(Debug, Default, Clone)]
struct Hold(Option<i64>);

impl Hold {
    fn check(&mut self, cond: bool, t: i64, need_ms: i64) -> Option<i64> {
        if !cond {
            self.0 = None;
            return None;
        }
        let since = *self.0.get_or_insert(t);
        (t - since >= need_ms).then_some(since)
    }
}

pub struct Tracker {
    sim: SimKind,
    aircraft: AircraftInfo,
    cfg: Config,
    phase: Option<Phase>,
    started_airborne: bool,
    start_ms: i64,
    block_out: Option<i64>,
    takeoff: Option<i64>,
    landing: Option<i64>,
    block_in: Option<i64>,
    dep: (f64, f64),
    last: Option<Telemetry>,
    recent: VecDeque<Telemetry>,
    landings: Vec<LandingReport>,
    /// Index of the landing still inside its touchdown window.
    refining: Option<usize>,
    off_ground_since: Option<i64>,
    holds: Holds,
    distance_nm: f64,
    max_alt_ft: f64,
    fuel_start: Option<f64>,
    fuel_last: f64,
    paused_ms: i64,
    pause_start: Option<i64>,
    just_resumed: bool,
    max_sim_rate: f64,
    track: Vec<TrackPoint>,
    events: Vec<FlightEvent>,
    finished: bool,
}

#[derive(Default)]
struct Holds {
    taxi: Hold,
    roll: Hold,
    air: Hold,
    reject: Hold,
    slow: Hold,
    park: Hold,
    stopped: Hold,
}

impl Holds {
    fn reset(&mut self) {
        *self = Holds::default();
    }
}

impl Tracker {
    pub fn new(sim: SimKind, aircraft: AircraftInfo, cfg: Config) -> Self {
        Tracker {
            sim,
            aircraft,
            cfg,
            phase: None,
            started_airborne: false,
            start_ms: 0,
            block_out: None,
            takeoff: None,
            landing: None,
            block_in: None,
            dep: (0.0, 0.0),
            last: None,
            recent: VecDeque::with_capacity(128),
            landings: Vec::new(),
            refining: None,
            off_ground_since: None,
            holds: Holds::default(),
            distance_nm: 0.0,
            max_alt_ft: 0.0,
            fuel_start: None,
            fuel_last: 0.0,
            paused_ms: 0,
            pause_start: None,
            just_resumed: false,
            max_sim_rate: 1.0,
            track: Vec::new(),
            events: Vec::new(),
            finished: false,
        }
    }

    pub fn phase(&self) -> Option<Phase> {
        self.phase
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Replace the aircraft info (connectors sometimes learn it late).
    pub fn set_aircraft(&mut self, aircraft: AircraftInfo) {
        self.aircraft = aircraft;
    }

    pub fn aircraft(&self) -> &AircraftInfo {
        &self.aircraft
    }

    /// Has the flight got far enough to be worth saving?
    /// Worth saving: it took off, or it started in the air and really went
    /// somewhere. (In the MSFS menus the sim can report an aircraft hanging
    /// in the air that never moves: that is not a flight.)
    pub fn has_flown(&self) -> bool {
        self.takeoff.is_some() || (self.started_airborne && self.distance_nm >= 1.0)
    }

    pub fn live(&self) -> Option<LiveFlight> {
        Some(LiveFlight {
            phase: self.phase?,
            start_ms: self.start_ms,
            block_out_ms: self.block_out,
            takeoff_ms: self.takeoff,
            distance_nm: self.distance_nm,
            max_alt_ft: self.max_alt_ft,
            fuel_used_kg: self.fuel_used(),
            dep_lat: self.dep.0,
            dep_lon: self.dep.1,
            last_landing: self.landings.last().cloned(),
            track_points: self.track.len(),
        })
    }

    fn fuel_used(&self) -> f64 {
        self.fuel_start.map(|f| (f - self.fuel_last).max(0.0)).unwrap_or(0.0)
    }

    fn event(&mut self, t_ms: i64, kind: &str, data: serde_json::Value) {
        self.events.push(FlightEvent { t_ms, kind: kind.into(), data });
    }

    fn set_phase(&mut self, to: Phase, t_ms: i64, out: &mut Vec<TrackerOutput>) {
        let from = self.phase;
        if from == Some(to) {
            return;
        }
        self.phase = Some(to);
        self.holds.reset();
        self.event(t_ms, "phase", serde_json::json!({ "from": from, "to": to }));
        out.push(TrackerOutput::PhaseChanged { from, to, t_ms });
    }

    /// Process one sample.
    pub fn update(&mut self, s: &Telemetry) -> Vec<TrackerOutput> {
        let mut out = Vec::new();
        if self.finished {
            return out;
        }

        // Pause: stop the clocks and record nothing.
        if s.paused {
            if self.pause_start.is_none() && self.phase.is_some() {
                self.pause_start = Some(s.t_ms);
                self.event(s.t_ms, "paused", serde_json::Value::Null);
            }
            return out;
        }
        if let Some(p) = self.pause_start.take() {
            self.paused_ms += (s.t_ms - p).max(0);
            self.just_resumed = true;
            self.event(s.t_ms, "resumed", serde_json::Value::Null);
        }

        // First sample: decide where we are starting from.
        let Some(last) = self.last else {
            self.start_ms = s.t_ms;
            self.dep = (s.lat, s.lon);
            self.fuel_start = Some(s.fuel_kg);
            self.fuel_last = s.fuel_kg;
            self.max_alt_ft = s.alt_msl_ft;
            if s.on_ground {
                self.set_phase(Phase::Preflight, s.t_ms, &mut out);
            } else {
                self.started_airborne = true;
                self.set_phase(Phase::Airborne, s.t_ms, &mut out);
            }
            self.keep_track_point(s, &mut out);
            self.last = Some(*s);
            return out;
        };

        // Teleport / slew: an impossible jump ends the flight as partial.
        let jump = geo::distance_nm(last.lat, last.lon, s.lat, s.lon);
        let dt_s = ((s.t_ms - last.t_ms) as f64 / 1000.0).max(0.05);
        let resumed = std::mem::take(&mut self.just_resumed);
        if jump > self.cfg.teleport_min_nm && (resumed || jump / dt_s > self.cfg.teleport_nm_per_s) {
            self.event(s.t_ms, "teleport", serde_json::json!({ "jump_nm": jump }));
            if let Some(summary) = self.finish(FlightStatus::Partial, "teleport") {
                out.push(TrackerOutput::Completed(Box::new(summary)));
            }
            return out;
        }

        self.distance_nm += jump;
        self.max_alt_ft = self.max_alt_ft.max(s.alt_msl_ft);
        self.fuel_last = s.fuel_kg;
        if s.sim_rate > self.max_sim_rate + 0.01 {
            self.event(s.t_ms, "sim_rate", serde_json::json!({ "rate": s.sim_rate }));
            self.max_sim_rate = s.sim_rate;
        }

        self.step_phase(s, &mut out);
        self.keep_track_point(s, &mut out);
        self.last = Some(*s);

        if self.phase == Some(Phase::Arrived) {
            if let Some(summary) = self.finish(FlightStatus::Complete, "arrived") {
                out.push(TrackerOutput::Completed(Box::new(summary)));
            }
        }
        out
    }

    fn step_phase(&mut self, s: &Telemetry, out: &mut Vec<TrackerOutput>) {
        let c = self.cfg.clone();
        let t = s.t_ms;
        let Some(phase) = self.phase else { return };
        match phase {
            Phase::Preflight => {
                if let Some(t0) = self.holds.taxi.check(s.on_ground && s.gs_kt > c.taxi_gs_kt, t, c.debounce_ms) {
                    self.block_out = Some(t0);
                    self.dep = (s.lat, s.lon);
                    self.fuel_start = Some(s.fuel_kg);
                    self.set_phase(Phase::TaxiOut, t0, out);
                } else if let Some(t0) = self.holds.air.check(!s.on_ground, t, c.airborne_ms) {
                    // Vertical lift-off without taxi (helicopters, slewed starts).
                    self.block_out.get_or_insert(t0);
                    self.takeoff = Some(t0);
                    self.set_phase(Phase::Airborne, t0, out);
                }
            }
            Phase::TaxiOut => {
                if let Some(t0) = self.holds.roll.check(s.on_ground && s.gs_kt > c.takeoff_gs_kt, t, 1000) {
                    self.set_phase(Phase::TakeoffRoll, t0, out);
                } else if let Some(t0) = self.holds.air.check(!s.on_ground, t, c.airborne_ms) {
                    self.takeoff = Some(t0);
                    self.set_phase(Phase::Airborne, t0, out);
                }
            }
            Phase::TakeoffRoll => {
                if let Some(t0) = self.holds.air.check(!s.on_ground, t, c.airborne_ms) {
                    self.takeoff = Some(t0);
                    self.set_phase(Phase::Airborne, t0, out);
                } else if let Some(t0) =
                    self.holds.reject.check(s.on_ground && s.gs_kt < c.rejected_gs_kt, t, c.debounce_ms)
                {
                    self.event(t0, "rejected_takeoff", serde_json::Value::Null);
                    self.set_phase(Phase::TaxiOut, t0, out);
                }
            }
            Phase::Airborne => {
                if s.on_ground {
                    let report = LandingReport::at_touchdown(s, self.recent.make_contiguous());
                    self.event(t, "touchdown", serde_json::json!({ "vs_fpm": report.vs_fpm }));
                    self.landings.push(report);
                    self.refining = Some(self.landings.len() - 1);
                    self.landing = Some(t);
                    self.off_ground_since = None;
                    self.recent.clear();
                    self.set_phase(Phase::LandingRollout, t, out);
                } else {
                    // Keep ~2 s of airborne samples for the touchdown VS average.
                    self.recent.push_back(*s);
                    while self.recent.front().is_some_and(|f| t - f.t_ms > 2000) {
                        self.recent.pop_front();
                    }
                }
            }
            Phase::LandingRollout => {
                self.refine_touchdown(s, out);
                if !s.on_ground {
                    let since = *self.off_ground_since.get_or_insert(t);
                    if t - since > c.bounce_window_ms {
                        // Took off again: touch-and-go.
                        if let Some(l) = self.landings.last_mut() {
                            l.touch_and_go = true;
                        }
                        self.flush_touchdown(out);
                        self.event(since, "touch_and_go", serde_json::Value::Null);
                        self.off_ground_since = None;
                        self.set_phase(Phase::Airborne, since, out);
                    }
                } else {
                    if let Some(since) = self.off_ground_since.take() {
                        if t - since >= c.bounce_min_ms {
                            if let Some(l) = self.landings.last_mut() {
                                l.bounces += 1;
                            }
                            self.event(t, "bounce", serde_json::Value::Null);
                        }
                    }
                    if let Some(t0) = self.holds.slow.check(s.gs_kt < c.taxi_in_gs_kt, t, c.debounce_ms) {
                        self.flush_touchdown(out);
                        self.set_phase(Phase::TaxiIn, t0, out);
                    }
                }
            }
            Phase::TaxiIn => {
                let stopped = s.on_ground && s.gs_kt < c.park_gs_kt;
                let parked = stopped && (s.engines_running == 0 || s.parking_brake);
                if let Some(t0) = self.holds.park.check(parked, t, c.debounce_ms) {
                    self.block_in = Some(t0);
                    self.set_phase(Phase::Arrived, t0, out);
                } else if let Some(t0) = self.holds.stopped.check(stopped, t, c.park_fallback_ms) {
                    self.event(t, "parked_by_timeout", serde_json::Value::Null);
                    self.block_in = Some(t0);
                    self.set_phase(Phase::Arrived, t0, out);
                }
            }
            Phase::Arrived => {}
        }
    }

    /// While inside the touchdown window: track peak G and pick up the sim's
    /// own touchdown rate when it has one.
    fn refine_touchdown(&mut self, s: &Telemetry, out: &mut Vec<TrackerOutput>) {
        let Some(i) = self.refining else { return };
        let window = self.cfg.touchdown_window_ms;
        let l = &mut self.landings[i];
        if s.t_ms - l.t_ms <= window {
            l.g_peak = l.g_peak.max(s.g_load);
            if let Some(v) = s.touchdown_vs_fpm {
                if v != 0.0 && l.vs_source != "sim" {
                    l.vs_fpm = -v.abs();
                    l.vs_source = "sim".into();
                }
            }
        } else {
            self.flush_touchdown(out);
        }
    }

    fn flush_touchdown(&mut self, out: &mut Vec<TrackerOutput>) {
        if let Some(i) = self.refining.take() {
            out.push(TrackerOutput::Touchdown(self.landings[i].clone()));
        }
    }

    fn keep_track_point(&mut self, s: &Telemetry, out: &mut Vec<TrackerOutput>) {
        let c = &self.cfg;
        let hires = (self.phase == Some(Phase::Airborne) && s.alt_agl_ft < c.hires_agl_ft && s.vs_fpm < 0.0)
            || (self.phase == Some(Phase::LandingRollout)
                && self.landing.is_some_and(|l| s.t_ms - l < c.hires_after_touchdown_ms));
        let keep = match self.track.last() {
            None => true,
            Some(p) => {
                hires
                    || s.t_ms - p.t_ms >= c.track_interval_ms
                    || geo::heading_diff(p.hdg_deg as f64, s.hdg_true_deg).abs() > c.track_hdg_change_deg
                    || (p.alt_ft as f64 - s.alt_msl_ft).abs() > c.track_alt_change_ft
                    || p.on_ground != s.on_ground
            }
        };
        if keep {
            let p = TrackPoint::from_telemetry(s);
            self.track.push(p);
            out.push(TrackerOutput::TrackPoint(p));
        }
    }

    /// End the flight now (sim closed, user pressed End Flight).
    /// Returns `None` if the aircraft never left the ground.
    pub fn end(&mut self, reason: &str) -> Option<FlightSummary> {
        let status = match self.phase {
            Some(Phase::TaxiIn) | Some(Phase::Arrived) => FlightStatus::Complete,
            _ => FlightStatus::Partial,
        };
        if self.phase == Some(Phase::TaxiIn) {
            self.block_in = self.last.map(|l| l.t_ms);
        }
        self.finish(status, reason)
    }

    fn finish(&mut self, status: FlightStatus, reason: &str) -> Option<FlightSummary> {
        if self.finished {
            return None;
        }
        self.finished = true;
        if let Some(p) = self.pause_start.take() {
            if let Some(l) = self.last {
                self.paused_ms += (l.t_ms - p).max(0);
            }
        }
        if !self.has_flown() {
            return None;
        }
        let last = self.last?;
        let (arr_lat, arr_lon) = self
            .landings
            .iter()
            .rev()
            .find(|l| !l.touch_and_go)
            .map(|l| (l.lat, l.lon))
            .unwrap_or((last.lat, last.lon));
        Some(FlightSummary {
            sim: self.sim,
            aircraft: self.aircraft.clone(),
            status,
            end_reason: reason.into(),
            started_airborne: self.started_airborne,
            start_ms: self.start_ms,
            end_ms: last.t_ms,
            block_out_ms: self.block_out,
            takeoff_ms: self.takeoff,
            landing_ms: self.landing,
            block_in_ms: self.block_in,
            dep_lat: self.dep.0,
            dep_lon: self.dep.1,
            arr_lat,
            arr_lon,
            distance_nm: self.distance_nm,
            max_alt_ft: self.max_alt_ft,
            fuel_used_kg: self.fuel_used(),
            paused_ms: self.paused_ms,
            max_sim_rate: self.max_sim_rate,
            landings: self.landings.clone(),
            track: std::mem::take(&mut self.track),
            events: std::mem::take(&mut self.events),
        })
    }
}

#[cfg(test)]
mod tests;
