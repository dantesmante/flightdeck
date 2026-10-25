use super::*;
use conn_demo::{sample_routes, DemoFlight, DemoOptions};
use telemetry::SimKind;
use tracker::{Config, Tracker, TrackerOutput};

fn fly(route_ix: usize) -> FlightSummary {
    let route = sample_routes().remove(route_ix);
    let samples = DemoFlight::new(route, DemoOptions::default(), 1_760_000_000_000).run_to_end();
    let mut tr = Tracker::new(SimKind::Demo, DemoFlight::aircraft(), Config::default());
    for s in &samples {
        for o in tr.update(s) {
            if let TrackerOutput::Completed(sum) = o {
                return *sum;
            }
        }
    }
    panic!("demo flight did not complete");
}

fn place(icao: &str, country: &str) -> Option<Place> {
    Some(Place { icao: icao.into(), name: format!("{icao} airport"), country: country.into(), region: String::new() })
}

#[test]
fn save_list_and_read_back() {
    let mut db = Store::open_in_memory().unwrap();
    let f = fly(1); // EGLL → LFPG
    let ctx = SaveContext { dep: place("EGLL", "GB"), arr: place("LFPG", "FR"), ..Default::default() };
    let id = db.save_flight(&f, &ctx).unwrap();

    let rows = db.list_flights(10, 0).unwrap();
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.id, id);
    assert_eq!(r.status, "complete");
    assert_eq!(r.dep_icao.as_deref(), Some("EGLL"));
    assert_eq!(r.arr_country.as_deref(), Some("FR"));
    assert_eq!(r.icao_type, "A320");
    assert_eq!(r.landing_rate_fpm, Some(-140.0));
    assert_eq!(r.landing_grade.as_deref(), Some("Smooth"));
    assert!(r.total_min > 20, "{}", r.total_min);

    let track = db.get_track(id).unwrap();
    assert!(track.len() > 10 && track.len() < f.track.len(), "{} of {}", track.len(), f.track.len());
    let landings = db.get_landings(id).unwrap();
    assert_eq!(landings.len(), 1);
    assert_eq!(landings[0].vs_fpm, -140.0);

    let t = db.totals().unwrap();
    assert_eq!((t.flights, t.landings, t.airports, t.countries), (1, 1, 2, 2));
}

#[test]
fn touchdown_window_keeps_full_resolution() {
    let mut db = Store::open_in_memory().unwrap();
    let f = fly(0);
    let id = db.save_flight(&f, &SaveContext::default()).unwrap();
    let td = f.landings[0].t_ms;
    let near = |pts: &[TrackPoint]| pts.iter().filter(|p| (p.t_ms - td).abs() < 10_000).count();
    assert_eq!(near(&db.get_track(id).unwrap()), near(&f.track));
}

#[test]
fn same_aircraft_is_reused_and_delete_cascades() {
    let mut db = Store::open_in_memory().unwrap();
    let a = db.save_flight(&fly(0), &SaveContext::default()).unwrap();
    let b = db.save_flight(&fly(2), &SaveContext::default()).unwrap();
    let n: i64 = db.conn.query_row("SELECT COUNT(*) FROM aircraft", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);
    assert!(db.delete_flight(a).unwrap());
    let tracks: i64 = db.conn.query_row("SELECT COUNT(*) FROM track", [], |r| r.get(0)).unwrap();
    assert_eq!(tracks, 1);
    assert!(db.get_flight(b).unwrap().is_some());
}

#[test]
fn landing_somewhere_else_is_a_diversion() {
    let mut db = Store::open_in_memory().unwrap();
    let ctx = SaveContext {
        arr: place("KPDX", "US"),
        planned_arr_icao: Some("KSEA".into()),
        ..Default::default()
    };
    let id = db.save_flight(&fly(0), &ctx).unwrap();
    assert_eq!(db.get_flight(id).unwrap().unwrap().status, "diverted");
}

#[test]
fn reopening_keeps_data() {
    let dir = std::env::temp_dir().join(format!("fd-store-{}", std::process::id()));
    let path = dir.join("flightdeck.db");
    {
        let mut db = Store::open(&path).unwrap();
        db.save_flight(&fly(0), &SaveContext::default()).unwrap();
    }
    let db = Store::open(&path).unwrap();
    assert_eq!(db.totals().unwrap().flights, 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn journal_recovers_a_crashed_flight() {
    use journal::{recover, Journal, JournalHeader};
    let dir = std::env::temp_dir().join(format!("fd-journal-{}", std::process::id()));
    let route = sample_routes().remove(0);
    let samples = DemoFlight::new(route, DemoOptions::default(), 1_760_000_000_000).run_to_end();
    {
        let mut j = Journal::create(&dir, JournalHeader { sim: SimKind::Demo, aircraft: DemoFlight::aircraft() }).unwrap();
        // "Crash" halfway through the flight.
        for s in samples.iter().take(samples.len() / 2).step_by(10) {
            j.append(s).unwrap();
        }
    }
    let r = recover(&dir).expect("journal should be found");
    assert_eq!(r.header.aircraft.icao_type, "A320");
    let mut tr = Tracker::new(r.header.sim, r.header.aircraft, Config::default());
    for s in &r.samples {
        tr.update(s);
    }
    let sum = tr.end("recovered after crash").expect("airborne flight should be saved");
    assert_eq!(sum.status, tracker::FlightStatus::Partial);
    assert!(recover(&dir).is_none(), "journal is removed after recovery");
    let _ = std::fs::remove_dir_all(dir);
}
