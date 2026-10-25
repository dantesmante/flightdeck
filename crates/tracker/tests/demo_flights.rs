//! Fly every built-in demo route through the tracker, gate to gate.

use conn_demo::{sample_routes, DemoFlight, DemoOptions};
use telemetry::{geo, SimKind};
use tracker::{Config, FlightStatus, Phase, Tracker, TrackerOutput};

#[test]
fn demo_routes_log_complete_flights() {
    for route in sample_routes() {
        let (from, to) = (route.from.clone(), route.to.clone());
        let opts = DemoOptions { touchdown_fpm: -165.0, ..Default::default() };
        let samples = DemoFlight::new(route, opts, 1_760_000_000_000).run_to_end();

        let mut tr = Tracker::new(SimKind::Demo, DemoFlight::aircraft(), Config::default());
        let mut summary = None;
        let mut phases = vec![];
        for s in &samples {
            for o in tr.update(s) {
                match o {
                    TrackerOutput::PhaseChanged { to, .. } => phases.push(to),
                    TrackerOutput::Completed(sum) => summary = Some(*sum),
                    _ => {}
                }
            }
        }
        let s = summary.unwrap_or_else(|| panic!("{} → {} did not complete; phases {phases:?}", from.icao, to.icao));
        assert_eq!(s.status, FlightStatus::Complete, "{}", from.icao);
        assert_eq!(phases.last(), Some(&Phase::Arrived));
        assert_eq!(s.landings.len(), 1);
        // The sim-reported rate wins over the derived one.
        assert_eq!(s.landings[0].vs_fpm, -165.0);
        assert_eq!(s.landings[0].vs_source, "sim");
        assert!(geo::distance_nm(s.dep_lat, s.dep_lon, from.lat, from.lon) < 1.0);
        assert!(geo::distance_nm(s.arr_lat, s.arr_lon, to.lat, to.lon) < 4.0);
        let direct = geo::distance_nm(from.lat, from.lon, to.lat, to.lon);
        assert!(s.distance_nm >= direct * 0.97 && s.distance_nm < direct * 1.15 + 3.0);
        // Track is thinned well below the raw sample count.
        assert!(s.track.len() * 4 < samples.len(), "{} of {}", s.track.len(), samples.len());
    }
}
