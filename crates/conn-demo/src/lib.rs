//! A tiny kinematic flight model that flies gate to gate between two airports.
//!
//! It produces the same [`SimMessage`]s as a real connector, so the tracker,
//! store and UI can be developed and tested on any PC with no simulator.
//! Sim time runs `speedup` times faster than real time, but timestamps
//! advance at sim speed, so logged durations look like a real flight.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use telemetry::{
    geo, AircraftInfo, ConnectorError, EngineType, SimConnector, SimKind, SimMessage, Telemetry,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Airport {
    pub icao: String,
    pub lat: f64,
    pub lon: f64,
    pub elev_ft: f64,
}

impl Airport {
    pub fn new(icao: &str, lat: f64, lon: f64, elev_ft: f64) -> Self {
        Airport { icao: icao.into(), lat, lon, elev_ft }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoRoute {
    pub from: Airport,
    pub to: Airport,
}

/// A few routes, including international ones so passport stamps can be tested.
pub fn sample_routes() -> Vec<DemoRoute> {
    let r = |a: Airport, b: Airport| DemoRoute { from: a, to: b };
    vec![
        r(Airport::new("KSEA", 47.4490, -122.3093, 433.0), Airport::new("KPDX", 45.5887, -122.5975, 31.0)),
        r(Airport::new("EGLL", 51.4706, -0.4619, 83.0), Airport::new("LFPG", 49.0097, 2.5479, 392.0)),
        r(Airport::new("KSFO", 37.6188, -122.3750, 13.0), Airport::new("KLAX", 33.9425, -118.4081, 128.0)),
        r(Airport::new("EDDF", 50.0333, 8.5706, 364.0), Airport::new("LOWW", 48.1103, 16.5697, 600.0)),
        r(Airport::new("CYVR", 49.1939, -123.1844, 14.0), Airport::new("KSEA", 47.4490, -122.3093, 433.0)),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Parked,
    TaxiOut,
    Roll,
    Climb,
    Cruise,
    Descent,
    Flare,
    Rollout,
    TaxiIn,
    Shutdown,
    Done,
}

/// Tunables for the demo flight.
#[derive(Debug, Clone)]
pub struct DemoOptions {
    /// Sim seconds per real second.
    pub speedup: f64,
    /// Target touchdown rate, fpm (negative).
    pub touchdown_fpm: f64,
    /// Sample rate in sim time.
    pub hz: f64,
}

impl Default for DemoOptions {
    fn default() -> Self {
        DemoOptions { speedup: 30.0, touchdown_fpm: -140.0, hz: 10.0 }
    }
}

pub struct DemoFlight {
    route: DemoRoute,
    opts: DemoOptions,
    stage: Stage,
    stage_time: f64,
    s: Telemetry,
    agl: f64,
    cruise_agl: f64,
    total_nm: f64,
    taxied_nm: f64,
    started: bool,
    last_poll: Option<Instant>,
    debt_s: f64,
}

const GLIDE_FT_PER_NM: f64 = 318.4; // 3 degree path

impl DemoFlight {
    pub fn new(route: DemoRoute, opts: DemoOptions, start_ms: i64) -> Self {
        let total_nm = geo::distance_nm(route.from.lat, route.from.lon, route.to.lat, route.to.lon);
        let cruise_agl = (total_nm * 110.0).clamp(5_000.0, 35_000.0).round();
        let hdg = geo::bearing_deg(route.from.lat, route.from.lon, route.to.lat, route.to.lon);
        let s = Telemetry {
            t_ms: start_ms,
            lat: route.from.lat,
            lon: route.from.lon,
            alt_msl_ft: route.from.elev_ft,
            hdg_true_deg: hdg,
            g_load: 1.0,
            on_ground: true,
            parking_brake: true,
            engines_running: 2,
            fuel_kg: 6000.0,
            sim_rate: 1.0,
            wind_dir_deg: (hdg + 200.0) % 360.0,
            wind_kt: 8.0,
            ..Default::default()
        };
        DemoFlight {
            route,
            opts,
            stage: Stage::Parked,
            stage_time: 0.0,
            s,
            agl: 0.0,
            cruise_agl,
            total_nm,
            taxied_nm: 0.0,
            started: false,
            last_poll: None,
            debt_s: 0.0,
        }
    }

    pub fn aircraft() -> AircraftInfo {
        AircraftInfo {
            title: "FlightDeck Demo Jet".into(),
            icao_type: "A320".into(),
            registration: "N320FD".into(),
            engine_count: 2,
            engine_type: EngineType::Jet,
        }
    }

    pub fn route(&self) -> &DemoRoute {
        &self.route
    }

    pub fn is_done(&self) -> bool {
        self.stage == Stage::Done
    }

    /// Advance the model by `dt` sim seconds and return the new sample.
    pub fn step(&mut self, dt: f64) -> Option<Telemetry> {
        if self.stage == Stage::Done {
            return None;
        }
        self.stage_time += dt;
        let to = &self.route.to;
        let dist = geo::distance_nm(self.s.lat, self.s.lon, to.lat, to.lon);
        let s = &mut self.s;
        let mut next: Option<Stage> = None;
        s.g_load = 1.0;
        s.touchdown_vs_fpm = None;

        match self.stage {
            Stage::Parked => {
                if self.stage_time > 20.0 {
                    s.parking_brake = false;
                    next = Some(Stage::TaxiOut);
                }
            }
            Stage::TaxiOut => {
                s.gs_kt = 15.0;
                if self.taxied_nm > 0.4 {
                    next = Some(Stage::Roll);
                }
            }
            Stage::Roll => {
                s.gs_kt += 3.5 * dt;
                if s.gs_kt >= 145.0 {
                    s.on_ground = false;
                    next = Some(Stage::Climb);
                }
            }
            Stage::Climb => {
                s.vs_fpm = 1800.0;
                s.pitch_deg = 8.0;
                s.gs_kt = (s.gs_kt + 1.0 * dt).min(300.0);
                if self.agl >= self.cruise_agl {
                    self.agl = self.cruise_agl;
                    next = Some(Stage::Cruise);
                } else if dist * GLIDE_FT_PER_NM < self.agl {
                    next = Some(Stage::Descent);
                }
            }
            Stage::Cruise => {
                s.vs_fpm = 0.0;
                s.pitch_deg = 2.0;
                s.gs_kt = (s.gs_kt + 1.0 * dt).min(300.0);
                if dist * GLIDE_FT_PER_NM < self.agl {
                    next = Some(Stage::Descent);
                }
            }
            Stage::Descent => {
                let target_gs = if dist < 8.0 { 140.0 } else if dist < 30.0 { 180.0 } else { 280.0 };
                s.gs_kt += (target_gs - s.gs_kt).clamp(-1.5 * dt, 1.5 * dt);
                let wanted = dist * GLIDE_FT_PER_NM;
                s.vs_fpm = (-s.gs_kt * 5.3 + (wanted - self.agl) * 0.5).clamp(-3000.0, 300.0);
                s.pitch_deg = 3.0;
                if self.agl < 40.0 {
                    next = Some(Stage::Flare);
                }
            }
            Stage::Flare => {
                s.vs_fpm = self.opts.touchdown_fpm;
                s.pitch_deg = 5.0;
                if self.agl + s.vs_fpm * dt / 60.0 <= 0.0 {
                    self.agl = 0.0;
                    s.on_ground = true;
                    s.g_load = 1.0 + self.opts.touchdown_fpm.abs() / 800.0;
                    s.touchdown_vs_fpm = Some(self.opts.touchdown_fpm);
                    s.vs_fpm = 0.0;
                    s.pitch_deg = 0.0;
                    next = Some(Stage::Rollout);
                }
            }
            Stage::Rollout => {
                s.gs_kt = (s.gs_kt - 4.0 * dt).max(12.0);
                if s.gs_kt <= 12.0 {
                    next = Some(Stage::TaxiIn);
                }
            }
            Stage::TaxiIn => {
                s.gs_kt = 12.0;
                if self.stage_time > 60.0 {
                    s.gs_kt = 0.0;
                    s.parking_brake = true;
                    next = Some(Stage::Shutdown);
                }
            }
            Stage::Shutdown => {
                if self.stage_time > 5.0 {
                    s.engines_running = 0;
                }
                if self.stage_time > 15.0 {
                    next = Some(Stage::Done);
                }
            }
            Stage::Done => {}
        }

        // Steer for the destination while moving, then integrate.
        if s.gs_kt > 0.5 && !matches!(self.stage, Stage::Rollout | Stage::TaxiIn) {
            s.hdg_true_deg = geo::bearing_deg(s.lat, s.lon, to.lat, to.lon);
        }
        let step_nm = s.gs_kt * dt / 3600.0;
        let (lat, lon) = geo::destination(s.lat, s.lon, s.hdg_true_deg, step_nm);
        s.lat = lat;
        s.lon = lon;
        if self.stage == Stage::TaxiOut {
            self.taxied_nm += step_nm;
        }
        if !s.on_ground {
            self.agl = (self.agl + s.vs_fpm * dt / 60.0).max(0.0);
        }
        let frac = 1.0 - (dist / self.total_nm).clamp(0.0, 1.0);
        let ground = self.route.from.elev_ft + (to.elev_ft - self.route.from.elev_ft) * frac;
        s.alt_agl_ft = self.agl;
        s.alt_msl_ft = ground + self.agl;
        s.ias_kt = s.gs_kt / (1.0 + s.alt_msl_ft / 1000.0 * 0.02);
        let burn = if s.on_ground { 0.15 } else { 0.7 };
        if s.engines_running > 0 {
            s.fuel_kg = (s.fuel_kg - burn * dt).max(0.0);
        }
        s.t_ms += (dt * 1000.0).round() as i64;
        let out = *s;
        if let Some(n) = next {
            self.stage = n;
            self.stage_time = 0.0;
        }
        Some(out)
    }

    /// Run the whole flight instantly, for tests and fixtures.
    pub fn run_to_end(mut self) -> Vec<Telemetry> {
        let dt = 1.0 / self.opts.hz;
        let mut v = Vec::new();
        while let Some(s) = self.step(dt) {
            v.push(s);
            assert!(v.len() < 10_000_000, "demo flight never finished");
        }
        v
    }
}

/// The demo as a live [`SimConnector`], paced against the real clock.
pub struct DemoConnector {
    flight: DemoFlight,
}

impl DemoConnector {
    pub fn new(flight: DemoFlight) -> Self {
        DemoConnector { flight }
    }
}

impl SimConnector for DemoConnector {
    fn sim(&self) -> SimKind {
        SimKind::Demo
    }

    fn poll(&mut self) -> Result<Vec<SimMessage>, ConnectorError> {
        let f = &mut self.flight;
        let mut out = Vec::new();
        if !f.started {
            f.started = true;
            out.push(SimMessage::Connected {
                sim: SimKind::Demo,
                app_name: "FlightDeck demo".into(),
                app_version: format!("{} to {}", f.route.from.icao, f.route.to.icao),
            });
            out.push(SimMessage::Aircraft(DemoFlight::aircraft()));
        }
        let now = Instant::now();
        let real = f.last_poll.map(|p| now.duration_since(p).as_secs_f64()).unwrap_or(0.0);
        f.last_poll = Some(now);
        f.debt_s += (real * f.opts.speedup).min(30.0);
        let dt = 1.0 / f.opts.hz;
        while f.debt_s >= dt {
            f.debt_s -= dt;
            match f.step(dt) {
                Some(s) => out.push(SimMessage::Telemetry(s)),
                None => {
                    out.push(SimMessage::Disconnected { reason: "demo flight finished".into() });
                    break;
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_route_finishes_at_its_destination() {
        for route in sample_routes() {
            let to = route.to.clone();
            let samples = DemoFlight::new(route, DemoOptions::default(), 0).run_to_end();
            let last = samples.last().unwrap();
            let miss = geo::distance_nm(last.lat, last.lon, to.lat, to.lon);
            assert!(miss < 4.0, "{} missed by {miss} nm", to.icao);
            assert_eq!(last.engines_running, 0);
            assert!(samples.iter().any(|s| !s.on_ground));
        }
    }
}
