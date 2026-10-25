use super::*;
use telemetry::geo;

/// Drives a tracker with a hand-scripted flight.
struct Script {
    s: Telemetry,
    tr: Tracker,
    out: Vec<TrackerOutput>,
}

impl Script {
    fn new() -> Self {
        Self::starting(true)
    }

    fn starting(on_ground: bool) -> Self {
        let s = Telemetry {
            t_ms: 1_760_000_000_000,
            lat: 47.4490,
            lon: -122.3093,
            alt_msl_ft: if on_ground { 430.0 } else { 3430.0 },
            alt_agl_ft: if on_ground { 0.0 } else { 3000.0 },
            hdg_true_deg: 180.0,
            g_load: 1.0,
            on_ground,
            parking_brake: on_ground,
            engines_running: 2,
            fuel_kg: 5000.0,
            sim_rate: 1.0,
            ..Default::default()
        };
        let aircraft = AircraftInfo { title: "Test jet".into(), icao_type: "A320".into(), ..Default::default() };
        Script { s, tr: Tracker::new(SimKind::Demo, aircraft, Config::default()), out: vec![] }
    }

    /// Run for `secs` at `hz`, letting `f` set the controls each step.
    fn run(&mut self, secs: f64, hz: f64, f: impl Fn(&mut Telemetry)) {
        let dt = 1.0 / hz;
        let steps = (secs * hz).round() as usize;
        for _ in 0..steps {
            f(&mut self.s);
            let s = &mut self.s;
            if !s.paused {
                let (lat, lon) = geo::destination(s.lat, s.lon, s.hdg_true_deg, s.gs_kt * dt / 3600.0);
                s.lat = lat;
                s.lon = lon;
                if !s.on_ground {
                    let dz = s.vs_fpm * dt / 60.0;
                    s.alt_msl_ft += dz;
                    s.alt_agl_ft = (s.alt_agl_ft + dz).max(0.0);
                }
                s.fuel_kg -= 0.5 * dt;
            }
            s.t_ms += (dt * 1000.0) as i64;
            let snap = *s;
            self.out.extend(self.tr.update(&snap));
        }
    }

    fn phases(&self) -> Vec<Phase> {
        self.out
            .iter()
            .filter_map(|o| match o {
                TrackerOutput::PhaseChanged { to, .. } => Some(*to),
                _ => None,
            })
            .collect()
    }

    fn completed(&self) -> Option<&FlightSummary> {
        self.out.iter().find_map(|o| match o {
            TrackerOutput::Completed(s) => Some(s.as_ref()),
            _ => None,
        })
    }

    // Reusable flight segments ------------------------------------------

    fn taxi_and_takeoff(&mut self) {
        self.run(10.0, 10.0, |s| s.parking_brake = true);
        self.run(60.0, 10.0, |s| {
            s.parking_brake = false;
            s.gs_kt = 15.0;
        });
        self.run(25.0, 10.0, |s| s.gs_kt = (s.gs_kt + 0.6).min(150.0));
        self.run(120.0, 10.0, |s| {
            s.on_ground = false;
            s.vs_fpm = 1500.0;
            s.pitch_deg = 8.0;
        });
    }

    fn cruise(&mut self, secs: f64) {
        self.run(secs, 10.0, |s| {
            s.vs_fpm = 0.0;
            s.gs_kt = 300.0;
        });
    }

    /// Descend until the wheels touch at `vs` fpm.
    fn approach_and_land(&mut self, vs: f64) {
        self.run(60.0, 10.0, |s| {
            s.gs_kt = 140.0;
            s.vs_fpm = if s.alt_agl_ft > 100.0 { -800.0 } else { vs };
        });
        // Fly the last bit at high rate until touchdown.
        for _ in 0..40_000 {
            if self.s.on_ground {
                break;
            }
            self.run(0.05, 20.0, |s| {
                s.vs_fpm = if s.alt_agl_ft > 100.0 { -800.0 } else { vs };
                if s.alt_agl_ft <= 0.5 {
                    s.on_ground = true;
                    s.g_load = 1.3;
                }
            });
        }
        assert!(self.s.on_ground, "never touched down");
    }

    fn rollout_taxi_park(&mut self) {
        self.run(30.0, 10.0, |s| {
            s.vs_fpm = 0.0;
            s.g_load = 1.0;
            s.gs_kt = (s.gs_kt - 4.0).max(12.0);
        });
        self.run(60.0, 10.0, |s| s.gs_kt = 12.0);
        self.run(10.0, 10.0, |s| {
            s.gs_kt = 0.0;
            s.parking_brake = true;
            s.engines_running = 0;
        });
    }
}

#[test]
fn full_flight_goes_through_every_phase() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.cruise(300.0);
    f.approach_and_land(-150.0);
    f.rollout_taxi_park();

    assert_eq!(
        f.phases(),
        vec![
            Phase::Preflight,
            Phase::TaxiOut,
            Phase::TakeoffRoll,
            Phase::Airborne,
            Phase::LandingRollout,
            Phase::TaxiIn,
            Phase::Arrived
        ]
    );
    let s = f.completed().expect("flight should complete");
    assert_eq!(s.status, FlightStatus::Complete);
    assert!(s.block_out_ms.is_some() && s.takeoff_ms.is_some() && s.block_in_ms.is_some());
    assert_eq!(s.landings.len(), 1);
    let l = &s.landings[0];
    assert!((l.vs_fpm + 150.0).abs() < 10.0, "landing rate {}", l.vs_fpm);
    assert!((l.g_peak - 1.3).abs() < 1e-9);
    assert_eq!(l.bounces, 0);
    assert!(s.distance_nm > 20.0);
    assert!(s.fuel_used_kg > 0.0);
    // Block out happens once taxi starts, not while parked.
    assert!(s.block_out_ms.unwrap() >= s.start_ms + 10_000);
    assert!(s.block_minutes().unwrap() > s.air_minutes().unwrap());
    assert!(f.tr.is_finished());
}

#[test]
fn touchdown_report_is_emitted_once() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.approach_and_land(-200.0);
    f.rollout_taxi_park();
    let n = f.out.iter().filter(|o| matches!(o, TrackerOutput::Touchdown(_))).count();
    assert_eq!(n, 1);
}

#[test]
fn short_hop_off_the_runway_is_a_bounce() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.approach_and_land(-300.0);
    f.run(1.0, 10.0, |_| {});
    f.run(2.0, 10.0, |s| s.on_ground = false);
    f.run(1.0, 10.0, |s| s.on_ground = true);
    f.rollout_taxi_park();
    let s = f.completed().unwrap();
    assert_eq!(s.landings.len(), 1);
    assert_eq!(s.landings[0].bounces, 1);
}

#[test]
fn touch_and_go_counts_two_landings() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.approach_and_land(-120.0);
    f.run(5.0, 10.0, |_| {});
    f.run(60.0, 10.0, |s| {
        s.on_ground = false;
        s.vs_fpm = 800.0;
    });
    f.approach_and_land(-90.0);
    f.rollout_taxi_park();
    let s = f.completed().unwrap();
    assert_eq!(s.landings.len(), 2);
    assert!(s.landings[0].touch_and_go);
    assert!(!s.landings[1].touch_and_go);
    assert_eq!(s.final_landing().unwrap().t_ms, s.landings[1].t_ms);
    assert_eq!(s.status, FlightStatus::Complete);
}

#[test]
fn teleport_ends_the_flight_as_partial() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.cruise(60.0);
    f.run(0.1, 10.0, |s| s.lat += 1.0); // 60 nm in a tenth of a second
    let s = f.completed().expect("teleport should end the flight");
    assert_eq!(s.status, FlightStatus::Partial);
    assert_eq!(s.end_reason, "teleport");
}

#[test]
fn pause_is_excluded_and_does_not_teleport() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.run(600.0, 1.0, |s| s.paused = true);
    f.run(1.0, 10.0, |s| s.paused = false);
    f.cruise(30.0);
    f.approach_and_land(-150.0);
    f.rollout_taxi_park();
    let s = f.completed().unwrap();
    assert_eq!(s.status, FlightStatus::Complete);
    assert!((s.paused_ms - 600_000).abs() <= 1_000, "{}", s.paused_ms);
}

#[test]
fn moving_while_paused_counts_as_teleport() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.run(5.0, 1.0, |s| s.paused = true);
    f.run(0.1, 10.0, |s| {
        s.paused = false;
        s.lon += 0.5;
    });
    assert_eq!(f.completed().unwrap().end_reason, "teleport");
}

#[test]
fn air_start_is_logged_without_block_out() {
    let mut f = Script::starting(false);
    f.run(30.0, 10.0, |s| s.gs_kt = 140.0);
    f.approach_and_land(-100.0);
    f.rollout_taxi_park();
    let s = f.completed().unwrap();
    assert!(s.started_airborne);
    assert!(s.block_out_ms.is_none() && s.takeoff_ms.is_none());
    assert_eq!(s.status, FlightStatus::Complete);
    assert!(s.total_minutes() > 1.0);
}

#[test]
fn ending_before_takeoff_saves_nothing() {
    let mut f = Script::new();
    f.run(10.0, 10.0, |_| {});
    f.run(30.0, 10.0, |s| s.gs_kt = 15.0);
    assert!(f.tr.end("sim closed").is_none());
}

#[test]
fn ending_in_cruise_saves_a_partial_flight() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.cruise(60.0);
    let s = f.tr.end("sim closed").unwrap();
    assert_eq!(s.status, FlightStatus::Partial);
    assert!(!s.track.is_empty());
}

#[test]
fn rejected_takeoff_returns_to_taxi() {
    let mut f = Script::new();
    f.run(10.0, 10.0, |_| {});
    f.run(30.0, 10.0, |s| s.gs_kt = 15.0);
    f.run(10.0, 10.0, |s| s.gs_kt = 70.0);
    f.run(10.0, 10.0, |s| s.gs_kt = 10.0);
    assert_eq!(f.phases(), vec![Phase::Preflight, Phase::TaxiOut, Phase::TakeoffRoll, Phase::TaxiOut]);
}

#[test]
fn sim_reported_touchdown_rate_wins() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.approach_and_land(-250.0);
    f.run(0.5, 20.0, |s| s.touchdown_vs_fpm = Some(-212.0));
    f.rollout_taxi_park();
    let l = &f.completed().unwrap().landings[0];
    assert_eq!(l.vs_fpm, -212.0);
    assert_eq!(l.vs_source, "sim");
}

#[test]
fn cruise_track_is_thinned_to_about_one_hertz() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    let before = f.tr.track.len();
    f.cruise(120.0); // 1,200 samples at 10 Hz
    let kept = f.tr.track.len() - before;
    assert!((115..=125).contains(&kept), "kept {kept}");
}

#[test]
fn grades() {
    assert_eq!(grade_for(-45.0), "Butter");
    assert_eq!(grade_for(-150.0), "Smooth");
    assert_eq!(grade_for(-250.0), "Firm");
    assert_eq!(grade_for(-700.0), "Very hard");
}

#[test]
fn wheel_flicker_is_not_a_bounce() {
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.approach_and_land(-150.0);
    f.run(0.2, 10.0, |s| s.on_ground = false);
    f.run(1.0, 10.0, |s| s.on_ground = true);
    f.rollout_taxi_park();
    assert_eq!(f.completed().unwrap().landings[0].bounces, 0);
}

#[test]
fn standing_still_after_landing_ends_the_flight() {
    // A sim that never reports engines off or the parking brake.
    let mut f = Script::new();
    f.taxi_and_takeoff();
    f.approach_and_land(-150.0);
    f.run(30.0, 10.0, |s| {
        s.vs_fpm = 0.0;
        s.gs_kt = (s.gs_kt - 4.0).max(12.0);
    });
    f.run(30.0, 10.0, |s| s.gs_kt = 12.0);
    f.run(70.0, 10.0, |s| {
        s.gs_kt = 0.0;
        s.engines_running = 2;
        s.parking_brake = false;
    });
    let s = f.completed().expect("should end after a minute standing still");
    assert_eq!(s.status, FlightStatus::Complete);
    assert!(s.events.iter().any(|e| e.kind == "parked_by_timeout"));
}
