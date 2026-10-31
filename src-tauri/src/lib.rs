//! FlightDeck desktop app: Tauri commands and startup.

mod diag;
mod elevatex;
mod engine;
mod photos;
mod remote;
mod samples;
mod simbrief;
mod volanta;
mod update;

use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use engine::{Command, FullStatus, SharedState};
use std::collections::HashMap;

use store::book::{AircraftEdit, FlightPatch, HangarRow, LogPatch, LogRow, ManualFlight, NewAircraft, Pilot, RouteRow};
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
    data_dir: std::path::PathBuf,
    photos: Arc<photos::Cache>,
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

/// Discard the flight in progress without saving it.
#[tauri::command]
fn cancel_flight(state: tauri::State<AppState>) -> CmdResult<()> {
    send(&state, Command::CancelFlight)
}

/// The pilot's latest SimBrief flight plan, using the Pilot ID from Settings.
#[tauri::command]
async fn simbrief_latest(state: tauri::State<'_, AppState>) -> CmdResult<simbrief::FlightPlan> {
    let who = state.db.lock().unwrap().settings().map_err(err)?.get("simbrief_id").cloned().unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || simbrief::latest(&who)).await.map_err(err)?
}

/// Start tracking now (even before the battery is on), optionally with a plan.
#[tauri::command]
fn start_flight(
    state: tauri::State<AppState>,
    plan: Option<simbrief::FlightPlan>,
    aircraft_id: Option<i64>,
) -> CmdResult<()> {
    // "Fly this aircraft": the flight is logged to this hangar aircraft
    // whatever the sim shows.
    let aircraft = match aircraft_id {
        Some(id) => Some(
            state
                .db
                .lock()
                .unwrap()
                .hangar()
                .map_err(err)?
                .into_iter()
                .find(|a| a.id == id)
                .map(Box::new)
                .ok_or("That aircraft is no longer in the hangar.")?,
        ),
        None => None,
    };
    send(&state, Command::StartFlight { plan, aircraft })
}

#[derive(serde::Serialize)]
struct AirportHit {
    icao: String,
    name: String,
    country: String,
    lat: f64,
    lon: f64,
}

/// Airports for the flight plan fields, matched by code or name.
#[tauri::command]
fn search_airports(state: tauri::State<AppState>, query: String) -> Vec<AirportHit> {
    state
        .refdata
        .search(&query, 8)
        .into_iter()
        .map(|a| AirportHit {
            icao: a.ident.clone(),
            name: a.name.clone(),
            country: state.refdata.country_name(&a.country).unwrap_or(&a.country).to_string(),
            lat: a.lat,
            lon: a.lon,
        })
        .collect()
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
    import_files(&app, &state, files, volanta::SOURCE, volanta::parse)
}

/// Import Elevatex flight files, like Volanta's.
#[tauri::command]
fn import_elevatex(app: tauri::AppHandle, state: tauri::State<AppState>, files: Vec<ImportFile>) -> ImportReport {
    import_files(&app, &state, files, elevatex::SOURCE, elevatex::parse)
}

type Parser = fn(&str, &refdata::RefData) -> Result<volanta::ImportedFlight, String>;

fn import_files(app: &tauri::AppHandle, state: &AppState, files: Vec<ImportFile>, source: &str, parse: Parser) -> ImportReport {
    let mut report = ImportReport::default();
    for file in files {
        let parsed = match parse(&file.text, &state.refdata) {
            Ok(p) => p,
            Err(e) => {
                report.failed.push((file.name, e));
                continue;
            }
        };
        let mut db = state.db.lock().unwrap();
        if matches!(db.find_imported(source, &parsed.id), Ok(Some(_))) {
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
                log::info!("imported {source} flight {} as #{id}: {route}", parsed.id);
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

/// Save the chosen flights (all when none are given) as Volanta files, in
/// a new dated folder.
#[tauri::command]
fn export_all_volanta(app: tauri::AppHandle, state: tauri::State<AppState>, ids: Option<Vec<i64>>) -> CmdResult<ExportAll> {
    let ids = match ids {
        Some(ids) if !ids.is_empty() => ids,
        _ => state.db.lock().unwrap().all_flight_ids().map_err(err)?,
    };
    let stamp = volanta::format_time(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
        false,
    );
    let sub = format!("Flights {} {}{}", &stamp[0..10], &stamp[11..13], &stamp[14..16]);
    let dir = export_dir(&app, Some(sub))?;
    for id in &ids {
        let (name, text) = volanta_json(&state, *id)?;
        std::fs::write(dir.join(name), text).map_err(err)?;
    }
    Ok(ExportAll { folder: dir.display().to_string(), count: ids.len() })
}

/// A plain-text report for diagnosing problems: what FlightDeck sees now,
/// the last few flights step by step, and the recent log.
fn diagnostics_text(app: &tauri::AppHandle, state: &AppState) -> String {
    use std::fmt::Write as _;
    let z = |ms: Option<i64>| ms.map(|t| volanta::format_time(t, true)).unwrap_or_else(|| "-".into());
    let mut o = String::new();
    let _ = writeln!(o, "FlightDeck diagnostics");
    let _ = writeln!(o, "Version {} on {} {}", app.package_info().version, std::env::consts::OS, std::env::consts::ARCH);
    let _ = writeln!(o, "Made {}", z(Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0))));
    let _ = writeln!(o, "Data folder {}", state.data_dir.display());

    let st = state.shared.lock().unwrap().status.clone();
    let _ = writeln!(o, "\n== Now ==");
    let _ = writeln!(o, "link {} | sim {:?} {} | waiting for pilot {} | armed {}", st.link, st.sim, st.sim_version.as_deref().unwrap_or(""), st.waiting_for_pilot, st.armed);
    let _ = writeln!(o, "phase {:?} | aircraft {:?}", st.phase, st.aircraft);
    if let Some(t) = st.telemetry {
        let _ = writeln!(
            o,
            "telemetry: paused {} on ground {} gs {:.1} kt alt {:.0} ft agl {:.0} ft engines {} brake {} powered {:?} camera {:?} sim rate {}",
            t.paused, t.on_ground, t.gs_kt, t.alt_msl_ft, t.alt_agl_ft, t.engines_running, t.parking_brake, t.powered, t.camera_state, t.sim_rate
        );
    }
    if let Some(p) = &st.plan {
        let _ = writeln!(o, "plan {} {}-{} {}", p.callsign, p.origin.icao, p.destination.icao, p.aircraft_icao);
    }

    let db = state.db.lock().unwrap();
    let _ = writeln!(o, "\n== Last flights ==");
    for id in db.recent_flight_ids(5).unwrap_or_default() {
        let Ok(Some(r)) = db.get_flight(id) else { continue };
        let extra = db.flight_diag(id).ok().flatten();
        let _ = writeln!(
            o,
            "\n#{id} {} -> {} | {} | {} / {} / {}",
            r.dep_icao.as_deref().unwrap_or("?"),
            r.arr_icao.as_deref().unwrap_or("?"),
            r.sim,
            r.aircraft_title,
            r.icao_type,
            r.registration
        );
        if let Some((sim, status, reason, paused, air, rate, source)) = extra {
            let _ = writeln!(o, "  sim {sim} | status {status} | ended: {reason} | paused {} s | started airborne {air} | max sim rate {rate} | source {}", paused / 1000, source.as_deref().unwrap_or("FlightDeck"));
        }
        let _ = writeln!(o, "  start {} end {}", z(Some(r.start_ms)), z(Some(r.end_ms)));
        let _ = writeln!(o, "  block out {} takeoff {} landing {} block in {}", z(r.block_out), z(r.takeoff), z(r.landing), z(r.block_in));
        let _ = writeln!(o, "  total {} min, {:.1} nm, max {} ft, fuel {:.0} kg", r.total_min, r.distance_nm, r.max_alt_ft, r.fuel_used_kg);
        for (t, kind, data) in db.get_events(id).unwrap_or_default() {
            if kind == "landing_report" {
                continue;
            }
            let data = if data == "null" { String::new() } else { format!(" {data}") };
            let _ = writeln!(o, "  {} {kind}{data}", z(Some(t)));
        }
        for l in db.get_landings(id).unwrap_or_default() {
            let _ = writeln!(
                o,
                "  landing {} {:.0} fpm ({}) {:.2} G, {} bounces, touch and go {}",
                z(Some(l.t_ms)),
                l.vs_fpm,
                l.vs_source,
                l.g_peak,
                l.bounces,
                l.touch_and_go
            );
        }
        let track = db.get_track(id).unwrap_or_default();
        let _ = writeln!(o, "  track {} points; last 12:", track.len());
        for p in track.iter().rev().take(12).rev() {
            let _ = writeln!(
                o,
                "    {} gs {:.1} alt {:.0} vs {:.0} hdg {:.0} ground {}",
                z(Some(p.t_ms)),
                p.gs_kt,
                p.alt_ft,
                p.vs_fpm,
                p.hdg_deg,
                p.on_ground
            );
        }
    }
    drop(db);

    let _ = writeln!(o, "\n== Log (latest, including earlier sessions) ==");
    let mut lines = diag::file_tail(&state.data_dir, 600);
    if lines.is_empty() {
        lines = diag::recent(600);
    }
    for line in lines {
        let _ = writeln!(o, "{line}");
    }
    o
}

#[tauri::command]
fn get_diagnostics(app: tauri::AppHandle, state: tauri::State<AppState>) -> String {
    diagnostics_text(&app, &state)
}

/// Save the diagnostics report as a text file next to the exports.
#[tauri::command]
fn save_diagnostics(app: tauri::AppHandle, state: tauri::State<AppState>) -> CmdResult<String> {
    let text = diagnostics_text(&app, &state);
    let stamp = volanta::format_time(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
        false,
    );
    let name = format!("FlightDeck diagnostics {} {}{}.txt", &stamp[0..10], &stamp[11..13], &stamp[14..16]);
    let path = export_dir(&app, None)?.join(name);
    std::fs::write(&path, text).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    Ok(path.display().to_string())
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
    let mut s = state.db.lock().unwrap().settings().map_err(err)?;
    // Pairing tokens never leave the store.
    s.retain(|k, _| !k.starts_with("remote_devices"));
    Ok(s)
}

#[tauri::command]
fn set_setting(state: tauri::State<AppState>, key: String, value: String) -> CmdResult<()> {
    const ALLOWED: &[&str] = &["distance_unit", "logbook_layout", "logbook_hand", "logbook_theme", "logbook_view", "passport_color", "map_3d", "map_layers", "simbrief_id"];
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

/// An airport by its code, with where it is ([lat, lon]).
fn airport_place(refdata: &refdata::RefData, code: &Option<String>) -> CmdResult<Option<(store::Place, [f64; 2])>> {
    let Some(code) = code.as_deref().map(str::trim).filter(|c| !c.is_empty()) else { return Ok(None) };
    let a = refdata
        .airport(code)
        .ok_or_else(|| format!("{} is not an airport code FlightDeck knows. Use the ICAO code, e.g. KORD.", code.to_uppercase()))?;
    Ok(Some((
        store::Place { icao: a.ident.clone(), name: a.name.clone(), country: a.country.clone(), region: a.region.clone() },
        [a.lat, a.lon],
    )))
}

#[tauri::command]
fn update_flight(state: tauri::State<AppState>, flight_id: i64, edit: FlightEdit) -> CmdResult<bool> {
    let dep = airport_place(&state.refdata, &edit.from_icao)?;
    let arr = airport_place(&state.refdata, &edit.to_icao)?;
    let patch = FlightPatch {
        date_ms: edit.date_ms,
        out_min: edit.out_min,
        in_min: edit.in_min,
        dep_at: dep.as_ref().map(|d| d.1),
        arr_at: arr.as_ref().map(|a| a.1),
        dep: dep.map(|d| d.0),
        arr: arr.map(|a| a.0),
        aircraft_type: edit.aircraft_type,
        ident: edit.ident,
    };
    state.db.lock().unwrap().update_flight(flight_id, &patch).map_err(err)
}

#[derive(serde::Deserialize)]
struct ManualEntry {
    date_ms: i64,
    out_min: Option<i64>,
    in_min: Option<i64>,
    from_icao: String,
    to_icao: String,
    aircraft_id: Option<i64>,
    #[serde(default)]
    aircraft_type: String,
    #[serde(default)]
    ident: String,
    #[serde(default)]
    callsign: String,
    #[serde(default)]
    total_min: i64,
    #[serde(default)]
    night_min: i64,
    #[serde(default)]
    ifr_min: i64,
    #[serde(default)]
    day_landings: i64,
    #[serde(default)]
    night_landings: i64,
    #[serde(default)]
    remarks: String,
}

/// Log a flight that FlightDeck did not record.
#[tauri::command]
fn add_manual_entry(app: tauri::AppHandle, state: tauri::State<AppState>, entry: ManualEntry) -> CmdResult<i64> {
    let (dep, dep_at) = airport_place(&state.refdata, &Some(entry.from_icao.clone()))?.ok_or("Enter the departure airport.")?;
    let (arr, arr_at) = airport_place(&state.refdata, &Some(entry.to_icao.clone()))?.ok_or("Enter the arrival airport.")?;
    if entry.total_min <= 0 && (entry.out_min.is_none() || entry.in_min.is_none()) {
        return Err("Enter the block times or the total time.".into());
    }
    let m = ManualFlight {
        date_ms: entry.date_ms,
        out_min: entry.out_min,
        in_min: entry.in_min,
        dep,
        arr,
        dep_at: Some(dep_at),
        arr_at: Some(arr_at),
        aircraft_id: entry.aircraft_id,
        aircraft_type: entry.aircraft_type,
        ident: entry.ident,
        callsign: entry.callsign,
        total_min: entry.total_min,
        night_min: entry.night_min,
        ifr_min: entry.ifr_min,
        day_landings: entry.day_landings,
        night_landings: entry.night_landings,
        remarks: entry.remarks,
    };
    let id = state.db.lock().unwrap().add_manual_flight(&m).map_err(err)?;
    log::info!("logbook entry #{id} added by hand: {} to {}", entry.from_icao, entry.to_icao);
    let _ = app.emit("flights-changed", 1);
    Ok(id)
}

#[tauri::command]
fn update_aircraft(app: tauri::AppHandle, state: tauri::State<AppState>, id: i64, edit: AircraftEdit) -> CmdResult<bool> {
    let ok = state.db.lock().unwrap().update_aircraft(id, &edit).map_err(err)?;
    let _ = app.emit("flights-changed", 0);
    Ok(ok)
}

#[tauri::command]
fn add_aircraft(state: tauri::State<AppState>, aircraft: NewAircraft) -> CmdResult<i64> {
    if aircraft.registration.trim().is_empty() {
        return Err("Enter the aircraft's registration.".into());
    }
    if aircraft.model.trim().is_empty() && aircraft.icao_type.trim().is_empty() {
        return Err("Enter the aircraft model.".into());
    }
    let db = state.db.lock().unwrap();
    if db.aircraft_with_registration(&aircraft.registration).map_err(err)?.is_some() {
        return Err(format!("{} is already in your hangar.", aircraft.registration.trim().to_uppercase()));
    }
    db.add_aircraft(&aircraft).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "That aircraft is already in your hangar.".to_string()
        } else {
            err(e)
        }
    })
}

/// Delete an aircraft; its flights move to `move_to`, or are deleted too.
#[tauri::command]
fn delete_aircraft(app: tauri::AppHandle, state: tauri::State<AppState>, id: i64, move_to: Option<i64>) -> CmdResult<usize> {
    let n = state.db.lock().unwrap().delete_aircraft(id, move_to).map_err(err)?;
    match move_to {
        Some(t) => log::info!("aircraft #{id} deleted; {n} flights moved to #{t}"),
        None => log::info!("aircraft #{id} deleted with its {n} flights"),
    }
    let _ = app.emit("flights-changed", n);
    Ok(n)
}

/// Sets of flights that look like the same flight logged twice or more.
#[tauri::command]
fn find_duplicates(state: tauri::State<AppState>) -> CmdResult<Vec<Vec<store::DuplicateFlight>>> {
    state.db.lock().unwrap().duplicate_flights().map_err(err)
}

/// Delete the chosen flights (from the duplicate finder).
#[tauri::command]
fn delete_flights(app: tauri::AppHandle, state: tauri::State<AppState>, ids: Vec<i64>) -> CmdResult<usize> {
    let n = state.db.lock().unwrap().delete_flights(&ids).map_err(err)?;
    log::info!("deleted {n} duplicate flights");
    let _ = app.emit("flights-changed", n);
    Ok(n)
}

/// The hangar aircraft with a registration, if any (to warn before merging).
#[tauri::command]
fn aircraft_with_registration(state: tauri::State<AppState>, registration: String) -> CmdResult<Option<i64>> {
    state.db.lock().unwrap().aircraft_with_registration(&registration).map_err(err)
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
fn set_aircraft_favorite(state: tauri::State<AppState>, id: i64, favorite: bool) -> CmdResult<bool> {
    state.db.lock().unwrap().set_aircraft_favorite(id, favorite).map_err(err)
}

/// The latest Planespotters photo of a registration (None if there is none).
#[tauri::command]
async fn spotter_photo(state: tauri::State<'_, AppState>, registration: String) -> CmdResult<Option<photos::SpotterPhoto>> {
    let cache = state.photos.clone();
    tauri::async_runtime::spawn_blocking(move || cache.photo(&registration)).await.map_err(err)?
}

/// Every flight flown in one aircraft, newest first.
#[tauri::command]
fn aircraft_flights(state: tauri::State<AppState>, id: i64) -> CmdResult<Vec<FlightRow>> {
    state.db.lock().unwrap().flights_of_aircraft(id).map_err(err)
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

// --- access from other devices --------------------------------------------

#[tauri::command]
fn remote_info(app: tauri::AppHandle) -> remote::RemoteInfo {
    remote::info(&app)
}

#[tauri::command]
fn remote_set(app: tauri::AppHandle, enabled: Option<bool>, can_edit: Option<bool>) -> remote::RemoteInfo {
    if let Some(e) = can_edit {
        remote::set_edit(&app, e);
    }
    if let Some(on) = enabled {
        remote::set_enabled(&app, on);
    }
    remote::info(&app)
}

#[tauri::command]
fn remote_new_code(app: tauri::AppHandle) -> remote::RemoteInfo {
    remote::new_pairing_code(&app);
    remote::info(&app)
}

#[tauri::command]
fn remote_forget(app: tauri::AppHandle) -> remote::RemoteInfo {
    remote::forget_devices(&app);
    remote::info(&app)
}

/// Run a command for a browser on another device: the same functions the
/// desktop window calls, with the browser's (camelCase) arguments.
pub(crate) async fn dispatch(app: &tauri::AppHandle, cmd: &str, a: serde_json::Value) -> Result<serde_json::Value, String> {
    use serde::de::DeserializeOwned;
    use serde_json::to_value;
    fn arg<T: DeserializeOwned>(a: &serde_json::Value, k: &str) -> Result<T, String> {
        serde_json::from_value(a.get(k).cloned().unwrap_or(serde_json::Value::Null)).map_err(|e| format!("{k}: {e}"))
    }
    let st = || app.state::<AppState>();
    let ok = |v: Result<serde_json::Value, serde_json::Error>| v.map_err(|e| e.to_string());
    match cmd {
        "get_status" => ok(to_value(get_status(st()))),
        "list_flights" => ok(to_value(list_flights(st(), arg(&a, "limit")?, arg(&a, "offset")?)?)),
        "get_totals" => ok(to_value(get_totals(st())?)),
        "get_flight" => ok(to_value(get_flight(st(), arg(&a, "id")?)?)),
        "delete_flight" => ok(to_value(delete_flight(st(), arg(&a, "id")?)?)),
        "start_demo" => ok(to_value(start_demo(st(), arg(&a, "route")?, arg(&a, "speedup")?)?)),
        "stop_demo" => ok(to_value(stop_demo(st())?)),
        "end_flight" => ok(to_value(end_flight(st())?)),
        "cancel_flight" => ok(to_value(cancel_flight(st())?)),
        "simbrief_latest" => ok(to_value(simbrief_latest(st()).await?)),
        "start_flight" => ok(to_value(start_flight(st(), arg(&a, "plan")?, arg(&a, "aircraftId")?)?)),
        "set_plan" => ok(to_value(set_plan(st(), arg(&a, "plan")?)?)),
        "current_briefing" => ok(to_value(current_briefing(st()))),
        "import_volanta" => ok(to_value(import_volanta(app.clone(), st(), arg(&a, "files")?))),
        "import_elevatex" => ok(to_value(import_elevatex(app.clone(), st(), arg(&a, "files")?))),
        "demo_routes" => ok(to_value(demo_routes())),
        "search_airports" => ok(to_value(search_airports(st(), arg(&a, "query")?))),
        "get_settings" => ok(to_value(get_settings(st())?)),
        "set_setting" => ok(to_value(set_setting(st(), arg(&a, "key")?, arg(&a, "value")?)?)),
        "get_pilot" => ok(to_value(get_pilot(st())?)),
        "set_pilot" => ok(to_value(set_pilot(st(), arg(&a, "pilot")?)?)),
        "get_logbook" => ok(to_value(get_logbook(st())?)),
        "update_log_entry" => ok(to_value(update_log_entry(st(), arg(&a, "flightId")?, arg(&a, "patch")?)?)),
        "update_flight" => ok(to_value(update_flight(st(), arg(&a, "flightId")?, arg(&a, "edit")?)?)),
        "add_manual_entry" => ok(to_value(add_manual_entry(app.clone(), st(), arg(&a, "entry")?)?)),
        "update_aircraft" => ok(to_value(update_aircraft(app.clone(), st(), arg(&a, "id")?, arg(&a, "edit")?)?)),
        "add_aircraft" => ok(to_value(add_aircraft(st(), arg(&a, "aircraft")?)?)),
        "delete_aircraft" => ok(to_value(delete_aircraft(app.clone(), st(), arg(&a, "id")?, arg(&a, "moveTo")?)?)),
        "find_duplicates" => ok(to_value(find_duplicates(st())?)),
        "delete_flights" => ok(to_value(delete_flights(app.clone(), st(), arg(&a, "ids")?)?)),
        "aircraft_with_registration" => ok(to_value(aircraft_with_registration(st(), arg(&a, "registration")?)?)),
        "get_passport" => ok(to_value(get_passport(st())?)),
        "get_hangar" => ok(to_value(get_hangar(st())?)),
        "get_routes" => ok(to_value(get_routes(st())?)),
        "set_aircraft_favorite" => ok(to_value(set_aircraft_favorite(st(), arg(&a, "id")?, arg(&a, "favorite")?)?)),
        "spotter_photo" => ok(to_value(spotter_photo(st(), arg(&a, "registration")?).await?)),
        "aircraft_flights" => ok(to_value(aircraft_flights(st(), arg(&a, "id")?)?)),
        "get_aircraft_photo" => ok(to_value(get_aircraft_photo(st(), arg(&a, "id")?)?)),
        "set_aircraft_photo" => ok(to_value(set_aircraft_photo(st(), arg(&a, "id")?, arg(&a, "dataUrl")?)?)),
        "add_sample_flights" => ok(to_value(add_sample_flights(st()))),
        "remove_sample_flights" => ok(to_value(remove_sample_flights(st()))),
        "app_version" => ok(to_value(app_version(app.clone()))),
        _ => Err(format!("unknown command {cmd}")),
    }
}

pub fn run() {
    diag::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            diag::set_folder(&data_dir);
            log::info!(
                "FlightDeck {} starting on {} {}; data folder: {}",
                app.package_info().version,
                std::env::consts::OS,
                std::env::consts::ARCH,
                data_dir.display()
            );
            let db = Arc::new(Mutex::new(Store::open(&data_dir.join("flightdeck.db"))?));
            let refdata = Arc::new(refdata::RefData::load());
            engine::recover_journal(&data_dir, &db, &refdata);

            let shared: SharedState = Default::default();
            let (tx, rx) = channel();
            let refdata_for_state = refdata.clone();
            engine::spawn(app.handle().clone(), shared.clone(), db.clone(), refdata, data_dir.clone(), rx);
            app.manage(AppState { db, refdata: refdata_for_state, shared, tx: Mutex::new(tx), data_dir, photos: Default::default() });
            app.manage(remote::Remote::default());
            remote::init(app.handle());
            remote::start_if_enabled(app.handle());
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
            cancel_flight,
            simbrief_latest,
            start_flight,
            set_plan,
            current_briefing,
            import_volanta,
            import_elevatex,
            export_flight_volanta,
            export_all_volanta,
            get_diagnostics,
            set_aircraft_favorite,
            update_aircraft,
            add_aircraft,
            delete_aircraft,
            find_duplicates,
            remote_info,
            remote_set,
            remote_new_code,
            remote_forget,
            delete_flights,
            aircraft_with_registration,
            add_manual_entry,
            spotter_photo,
            aircraft_flights,
            save_diagnostics,
            demo_routes,
            search_airports,
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
