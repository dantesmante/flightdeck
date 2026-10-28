//! FlightDeck desktop app: Tauri commands and startup.

mod engine;
mod samples;
mod simbrief;
mod volanta;
mod update;

use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use engine::{Command, FullStatus, SharedState};
use std::collections::HashMap;

use store::book::{FlightPatch, HangarRow, LogPatch, LogRow, Pilot, RouteRow};
use store::passport::{self, CountryVisit, Stamp};
use store::{FlightRow, Store, Totals};
use tauri::{Emitter, Manager};
use telemetry::TrackPoint;
use tracker::LandingReport;

struct AppState {
    db: Arc<Mutex<Store>>,
    refdata: Arc<refdata::RefData>,
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
    /// The flight plan flown, if one was loaded.
    plan: Option<simbrief::FlightPlan>,
}

#[tauri::command]
fn get_flight(state: tauri::State<AppState>, id: i64) -> CmdResult<Option<FlightDetail>> {
    let db = state.db.lock().unwrap();
    let Some(flight) = db.get_flight(id).map_err(err)? else { return Ok(None) };
    let plan = db.get_plan(id).map_err(err)?.and_then(|j| serde_json::from_str(&j).ok());
    Ok(Some(FlightDetail { flight, track: db.get_track(id).map_err(err)?, landings: db.get_landings(id).map_err(err)?, plan }))
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

/// The pilot's latest SimBrief flight plan, using the Pilot ID from Settings.
#[tauri::command]
async fn simbrief_latest(state: tauri::State<'_, AppState>) -> CmdResult<simbrief::FlightPlan> {
    let who = state.db.lock().unwrap().settings().map_err(err)?.get("simbrief_id").cloned().unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || simbrief::latest(&who)).await.map_err(err)?
}

/// Start tracking now (even before the battery is on), optionally with a plan.
#[tauri::command]
fn start_flight(state: tauri::State<AppState>, plan: Option<simbrief::FlightPlan>) -> CmdResult<()> {
    send(&state, Command::StartFlight { plan })
}

#[derive(serde::Deserialize)]
struct ImportFile {
    name: String,
    text: String,
}

#[derive(serde::Serialize, Default)]
struct ImportReport {
    imported: Vec<String>,
    duplicates: Vec<String>,
    failed: Vec<(String, String)>,
}

/// Import Volanta flight files. Each becomes a FlightDeck flight; the
/// original file is kept so it can be exported again unchanged. Files
/// already imported are skipped.
#[tauri::command]
fn import_volanta(app: tauri::AppHandle, state: tauri::State<AppState>, files: Vec<ImportFile>) -> ImportReport {
    let mut report = ImportReport::default();
    for file in files {
        let parsed = match volanta::parse(&file.text, &state.refdata) {
            Ok(p) => p,
            Err(e) => {
                report.failed.push((file.name, e));
                continue;
            }
        };
        let mut db = state.db.lock().unwrap();
        if matches!(db.find_imported(volanta::SOURCE, &parsed.id), Ok(Some(_))) {
            report.duplicates.push(file.name);
            continue;
        }
        match db.save_flight(&parsed.summary, &parsed.ctx) {
            Ok(id) => {
                if let Some(note) = parsed.note {
                    let patch = store::book::LogPatch { remarks: Some(note), ..Default::default() };
                    let _ = db.update_log_entry(id, &patch);
                }
                let route = format!(
                    "{} to {}",
                    parsed.ctx.dep.as_ref().map(|p| p.icao.as_str()).unwrap_or("?"),
                    parsed.ctx.arr.as_ref().map(|p| p.icao.as_str()).unwrap_or("?")
                );
                report.imported.push(route);
            }
            Err(e) => report.failed.push((file.name, e.to_string())),
        }
    }
    if !report.imported.is_empty() {
        let _ = app.emit("flights-changed", report.imported.len());
    }
    report
}

/// One flight as a Volanta file: the original file for flights imported
/// from Volanta, otherwise built from FlightDeck's record.
fn volanta_json(state: &AppState, id: i64) -> CmdResult<(String, String)> {
    let db = state.db.lock().unwrap();
    if let Some(imp) = db.get_imported(id).map_err(err)?.filter(|i| i.source == volanta::SOURCE) {
        let name = format!("{}.json", imp.id);
        return Ok((name, imp.file));
    }
    let row = db.get_flight(id).map_err(err)?.ok_or("That flight no longer exists.")?;
    let track = db.get_track(id).map_err(err)?;
    let landings = db.get_landings(id).map_err(err)?;
    let events = db.get_events(id).map_err(err)?;
    let plan: Option<simbrief::FlightPlan> = db.get_plan(id).map_err(err)?.and_then(|j| serde_json::from_str(&j).ok());
    let v = volanta::export(
        &state.refdata,
        &volanta::ExportInput { row: &row, track: &track, landings: &landings, events: &events, plan: plan.as_ref(), note: None },
    );
    Ok((volanta::file_name(&v), serde_json::to_string_pretty(&v).map_err(err)?))
}

fn export_dir(app: &tauri::AppHandle, sub: Option<String>) -> CmdResult<std::path::PathBuf> {
    // Downloads, else Documents, else the home folder.
    let p = app.path();
    let base = p
        .download_dir()
        .or_else(|_| p.document_dir())
        .or_else(|_| p.home_dir())
        .map_err(|e| format!("Could not find a folder to save to: {e}"))?;
    let mut dir = base.join("FlightDeck export");
    if let Some(s) = sub {
        dir = dir.join(s);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Save one flight as a Volanta file in Downloads\FlightDeck export.
#[tauri::command]
fn export_flight_volanta(app: tauri::AppHandle, state: tauri::State<AppState>, id: i64) -> CmdResult<String> {
    let (name, text) = volanta_json(&state, id)?;
    let path = export_dir(&app, None)?.join(name);
    std::fs::write(&path, text).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

#[derive(serde::Serialize)]
struct ExportAll {
    folder: String,
    count: usize,
}

/// Save every flight as a Volanta file, in a new dated folder.
#[tauri::command]
fn export_all_volanta(app: tauri::AppHandle, state: tauri::State<AppState>) -> CmdResult<ExportAll> {
    let ids = state.db.lock().unwrap().all_flight_ids().map_err(err)?;
    let stamp = volanta::format_time(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
        false,
    );
    let sub = format!("All flights {} {}{}", &stamp[0..10], &stamp[11..13], &stamp[14..16]);
    let dir = export_dir(&app, Some(sub))?;
    for id in &ids {
        let (name, text) = volanta_json(&state, *id)?;
        std::fs::write(dir.join(name), text).map_err(err)?;
    }
    Ok(ExportAll { folder: dir.display().to_string(), count: ids.len() })
}

/// The full briefing (OFP) of the current flight plan.
#[tauri::command]
fn current_briefing(state: tauri::State<AppState>) -> Option<simbrief::Ofp> {
    state.shared.lock().unwrap().briefing.clone()
}

#[tauri::command]
fn set_plan(state: tauri::State<AppState>, plan: Option<simbrief::FlightPlan>) -> CmdResult<()> {
    send(&state, Command::SetPlan(plan))
}

#[tauri::command]
fn demo_routes() -> Vec<String> {
    conn_demo::sample_routes().iter().map(|r| format!("{} to {}", r.from.icao, r.to.icao)).collect()
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> CmdResult<HashMap<String, String>> {
    state.db.lock().unwrap().settings().map_err(err)
}

#[tauri::command]
fn set_setting(state: tauri::State<AppState>, key: String, value: String) -> CmdResult<()> {
    const ALLOWED: &[&str] = &["distance_unit", "logbook_layout", "logbook_hand", "passport_color", "map_3d", "map_layers", "simbrief_id"];
    if !ALLOWED.contains(&key.as_str()) {
        return Err(format!("unknown setting {key}"));
    }
    state.db.lock().unwrap().set_setting(&key, &value).map_err(err)
}

#[tauri::command]
fn get_pilot(state: tauri::State<AppState>) -> CmdResult<Pilot> {
    state.db.lock().unwrap().pilot().map_err(err)
}

#[tauri::command]
fn set_pilot(state: tauri::State<AppState>, pilot: Pilot) -> CmdResult<()> {
    state.db.lock().unwrap().set_pilot(&pilot).map_err(err)
}

#[tauri::command]
fn get_logbook(state: tauri::State<AppState>) -> CmdResult<Vec<LogRow>> {
    state.db.lock().unwrap().logbook().map_err(err)
}

#[tauri::command]
fn update_log_entry(state: tauri::State<AppState>, flight_id: i64, patch: LogPatch) -> CmdResult<bool> {
    state.db.lock().unwrap().update_log_entry(flight_id, &patch).map_err(err)
}

/// What the logbook's edit form sends: airports as ICAO codes.
#[derive(serde::Deserialize)]
struct FlightEdit {
    date_ms: Option<i64>,
    out_min: Option<i64>,
    in_min: Option<i64>,
    from_icao: Option<String>,
    to_icao: Option<String>,
    aircraft_type: Option<String>,
    ident: Option<String>,
}

#[tauri::command]
fn update_flight(state: tauri::State<AppState>, flight_id: i64, edit: FlightEdit) -> CmdResult<bool> {
    let place = |code: &Option<String>| -> CmdResult<Option<store::Place>> {
        let Some(code) = code.as_deref().map(str::trim).filter(|c| !c.is_empty()) else { return Ok(None) };
        let a = state
            .refdata
            .airport(code)
            .ok_or_else(|| format!("{} is not an airport code FlightDeck knows. Use the ICAO code, e.g. KORD.", code.to_uppercase()))?;
        Ok(Some(store::Place { icao: a.ident.clone(), name: a.name.clone(), country: a.country.clone(), region: a.region.clone() }))
    };
    let patch = FlightPatch {
        date_ms: edit.date_ms,
        out_min: edit.out_min,
        in_min: edit.in_min,
        dep: place(&edit.from_icao)?,
        arr: place(&edit.to_icao)?,
        aircraft_type: edit.aircraft_type,
        ident: edit.ident,
    };
    state.db.lock().unwrap().update_flight(flight_id, &patch).map_err(err)
}

#[derive(serde::Serialize)]
struct NamedStamp {
    #[serde(flatten)]
    stamp: Stamp,
    country_name: String,
    region_name: String,
}

#[derive(serde::Serialize)]
struct NamedVisit {
    #[serde(flatten)]
    visit: CountryVisit,
    name: String,
}

#[derive(serde::Serialize)]
struct PassportData {
    pilot: Pilot,
    stamps: Vec<NamedStamp>,
    countries: Vec<NamedVisit>,
    first_flight_ms: Option<i64>,
    home_name: Option<String>,
    totals: Totals,
}

fn country_name(r: &refdata::RefData, code: &str) -> String {
    r.country_name(code).unwrap_or(code).to_string()
}

#[tauri::command]
fn get_passport(state: tauri::State<AppState>) -> CmdResult<PassportData> {
    let db = state.db.lock().unwrap();
    let r = &state.refdata;
    let flights = db.stamp_flights().map_err(err)?;
    let stamps = passport::derive_stamps(&flights)
        .into_iter()
        .map(|s| NamedStamp {
            country_name: country_name(r, &s.country),
            region_name: r.region_name(&s.region).unwrap_or("").to_string(),
            stamp: s,
        })
        .collect();
    let countries = passport::country_visits(&flights)
        .into_iter()
        .map(|v| NamedVisit { name: country_name(r, &v.country), visit: v })
        .collect();
    let pilot = db.pilot().map_err(err)?;
    let home_name = r.airport(&pilot.home_icao).map(|a| a.name.clone());
    Ok(PassportData {
        first_flight_ms: flights.first().map(|f| f.dep_ms),
        stamps,
        countries,
        home_name,
        totals: db.totals().map_err(err)?,
        pilot,
    })
}

#[derive(serde::Serialize)]
struct HangarItem {
    #[serde(flatten)]
    row: HangarRow,
    last_country_name: Option<String>,
    last_region_name: Option<String>,
}

#[tauri::command]
fn get_hangar(state: tauri::State<AppState>) -> CmdResult<Vec<HangarItem>> {
    let rows = state.db.lock().unwrap().hangar().map_err(err)?;
    let r = &state.refdata;
    Ok(rows
        .into_iter()
        .map(|row| HangarItem {
            last_country_name: row.last_country.as_deref().map(|c| country_name(r, c)),
            last_region_name: row.last_region.as_deref().and_then(|c| r.region_name(c)).map(str::to_string),
            row,
        })
        .collect())
}

#[derive(serde::Serialize)]
struct Route {
    flight_id: i64,
    dep_icao: Option<String>,
    arr_icao: Option<String>,
    /// [lon, lat]
    from: [f64; 2],
    to: [f64; 2],
    date_ms: i64,
    aircraft_type: String,
}

/// Every flight as a line from its departure to its arrival airport. Ends
/// sit on the airports' published positions when the ICAO code is known.
#[tauri::command]
fn get_routes(state: tauri::State<AppState>) -> CmdResult<Vec<Route>> {
    let rows: Vec<RouteRow> = state.db.lock().unwrap().routes().map_err(err)?;
    let r = &state.refdata;
    let at = |icao: &Option<String>, lat: Option<f64>, lon: Option<f64>| -> Option<[f64; 2]> {
        icao.as_deref()
            .and_then(|i| r.airport(i))
            .map(|a| [a.lon, a.lat])
            .or(lat.zip(lon).map(|(la, lo)| [lo, la]))
    };
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let from = at(&row.dep_icao, row.dep_lat, row.dep_lon)?;
            let to = at(&row.arr_icao, row.arr_lat, row.arr_lon)?;
            Some(Route {
                flight_id: row.flight_id,
                dep_icao: row.dep_icao,
                arr_icao: row.arr_icao,
                from,
                to,
                date_ms: row.date_ms,
                aircraft_type: row.aircraft_type,
            })
        })
        .collect())
}

#[tauri::command]
fn get_aircraft_photo(state: tauri::State<AppState>, id: i64) -> CmdResult<Option<String>> {
    state.db.lock().unwrap().aircraft_photo(id).map_err(err)
}

#[tauri::command]
fn set_aircraft_photo(state: tauri::State<AppState>, id: i64, data_url: Option<String>) -> CmdResult<bool> {
    if let Some(d) = &data_url {
        if !d.starts_with("data:image/") || d.len() > 4_000_000 {
            return Err("Pictures must be images under about 3 MB.".into());
        }
    }
    state.db.lock().unwrap().set_aircraft_photo(id, data_url.as_deref()).map_err(err)
}

#[tauri::command]
fn add_sample_flights(state: tauri::State<AppState>) -> usize {
    samples::add(&state.db, &state.refdata)
}

#[tauri::command]
fn remove_sample_flights(state: tauri::State<AppState>) -> usize {
    samples::remove(&state.db)
}

#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> CmdResult<update::UpdateInfo> {
    let current = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || update::check(&current)).await.map_err(err)?
}

/// Download the new installer, start it, and close FlightDeck so it can
/// replace the program files. Flights are stored elsewhere and are kept.
#[tauri::command]
async fn install_update(app: tauri::AppHandle, url: String) -> CmdResult<()> {
    let path = tauri::async_runtime::spawn_blocking(move || update::download(&url)).await.map_err(err)??;
    // /P: progress bar only, no pages (so no "Already installed" question).
    // /UPDATE: install over the current version without uninstalling it.
    // /R: open FlightDeck again when done.
    std::process::Command::new(&path)
        .args(["/P", "/UPDATE", "/R"])
        .spawn()
        .map_err(|e| format!("Could not start the installer: {e}"))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
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
            let refdata_for_state = refdata.clone();
            engine::spawn(app.handle().clone(), shared.clone(), db.clone(), refdata, data_dir, rx);
            app.manage(AppState { db, refdata: refdata_for_state, shared, tx: Mutex::new(tx) });
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
            simbrief_latest,
            start_flight,
            set_plan,
            current_briefing,
            import_volanta,
            export_flight_volanta,
            export_all_volanta,
            demo_routes,
            get_settings,
            set_setting,
            get_pilot,
            set_pilot,
            get_logbook,
            update_log_entry,
            update_flight,
            get_passport,
            get_hangar,
            get_routes,
            get_aircraft_photo,
            set_aircraft_photo,
            add_sample_flights,
            remove_sample_flights,
            check_update,
            install_update,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlightDeck");
}
