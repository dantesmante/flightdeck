//! FlightDeck desktop app: Tauri commands and startup.

mod engine;

use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use engine::{Command, FullStatus, SharedState};
use store::{FlightRow, Store, Totals};
use tauri::Manager;
use telemetry::TrackPoint;
use tracker::LandingReport;

struct AppState {
    db: Arc<Mutex<Store>>,
    shared: SharedState,
    tx: Mutex<Sender<Command>>,
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> FullStatus {
    let s = state.shared.lock().unwrap();
    FullStatus { status: s.status.clone(), track: s.track.clone() }
}

#[tauri::command]
fn list_flights(state: tauri::State<AppState>, limit: Option<i64>, offset: Option<i64>) -> CmdResult<Vec<FlightRow>> {
    state.db.lock().unwrap().list_flights(limit.unwrap_or(200), offset.unwrap_or(0)).map_err(err)
}

#[tauri::command]
fn get_totals(state: tauri::State<AppState>) -> CmdResult<Totals> {
    state.db.lock().unwrap().totals().map_err(err)
}

#[derive(serde::Serialize)]
struct FlightDetail {
    flight: FlightRow,
    track: Vec<TrackPoint>,
    landings: Vec<LandingReport>,
}

#[tauri::command]
fn get_flight(state: tauri::State<AppState>, id: i64) -> CmdResult<Option<FlightDetail>> {
    let db = state.db.lock().unwrap();
    let Some(flight) = db.get_flight(id).map_err(err)? else { return Ok(None) };
    Ok(Some(FlightDetail { flight, track: db.get_track(id).map_err(err)?, landings: db.get_landings(id).map_err(err)? }))
}

#[tauri::command]
fn delete_flight(state: tauri::State<AppState>, id: i64) -> CmdResult<bool> {
    state.db.lock().unwrap().delete_flight(id).map_err(err)
}

fn send(state: &tauri::State<AppState>, cmd: Command) -> CmdResult<()> {
    state.tx.lock().unwrap().send(cmd).map_err(err)
}

#[tauri::command]
fn start_demo(state: tauri::State<AppState>, route: Option<usize>, speedup: Option<f64>) -> CmdResult<()> {
    send(&state, Command::StartDemo { route, speedup: speedup.unwrap_or(30.0) })
}

#[tauri::command]
fn stop_demo(state: tauri::State<AppState>) -> CmdResult<()> {
    send(&state, Command::StopDemo)
}

#[tauri::command]
fn end_flight(state: tauri::State<AppState>) -> CmdResult<()> {
    send(&state, Command::EndFlight)
}

#[tauri::command]
fn demo_routes() -> Vec<String> {
    conn_demo::sample_routes().iter().map(|r| format!("{} to {}", r.from.icao, r.to.icao)).collect()
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            log::info!("data folder: {}", data_dir.display());
            let db = Arc::new(Mutex::new(Store::open(&data_dir.join("flightdeck.db"))?));
            let refdata = Arc::new(refdata::RefData::load());
            engine::recover_journal(&data_dir, &db, &refdata);

            let shared: SharedState = Default::default();
            let (tx, rx) = channel();
            engine::spawn(app.handle().clone(), shared.clone(), db.clone(), refdata, data_dir, rx);
            app.manage(AppState { db, shared, tx: Mutex::new(tx) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            list_flights,
            get_totals,
            get_flight,
            delete_flight,
            start_demo,
            stop_demo,
            end_flight,
            demo_routes
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlightDeck");
}
