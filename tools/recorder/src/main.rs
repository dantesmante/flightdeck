//! fd-recorder: capture real sim telemetry once, then test against it forever.
//!
//!   fd-recorder record <file.jsonl>      record from MSFS until it closes or Ctrl+C
//!   fd-recorder demo <file.jsonl> [n]    write demo route n (0-4) instantly
//!   fd-recorder replay <file.jsonl>      run a recording through the tracker
//!
//! Recordings are JSON lines of `SimMessage`, the same messages the app sees.
//! Put good ones in fixtures/ so the tracker can be tested without a sim.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::time::{Duration, Instant};

use conn_demo::{sample_routes, DemoFlight, DemoOptions};
use telemetry::{AircraftInfo, SimConnector, SimKind, SimMessage};
use tracker::{Config, FlightSummary, Tracker, TrackerOutput};

fn usage() -> ! {
    eprintln!("usage:\n  fd-recorder record <file.jsonl>\n  fd-recorder demo <file.jsonl> [route 0-4]\n  fd-recorder replay <file.jsonl>");
    std::process::exit(2)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64
}

fn writer(path: &str) -> BufWriter<File> {
    BufWriter::new(File::create(path).unwrap_or_else(|e| {
        eprintln!("cannot create {path}: {e}");
        std::process::exit(1)
    }))
}

fn write_msg(w: &mut impl Write, m: &SimMessage) {
    serde_json::to_writer(&mut *w, m).unwrap();
    w.write_all(b"\n").unwrap();
}

fn record(path: &str) {
    let mut conn = match conn_simconnect::SimConnectConnector::connect(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("could not connect to MSFS: {e}");
            std::process::exit(1)
        }
    };
    let mut w = writer(path);
    let (mut n, start) = (0u64, Instant::now());
    let mut last_report = Instant::now();
    println!("Recording to {path}. Fly, then close MSFS or press Ctrl+C.");
    loop {
        match conn.poll() {
            Ok(msgs) => {
                for m in &msgs {
                    write_msg(&mut w, m);
                    n += 1;
                    if let SimMessage::Disconnected { reason } = m {
                        w.flush().unwrap();
                        println!("\n{reason}. Saved {n} messages.");
                        return;
                    }
                }
            }
            Err(e) => {
                eprintln!("connector error: {e}");
                break;
            }
        }
        if last_report.elapsed() > Duration::from_secs(5) {
            w.flush().unwrap();
            print!("\r{n} messages, {} s", start.elapsed().as_secs());
            std::io::stdout().flush().unwrap();
            last_report = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    w.flush().unwrap();
}

fn demo(path: &str, ix: usize) {
    let routes = sample_routes();
    let route = routes.get(ix).cloned().unwrap_or_else(|| usage());
    println!("Demo flight {} to {}", route.from.icao, route.to.icao);
    let mut w = writer(path);
    write_msg(&mut w, &SimMessage::Connected { sim: SimKind::Demo, app_name: "demo".into(), app_version: "0".into() });
    write_msg(&mut w, &SimMessage::Aircraft(DemoFlight::aircraft()));
    for s in DemoFlight::new(route, DemoOptions::default(), now_ms()).run_to_end() {
        write_msg(&mut w, &SimMessage::Telemetry(s));
    }
    write_msg(&mut w, &SimMessage::Disconnected { reason: "end of demo".into() });
    w.flush().unwrap();
}

fn fmt_time(ms: Option<i64>, base: i64) -> String {
    ms.map(|t| {
        let s = (t - base) / 1000;
        format!("+{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    })
    .unwrap_or_else(|| "-".into())
}

fn print_summary(f: &FlightSummary, refdata: &refdata::RefData) {
    let heli = f.aircraft.engine_type == telemetry::EngineType::Helo;
    let at = |lat, lon| refdata.airport_at(lat, lon, heli).map(|a| format!("{} ({})", a.ident, a.name)).unwrap_or("?".into());
    println!("\n== Flight summary ==");
    println!("status        {:?} ({})", f.status, f.end_reason);
    println!("aircraft      {} [{}] {}", f.aircraft.title, f.aircraft.icao_type, f.aircraft.registration);
    println!("from          {}", at(f.dep_lat, f.dep_lon));
    println!("to            {}", at(f.arr_lat, f.arr_lon));
    println!("block out     {}", fmt_time(f.block_out_ms, f.start_ms));
    println!("takeoff       {}", fmt_time(f.takeoff_ms, f.start_ms));
    println!("landing       {}", fmt_time(f.landing_ms, f.start_ms));
    println!("block in      {}", fmt_time(f.block_in_ms, f.start_ms));
    println!("total         {:.0} min (paused {:.0} min)", f.total_minutes(), f.paused_ms as f64 / 60_000.0);
    println!("distance      {:.1} nm, max alt {:.0} ft, fuel {:.0} kg", f.distance_nm, f.max_alt_ft, f.fuel_used_kg);
    for (i, l) in f.landings.iter().enumerate() {
        println!(
            "landing {}     {:.0} fpm ({}, {}) {:.2} G, {} bounce(s){}",
            i + 1,
            l.vs_fpm,
            l.grade(),
            l.vs_source,
            l.g_peak,
            l.bounces,
            if l.touch_and_go { ", touch-and-go" } else { "" }
        );
    }
    println!("track points  {}", f.track.len());
}

fn replay(path: &str) {
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("cannot open {path}: {e}");
        std::process::exit(1)
    });
    let refdata = refdata::RefData::load();
    let mut sim = SimKind::Demo;
    let mut aircraft = AircraftInfo::default();
    let mut tracker: Option<Tracker> = None;
    let mut saved = 0;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<SimMessage>(&line) else { continue };
        match msg {
            SimMessage::Connected { sim: s, app_name, app_version } => {
                sim = s;
                println!("{} ({app_name} {app_version})", s.label());
            }
            SimMessage::Aircraft(a) => {
                if let Some(t) = tracker.as_mut() {
                    t.set_aircraft(a.clone());
                }
                aircraft = a;
            }
            SimMessage::Telemetry(s) => {
                let t = tracker.get_or_insert_with(|| Tracker::new(sim, aircraft.clone(), Config::default()));
                for o in t.update(&s) {
                    match o {
                        TrackerOutput::PhaseChanged { to, t_ms, .. } => println!("{t_ms}  {}", to.label()),
                        TrackerOutput::Touchdown(l) => println!("touchdown {:.0} fpm, {:.2} G", l.vs_fpm, l.g_peak),
                        TrackerOutput::Completed(f) => {
                            print_summary(&f, &refdata);
                            saved += 1;
                        }
                        TrackerOutput::TrackPoint(_) => {}
                    }
                }
                if tracker.as_ref().is_some_and(Tracker::is_finished) {
                    tracker = None;
                }
            }
            SimMessage::Disconnected { reason } => {
                if let Some(f) = tracker.take().and_then(|mut t| t.end(&reason)) {
                    print_summary(&f, &refdata);
                    saved += 1;
                }
            }
        }
    }
    if let Some(f) = tracker.take().and_then(|mut t| t.end("end of recording")) {
        print_summary(&f, &refdata);
        saved += 1;
    }
    println!("\n{saved} flight(s) in recording");
}

fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["record", path] => record(path),
        ["demo", path] => demo(path, 0),
        ["demo", path, n] => demo(path, n.parse().unwrap_or_else(|_| usage())),
        ["replay", path] => replay(path),
        _ => usage(),
    }
}
