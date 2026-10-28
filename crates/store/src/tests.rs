use super::*;
use rusqlite::Connection;
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
fn flight_plan_is_kept_with_the_flight() {
    let mut db = Store::open_in_memory().unwrap();
    let ctx = SaveContext { plan: Some(r#"{"callsign":"AUA11"}"#.into()), callsign: "AUA11".into(), ..Default::default() };
    let id = db.save_flight(&fly(1), &ctx).unwrap();
    assert_eq!(db.get_plan(id).unwrap().as_deref(), Some(r#"{"callsign":"AUA11"}"#));
    let plain = db.save_flight(&fly(2), &SaveContext::default()).unwrap();
    assert_eq!(db.get_plan(plain).unwrap(), None);
    // A big briefing is stored compressed and comes back intact.
    let big = format!(r#"{{"ofp":"{}"}}"#, "LFMN 272330Z 33010KT CAVOK 15/08 Q1020 NOSIG ".repeat(4000));
    let id = db.save_flight(&fly(3), &SaveContext { plan: Some(big.clone()), ..Default::default() }).unwrap();
    assert_eq!(db.get_plan(id).unwrap().as_deref(), Some(big.as_str()));
    let stored: i64 = db.conn.query_row("SELECT length(plan) FROM flight WHERE id = ?1", [id], |r| r.get(0)).unwrap();
    assert!(stored * 5 < big.len() as i64, "{stored} of {}", big.len());
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
    // Only the arrival country counts as visited.
    assert_eq!((t.flights, t.landings, t.airports, t.countries), (1, 1, 2, 1));
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

#[test]
fn settings_have_defaults_and_persist() {
    let db = Store::open_in_memory().unwrap();
    assert_eq!(db.settings().unwrap()["distance_unit"], "nm");
    db.set_setting("distance_unit", "km").unwrap();
    db.set_setting("distance_unit", "mi").unwrap();
    assert_eq!(db.settings().unwrap()["distance_unit"], "mi");
}

#[test]
fn pilot_profile_round_trip() {
    let db = Store::open_in_memory().unwrap();
    let mut p = db.pilot().unwrap();
    p.name = "Dante".into();
    p.home_icao = "kc81".into();
    p.photo_url = Some("data:image/jpeg;base64,AAAA".into());
    db.set_pilot(&p).unwrap();
    let back = db.pilot().unwrap();
    assert_eq!((back.name.as_str(), back.home_icao.as_str()), ("Dante", "KC81"));
    assert!(back.photo_url.is_some());
}

#[test]
fn logbook_rows_are_chronological_and_editable() {
    let mut db = Store::open_in_memory().unwrap();
    let a = db.save_flight(&fly(0), &SaveContext { dep: place("KSEA", "US"), arr: place("KPDX", "US"), ..Default::default() }).unwrap();
    let mut later = fly(2);
    for p in later.track.iter_mut() {
        p.t_ms += 86_400_000;
    }
    later.start_ms += 86_400_000;
    later.end_ms += 86_400_000;
    later.block_out_ms = later.block_out_ms.map(|t| t + 86_400_000);
    later.block_in_ms = later.block_in_ms.map(|t| t + 86_400_000);
    later.takeoff_ms = later.takeoff_ms.map(|t| t + 86_400_000);
    later.landing_ms = later.landing_ms.map(|t| t + 86_400_000);
    let b = db.save_flight(&later, &SaveContext::default()).unwrap();
    let rows = db.logbook().unwrap();
    assert_eq!(rows.iter().map(|r| r.flight_id).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(rows[0].from_icao, "KSEA");
    assert_eq!(rows[0].category_class, "MEL");
    assert!(rows[0].xc_min > 0, "KSEA to KPDX is over 50 nm");

    let patch = LogPatchAlias { approaches: Some(2), remarks: Some("ILS 10R".into()), ..Default::default() };
    assert!(db.update_log_entry(a, &patch).unwrap());
    let r = &db.logbook().unwrap()[0];
    assert_eq!((r.approaches, r.remarks.as_str()), (2, "ILS 10R"));
    assert_eq!(r.total_min, rows[0].total_min, "unpatched fields stay");
}

use book::LogPatch as LogPatchAlias;

#[test]
fn hangar_totals_and_last_place() {
    let mut db = Store::open_in_memory().unwrap();
    db.save_flight(&fly(0), &SaveContext { dep: place("KSEA", "US"), arr: place("KPDX", "US"), ..Default::default() }).unwrap();
    db.save_flight(&fly(1), &SaveContext { dep: place("EGLL", "GB"), arr: place("LFPG", "FR"), ..Default::default() }).unwrap();
    let h = db.hangar().unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].flights, 2);
    assert!(h[0].distance_nm > 300.0);
    assert!(h[0].last_icao.is_some());
    assert!(h[0].last_lat.is_some());
    assert!(!h[0].has_photo);
    assert!(db.set_aircraft_photo(h[0].id, Some("data:image/png;base64,AA")).unwrap());
    assert!(db.hangar().unwrap()[0].has_photo);
    assert!(db.aircraft_photo(h[0].id).unwrap().is_some());
}

#[test]
fn stamps_come_from_saved_flights() {
    let mut db = Store::open_in_memory().unwrap();
    db.save_flight(&fly(1), &SaveContext { dep: place("EGLL", "GB"), arr: place("LFPG", "FR"), ..Default::default() }).unwrap();
    let flights = db.stamp_flights().unwrap();
    let stamps = passport::derive_stamps(&flights);
    assert_eq!(stamps.len(), 1);
    assert_eq!(stamps[0].kind, passport::StampKind::FirstEntry);
    assert_eq!(stamps[0].country, "FR");
}

#[test]
fn clearing_sample_flights_removes_their_aircraft() {
    let mut db = Store::open_in_memory().unwrap();
    db.save_flight(&fly(0), &SaveContext::default()).unwrap();
    assert_eq!(db.delete_flights_from_sim("demo").unwrap(), 1);
    assert!(db.hangar().unwrap().is_empty());
}

#[test]
fn a_version_1_database_is_upgraded_with_night_time() {
    let dir = std::env::temp_dir().join(format!("fd-migrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("flightdeck.db");
    {
        // Build a version 1 database by hand and save one flight into it.
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!("BEGIN; {SCHEMA_V1} PRAGMA user_version = 1; COMMIT;")).unwrap();
        let mut f = fly(1);
        // Move the flight to midnight UTC so it is flown at night over Europe.
        let shift = 1_782_000_000_000 - f.start_ms;
        f.start_ms += shift;
        f.end_ms += shift;
        for t in [&mut f.block_out_ms, &mut f.block_in_ms, &mut f.takeoff_ms, &mut f.landing_ms] {
            *t = t.map(|v| v + shift);
        }
        for p in f.track.iter_mut() {
            p.t_ms += shift;
        }
        for l in f.landings.iter_mut() {
            l.t_ms += shift;
        }
        conn.execute(
            "INSERT INTO flight (sim, start_ms, end_ms, block_out, takeoff, landing, block_in, status, created_at, updated_at)
             VALUES ('demo', ?1, ?2, ?3, ?4, ?5, ?6, 'complete', 0, 0)",
            params![f.start_ms, f.end_ms, f.block_out_ms, f.takeoff_ms, f.landing_ms, f.block_in_ms],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO log_entry (flight_id, total_min, day_landings) VALUES (?1, ?2, 1)",
            params![id, f.total_minutes().round() as i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO track (flight_id, points, point_count, bbox) VALUES (?1, ?2, ?3, '[]')",
            params![id, codec::encode(&f.track).unwrap(), f.track.len() as i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO flight_event (flight_id, t, kind, data) VALUES (?1, ?2, 'landing_report', ?3)",
            params![id, f.landings[0].t_ms, serde_json::to_string(&f.landings[0]).unwrap()],
        )
        .unwrap();
    }
    let db = Store::open(&path).unwrap();
    let r = &db.logbook().unwrap()[0];
    assert!(r.night_min > 30, "night {}", r.night_min);
    assert_eq!((r.day_landings, r.night_landings), (0, 1));
    let arr_lat: Option<f64> = db.conn.query_row("SELECT arr_lat FROM flight", [], |r| r.get(0)).unwrap();
    assert!((arr_lat.unwrap() - 49.0).abs() < 0.1, "landing point near LFPG");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn routes_list_every_flight_with_end_points() {
    let mut db = Store::open_in_memory().unwrap();
    let id = db.save_flight(&fly(1), &SaveContext { dep: place("EGLL", "GB"), arr: place("LFPG", "FR"), ..Default::default() }).unwrap();
    let r = db.routes().unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].flight_id, id);
    assert_eq!(r[0].arr_icao.as_deref(), Some("LFPG"));
    assert!((r[0].dep_lat.unwrap() - 51.47).abs() < 0.1);
    assert!((r[0].arr_lon.unwrap() - 2.55).abs() < 0.2);
}

#[test]
fn every_logbook_field_can_be_edited() {
    use book::FlightPatch;
    let mut db = Store::open_in_memory().unwrap();
    let a = db.save_flight(&fly(0), &SaveContext { dep: place("KSEA", "US"), arr: place("KPDX", "US"), ..Default::default() }).unwrap();
    let b = db.save_flight(&fly(2), &SaveContext::default()).unwrap();
    let before = db.logbook().unwrap().into_iter().find(|r| r.flight_id == a).unwrap();

    // 1 Jan 2026, out 14:05Z, in 01:10Z (next day), new airports and aircraft.
    let jan1 = 1_767_225_600_000;
    let patch = FlightPatch {
        date_ms: Some(jan1),
        out_min: Some(14 * 60 + 5),
        in_min: Some(70),
        dep: place("KORD", "US"),
        arr: place("CYYZ", "CA"),
        aircraft_type: Some("B38M".into()),
        ident: Some("N12345".into()),
    };
    assert!(db.update_flight(a, &patch).unwrap());
    let r = db.logbook().unwrap().into_iter().find(|r| r.flight_id == a).unwrap();
    assert_eq!(r.date_ms / 86_400_000, jan1 / 86_400_000);
    assert_eq!(r.out_ms, Some(jan1 + (14 * 60 + 5) * 60_000));
    assert_eq!(r.in_ms, Some(jan1 + 86_400_000 + 70 * 60_000));
    assert_eq!((r.from_icao.as_str(), r.to_icao.as_str()), ("KORD", "CYYZ"));
    assert_eq!((r.aircraft_type.as_str(), r.ident.as_str()), ("B38M", "N12345"));
    assert_eq!(r.total_min, before.total_min, "times in the log columns are kept");

    // The other flight still has its own aircraft.
    let other = db.logbook().unwrap().into_iter().find(|r| r.flight_id == b).unwrap();
    assert_eq!(other.aircraft_type, "A320");
    // The edited route now earns a Canada stamp.
    let stamps = passport::derive_stamps(&db.stamp_flights().unwrap());
    assert!(stamps.iter().any(|s| s.country == "CA"));
}

#[test]
fn deleting_an_entry_removes_the_flight_everywhere() {
    let mut db = Store::open_in_memory().unwrap();
    let a = db.save_flight(&fly(1), &SaveContext { dep: place("EGLL", "GB"), arr: place("LFPG", "FR"), ..Default::default() }).unwrap();
    assert!(db.delete_flight(a).unwrap());
    assert!(db.logbook().unwrap().is_empty());
    assert!(db.routes().unwrap().is_empty());
    assert!(db.stamp_flights().unwrap().is_empty());
    assert!(db.hangar().unwrap().is_empty());
}
