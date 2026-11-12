//! FlightDeck desktop app: Tauri commands and startup.

mod airports;
mod diag;
mod elevatex;
mod engine;
mod photos;
mod efb;
mod networks;
mod airspace;
mod countries;
mod fixes;
mod obstacles;
mod pln;
mod backup;
mod screenshots;
mod icing;
mod navaids;
mod winds;
mod wx;
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

pub(crate) struct AppState {
    pub(crate) db: Arc<Mutex<Store>>,
    refdata: Arc<refdata::RefData>,
    shared: SharedState,
    tx: Mutex<Sender<Command>>,
    data_dir: std::path::PathBuf,
    photos: Arc<photos::Cache>,
    nav: navaids::Shared,
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
    /// Its pilot score, and where the pilot stands now.
    score: Option<rating::FlightScore>,
    standing: Option<Standing>,
}

#[derive(serde::Serialize)]
struct Standing {
    rating: f64,
    xp: f64,
    rank: rating::Rank,
    next_rank: Option<rating::Rank>,
}

#[tauri::command]
fn get_flight(state: tauri::State<AppState>, id: i64) -> CmdResult<Option<FlightDetail>> {
    // A score that cannot be worked out never hides the flight itself.
    let profile = rating_profile(&state).map_err(|e| log::warn!("pilot rating: {e}")).ok();
    let db = state.db.lock().unwrap();
    let Some(flight) = db.get_flight(id).map_err(err)? else { return Ok(None) };
    let plan = db.get_plan(id).map_err(err)?.and_then(|j| serde_json::from_str(&j).ok());
    let score = profile.as_ref().and_then(|p| p.flights.iter().find(|f| f.flight_id == id).cloned());
    let standing = profile.map(|p| Standing { rating: p.rating, xp: p.xp, rank: p.rank, next_rank: p.next_rank });
    Ok(Some(FlightDetail {
        flight,
        track: db.get_track(id).map_err(err)?,
        landings: db.get_landings(id).map_err(err)?,
        plan,
        score,
        standing,
    }))
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
fn import_volanta(app: tauri::AppHandle, state: tauri::State<AppState>, files: Vec<ImportFile>, force: Option<Vec<String>>) -> ImportReport {
    import_files(&app, &state, files, volanta::SOURCE, volanta::parse, &force.unwrap_or_default())
}

/// Import Elevatex flight files, like Volanta's.
#[tauri::command]
fn import_elevatex(app: tauri::AppHandle, state: tauri::State<AppState>, files: Vec<ImportFile>, force: Option<Vec<String>>) -> ImportReport {
    import_files(&app, &state, files, elevatex::SOURCE, elevatex::parse, &force.unwrap_or_default())
}

/// One flight file as it would be imported, for the pilot to confirm.
#[derive(serde::Serialize)]
struct ImportPreview {
    name: String,
    dep: Option<String>,
    arr: Option<String>,
    start_ms: i64,
    minutes: i64,
    icao_type: String,
    registration: String,
    callsign: String,
    /// Why this looks like a flight FlightDeck has already, if it does.
    duplicate: Option<String>,
    /// Why the file cannot be imported, if it cannot.
    error: Option<String>,
}

/// Read flight files without saving anything: what each holds, and which
/// look like flights already in FlightDeck (the same file imported before,
/// or a flight on the same day between the same airports) or appear twice
/// in this import.
#[tauri::command]
fn preview_import(state: tauri::State<AppState>, kind: String, files: Vec<ImportFile>) -> Vec<ImportPreview> {
    let (source, parse): (&str, Parser) = if kind == "elevatex" { (elevatex::SOURCE, elevatex::parse) } else { (volanta::SOURCE, volanta::parse) };
    let mut seen: HashMap<String, String> = HashMap::new();
    files
        .into_iter()
        .map(|file| match parse(&file.text, &state.refdata) {
            Err(e) => ImportPreview {
                name: file.name,
                dep: None,
                arr: None,
                start_ms: 0,
                minutes: 0,
                icao_type: String::new(),
                registration: String::new(),
                callsign: String::new(),
                duplicate: None,
                error: Some(e),
            },
            Ok(p) => {
                let f = &p.summary;
                let dep = p.ctx.dep.as_ref().map(|x| x.icao.clone());
                let arr = p.ctx.arr.as_ref().map(|x| x.icao.clone());
                let out = f.block_out_ms.or(f.takeoff_ms).unwrap_or(f.start_ms);
                let end = f.block_in_ms.or(f.landing_ms).unwrap_or(f.end_ms);
                let day = out.div_euclid(86_400_000);
                let key = format!("{day}|{}|{}", dep.as_deref().unwrap_or(""), arr.as_deref().unwrap_or(""));
                let db = state.db.lock().unwrap();
                let duplicate = if matches!(db.find_imported(source, &p.id), Ok(Some(_))) {
                    Some("Already imported".to_string())
                } else if matches!(db.similar_flight(day, dep.as_deref().unwrap_or(""), arr.as_deref().unwrap_or("")), Ok(Some(_))) {
                    Some("Same day and airports as a flight in FlightDeck".to_string())
                } else {
                    seen.get(&key).map(|other| format!("Same flight as {other} in this import"))
                };
                seen.entry(key).or_insert_with(|| file.name.clone());
                ImportPreview {
                    name: file.name,
                    dep,
                    arr,
                    start_ms: out,
                    minutes: ((end - out).max(0) / 60_000),
                    icao_type: f.aircraft.icao_type.clone(),
                    registration: f.aircraft.registration.clone(),
                    callsign: p.ctx.callsign.clone(),
                    duplicate,
                    error: None,
                }
            }
        })
        .collect()
}

/// The airport an aircraft at this position is at (within a few miles),
/// for "Fly this aircraft" and "Start a flight" to fill in the origin.
#[tauri::command]
fn airport_at(state: tauri::State<AppState>, lat: f64, lon: f64) -> Option<AirportHit> {
    state.refdata.airport_at(lat, lon, true).map(|a| AirportHit {
        icao: a.ident.clone(),
        name: a.name.clone(),
        country: state.refdata.country_name(&a.country).unwrap_or(&a.country).to_string(),
        lat: a.lat,
        lon: a.lon,
    })
}

/// Airports for the map at a detail level (1 major, 2 + regional, 3 + small).
#[tauri::command]
fn map_airports(state: tauri::State<AppState>, detail: u8) -> Vec<airports::MapAirport> {
    airports::map_airports(&state.refdata, detail)
}

/// A network's pilots and controllers (VATSIM or IVAO) for the map.
#[tauri::command]
async fn online_network(net: String) -> CmdResult<networks::NetworkData> {
    tauri::async_runtime::spawn_blocking(move || networks::network_for_map(&net)).await.map_err(err)
}

/// Flight categories (VFR, MVFR, IFR, LIFR) of every METAR station.
#[tauri::command]
async fn wx_categories() -> CmdResult<wx::Categories> {
    tauri::async_runtime::spawn_blocking(wx::categories).await.map_err(err)
}

/// US obstacles (towers, antennas, wind turbines) as GeoJSON text (FAA DOF).
#[tauri::command]
async fn us_obstacles(state: tauri::State<'_, AppState>) -> CmdResult<String> {
    let dir = state.data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || obstacles::us(&dir)).await.map_err(err)?
}

/// A PFD is open: feed it for the next few seconds (the window asks again
/// while it stays open).
#[tauri::command]
fn pfd_watch(state: tauri::State<AppState>) -> CmdResult<()> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    state.shared.lock().unwrap().pfd_until_ms = now + 6_000;
    Ok(())
}

/// US Class B, C and D airspace as GeoJSON text (FAA, every 28 days).
#[tauri::command]
async fn us_airspace(state: tauri::State<'_, AppState>) -> CmdResult<String> {
    let dir = state.data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || airspace::us(&dir)).await.map_err(err)?
}

/// US special use airspace (prohibited, restricted, MOAs, warning, alert)
/// as GeoJSON text (FAA, every 28 days).
#[tauri::command]
async fn sua_airspace(state: tauri::State<'_, AppState>) -> CmdResult<String> {
    let dir = state.data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || airspace::sua(&dir)).await.map_err(err)?
}

/// Airspace outside the US in a box (openAIP, a few new countries per call).
#[derive(serde::Serialize)]
struct WorldAirspace {
    geojson: String,
    /// More countries in the box are still to be read: ask again.
    more: bool,
}

#[tauri::command]
async fn world_airspace(state: tauri::State<'_, AppState>, w: f64, s: f64, e: f64, n: f64) -> CmdResult<WorldAirspace> {
    let dir = state.data_dir.clone();
    let (geojson, more) = tauri::async_runtime::spawn_blocking(move || airspace::world(&dir, w, s, e, n, 2)).await.map_err(err)?;
    Ok(WorldAirspace { geojson, more })
}

/// openAIP's airports in a box: private and fuel, for the chart symbols.
#[derive(serde::Serialize)]
struct AirportExtras {
    list: Vec<airspace::AirportExtra>,
    more: bool,
}

#[tauri::command]
async fn airport_extras(state: tauri::State<'_, AppState>, w: f64, s: f64, e: f64, n: f64) -> CmdResult<AirportExtras> {
    let dir = state.data_dir.clone();
    let (list, more) = tauri::async_runtime::spawn_blocking(move || airspace::airport_extras(&dir, w, s, e, n, 2)).await.map_err(err)?;
    Ok(AirportExtras { list, more })
}

/// Positions of waypoints named in a flight plan without them (SimBrief and
/// MSFS 2024 .PLN files): the sim's nav data on this PC, then the FAA.
#[tauri::command]
async fn resolve_fixes(state: tauri::State<'_, AppState>, idents: Vec<String>) -> CmdResult<Vec<fixes::FixPos>> {
    let nav = state.nav.clone();
    let dir = state.data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || fixes::resolve(&nav, &dir, &idents)).await.map_err(err)
}

/// Nav aids (from the sim's own database) in a box, for the map layer:
/// VORs and NDBs, and waypoints when `waypoints`. Asking means the layer is
/// on, so FlightDeck asks the sim for them while it is connected.
#[tauri::command]
fn navaids(state: tauri::State<AppState>, w: f64, s: f64, e: f64, n: f64, waypoints: bool) -> navaids::NavView {
    let mut db = state.nav.lock().unwrap();
    db.wanted = true;
    let (items, more) = db.query(w, s, e, n, waypoints, 3000);
    navaids::NavView { items, more, known: db.len(), waypoints: db.waypoints(), world_ms: db.world_ms }
}

/// Ask the sim for the whole world's nav aids again (new navdata).
#[tauri::command]
fn navaids_refresh(state: tauri::State<AppState>) {
    let mut db = state.nav.lock().unwrap();
    db.wanted = true;
    db.refresh = true;
}

/// Icing Potential (simulation estimate) at a flight level, worldwide.
#[tauri::command]
async fn icing_grid(fl: u32) -> CmdResult<icing::IcingGrid> {
    tauri::async_runtime::spawn_blocking(move || icing::grid(fl.min(450))).await.map_err(err)?
}

/// Icing Potential and the conditions behind it at one place.
#[tauri::command]
async fn icing_point(lat: f64, lon: f64, fl: u32) -> CmdResult<icing::IcingPoint> {
    tauri::async_runtime::spawn_blocking(move || icing::point(lat, lon, fl.min(450))).await.map_err(err)?
}

#[tauri::command]
async fn temperature_grid(level: Option<String>) -> CmdResult<winds::TempGrid> {
    let level = level.unwrap_or_else(|| "sfc".into());
    tauri::async_runtime::spawn_blocking(move || winds::temperature(&level)).await.map_err(err)
}

/// Cloud cover ("cloud") or surface visibility ("vis") from the forecast.
#[tauri::command]
async fn sky_grid(kind: String) -> CmdResult<winds::TempGrid> {
    tauri::async_runtime::spawn_blocking(move || winds::sky(&kind)).await.map_err(err)
}

#[tauri::command]
async fn wind_grid(level: String) -> CmdResult<winds::WindGrid> {
    tauri::async_runtime::spawn_blocking(move || winds::wind(&level)).await.map_err(err)
}

/// Current SIGMETs and AIRMETs as map shapes.
#[tauri::command]
async fn wx_advisories() -> CmdResult<wx::Advisories> {
    tauri::async_runtime::spawn_blocking(wx::advisories).await.map_err(err)
}

/// One online pilot with the whole flight plan.
#[tauri::command]
fn online_pilot(net: String, callsign: String) -> Option<networks::Pilot> {
    networks::pilot(&net, &callsign)
}

/// An airport's card: details, runways, frequencies and your visits.
#[tauri::command]
fn airport_card(state: tauri::State<AppState>, ident: String) -> CmdResult<Option<airports::AirportCard>> {
    let flights = state.db.lock().unwrap().airport_flights(&ident.to_ascii_uppercase()).map_err(err)?;
    Ok(airports::card(&state.refdata, &flights, &ident))
}

/// An airport's elevation and runway thresholds, for the profile's
/// glidepath view.
#[derive(serde::Serialize)]
struct RunwayEnds {
    elevation_ft: i32,
    ends: Vec<refdata::RunwayEnd>,
}
#[tauri::command]
fn runway_ends(state: tauri::State<AppState>, ident: String) -> Option<RunwayEnds> {
    let a = state.refdata.airport(&ident)?;
    Some(RunwayEnds { elevation_ft: a.elevation_ft, ends: state.refdata.runway_ends_of(&a.ident) })
}

/// Back up everything to a file the pilot chooses. Returns where.
#[tauri::command]
async fn backup_save(app: tauri::AppHandle) -> CmdResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let a = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        a.dialog()
            .file()
            .set_title("Save a FlightDeck backup")
            .set_file_name(backup::dated_name())
            .add_filter("FlightDeck backup", &[backup::EXT])
            .blocking_save_file()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let db = app.state::<AppState>().db.clone();
    let p = path.clone();
    tauri::async_runtime::spawn_blocking(move || db.lock().unwrap().backup_to(&p)).await.map_err(err)?.map_err(err)?;
    log::info!("backup saved to {}", path.display());
    Ok(Some(path.display().to_string()))
}

/// A route drawn on the map as an MSFS .PLN text, with the file name to
/// offer (for devices, which download it).
#[tauri::command]
fn pln_text(state: tauri::State<'_, AppState>, route: pln::Route) -> CmdResult<(String, String)> {
    let rd = state.refdata.clone();
    Ok((pln::file_name(&route), pln::write(&route, |id| rd.airport(id).map(|a| a.elevation_ft as f64))))
}

/// Save a route drawn on the map as an MSFS .PLN file wherever the pilot
/// picks. Returns the path, or None if the picker was closed.
#[tauri::command]
async fn save_pln(app: tauri::AppHandle, route: pln::Route) -> CmdResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let rd = app.state::<AppState>().refdata.clone();
    let text = pln::write(&route, |id| rd.airport(id).map(|a| a.elevation_ft as f64));
    let name = pln::file_name(&route);
    let a = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        a.dialog()
            .file()
            .set_title("Save the flight plan for MSFS")
            .set_file_name(name)
            .add_filter("MSFS flight plan", &["pln", "PLN"])
            .blocking_save_file()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    std::fs::write(&path, text).map_err(|e| format!("Could not save the flight plan: {e}"))?;
    log::info!("flight plan saved to {}", path.display());
    Ok(Some(path.display().to_string()))
}

/// Choose the folder kept backed up (a cloud folder, say), and back up to
/// it at once. Returns the folder, or None if the picker was closed.
#[tauri::command]
async fn backup_choose_folder(app: tauri::AppHandle) -> CmdResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let a = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || a.dialog().file().set_title("Folder for FlightDeck backups").blocking_pick_folder())
        .await
        .map_err(err)?;
    let Some(dir) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let db = app.state::<AppState>().db.clone();
    db.lock().unwrap().set_setting(backup::KEY_DIR, &dir.display().to_string()).map_err(err)?;
    let d = dir.clone();
    tauri::async_runtime::spawn_blocking(move || backup::to_folder(&db, &d)).await.map_err(err)??;
    Ok(Some(dir.display().to_string()))
}

/// Stop keeping a folder backed up.
#[tauri::command]
fn backup_forget_folder(state: tauri::State<AppState>) -> CmdResult<()> {
    let db = state.db.lock().unwrap();
    db.set_setting(backup::KEY_DIR, "").map_err(err)?;
    db.set_setting(backup::KEY_ON_CLOSE, "0").map_err(err)
}

/// Back up to the chosen folder now.
#[tauri::command]
async fn backup_now(app: tauri::AppHandle) -> CmdResult<String> {
    let db = app.state::<AppState>().db.clone();
    let dir = db.lock().unwrap().settings().map_err(err)?.get(backup::KEY_DIR).cloned().unwrap_or_default();
    if dir.trim().is_empty() {
        return Err("Choose a backup folder first.".into());
    }
    let f = tauri::async_runtime::spawn_blocking(move || backup::to_folder(&db, std::path::Path::new(&dir))).await.map_err(err)??;
    Ok(f.display().to_string())
}

/// Choose a backup to restore: it is checked and set aside, and replaces
/// everything here when FlightDeck restarts. Returns what it holds.
#[tauri::command]
async fn backup_restore_pick(app: tauri::AppHandle) -> CmdResult<Option<store::BackupInfo>> {
    use tauri_plugin_dialog::DialogExt;
    let a = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        a.dialog().file().set_title("Restore a FlightDeck backup").add_filter("FlightDeck backup", &[backup::EXT, "db"]).blocking_pick_file()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let info = Store::check_backup(&path).map_err(|e| format!("This is not a FlightDeck backup FlightDeck can read ({e})."))?;
    backup::stage(&app.state::<AppState>().data_dir, &path)?;
    log::info!("restore staged from {}", path.display());
    Ok(Some(info))
}

/// Do not restore the backup chosen after all.
#[tauri::command]
fn backup_restore_cancel(state: tauri::State<AppState>) {
    backup::unstage(&state.data_dir);
}

/// Restart FlightDeck (to finish a restore).
#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

/// Every screenshot, newest first, placed on its flight (the Photos tab).
#[tauri::command]
fn photos(state: tauri::State<AppState>) -> CmdResult<Vec<store::Photo>> {
    state.db.lock().unwrap().photos().map_err(err)
}

/// Delete a screenshot: to the Recycle Bin (file_too), or only forgotten.
#[tauri::command]
async fn delete_screenshot(state: tauri::State<'_, AppState>, id: i64, file_too: bool) -> CmdResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || screenshots::delete(&db, id, file_too)).await.map_err(err)?
}

/// A saved flight's screenshots, in the order taken.
#[tauri::command]
fn flight_screenshots(state: tauri::State<AppState>, id: i64) -> CmdResult<Vec<store::Screenshot>> {
    state.db.lock().unwrap().screenshots_of(id).map_err(err)
}

/// Screenshots taken since a time and not yet part of a saved flight (the
/// flight under way's).
#[tauri::command]
fn screenshots_since(state: tauri::State<AppState>, since_ms: i64) -> CmdResult<Vec<store::Screenshot>> {
    state.db.lock().unwrap().screenshots_since(since_ms).map_err(err)
}

/// Choose the screenshots folder with Windows' folder picker. Returns the
/// folder chosen, or None if the pilot closed the picker.
#[tauri::command]
async fn pick_screenshot_folder(app: tauri::AppHandle) -> CmdResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().set_title("Folder FlightDeck watches for screenshots").blocking_pick_folder()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    Ok(Some(path.display().to_string()))
}

/// Set (or with None, stop) the folder watched for screenshots.
#[tauri::command]
fn set_screenshot_folder(state: tauri::State<AppState>, path: Option<String>) -> CmdResult<()> {
    let path = path.map(|p| p.trim().to_string()).unwrap_or_default();
    if !path.is_empty() && !std::path::Path::new(&path).is_dir() {
        return Err(format!("{path} is not a folder on this PC"));
    }
    state.db.lock().unwrap().set_setting(screenshots::KEY_DIR, &path).map_err(err)
}

/// Divert the flight under way to another airport (the plan's destination).
#[tauri::command]
fn divert(state: tauri::State<AppState>, icao: String) -> CmdResult<()> {
    let icao = icao.trim().to_ascii_uppercase();
    let a = state.refdata.airport(&icao).ok_or_else(|| format!("{icao} is not an airport FlightDeck knows"))?;
    send(
        &state,
        Command::Divert(simbrief::PlanAirport { icao: a.ident.clone(), name: a.name.clone(), lat: a.lat, lon: a.lon, runway: String::new() }),
    )
}

/// The runway the pilot means to land on, for the flight under way
/// (changeable at any time; the profile's glideslope uses it).
#[tauri::command]
fn set_landing_runway(state: tauri::State<AppState>, runway: String) -> CmdResult<()> {
    let r = runway.trim().to_ascii_uppercase();
    if r.len() > 4 || !r.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("{r} is not a runway"));
    }
    send(&state, Command::SetLandingRunway(r))
}

/// The nearest airports fit for a diversion from a position.
#[tauri::command]
fn divert_options(state: tauri::State<AppState>, lat: f64, lon: f64, skip: Option<String>) -> Vec<airports::DivertOption> {
    airports::divert_options(&state.refdata, lat, lon, skip.as_deref().unwrap_or(""), 3)
}

/// The latest METAR and TAF at (or near) an airport.
#[tauri::command]
async fn airport_weather(state: tauri::State<'_, AppState>, ident: String) -> CmdResult<Option<airports::Weather>> {
    let rd = state.refdata.clone();
    tauri::async_runtime::spawn_blocking(move || airports::weather(&rd, &ident)).await.map_err(err)?
}

type Parser = fn(&str, &refdata::RefData) -> Result<volanta::ImportedFlight, String>;

fn import_files(app: &tauri::AppHandle, state: &AppState, files: Vec<ImportFile>, source: &str, parse: Parser, force: &[String]) -> ImportReport {
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
        // Already imported: skipped, unless the pilot chose it anyway.
        if !force.contains(&file.name) && matches!(db.find_imported(source, &parsed.id), Ok(Some(_))) {
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

/// One field of a CSV line, quoted when it needs to be.
fn csv_field(v: &str) -> String {
    if v.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// The chosen flights (all if none) as one CSV file, in the column layout
/// flight trackers use for CSV import: origin, destination, departure time,
/// duration, then airline, callsign, flight number, aircraft, route,
/// arrival time, distance and fuel.
fn flights_csv(state: &AppState, ids: &[i64]) -> CmdResult<String> {
    let t = |ms: i64| {
        let s = volanta::format_time(ms, false);
        format!("{} {}", &s[0..10], &s[11..16])
    };
    let mut out = String::from(
        "Origin,Destination,DepartureTime,Duration,Airline,Callsign,FlightNumber,AircraftType,AircraftRegistration,Route,ArrivalTime,Distance,Fuel\r\n",
    );
    let db = state.db.lock().unwrap();
    for id in ids {
        let Some(f) = db.get_flight(*id).map_err(err)? else { continue };
        let plan: Option<simbrief::FlightPlan> = db.get_plan(*id).map_err(err)?.and_then(|j| serde_json::from_str(&j).ok());
        let flight_no = db.flight_no(*id).map_err(err)?;
        let dep_ms = f.block_out.or(f.takeoff).unwrap_or(f.start_ms);
        let arr_ms = f.block_in.or(f.landing).unwrap_or(f.end_ms);
        // The airline: from the plan, else an airline-style callsign (three letters, then the number).
        let cs = f.callsign.trim().to_uppercase();
        let from_callsign = (cs.len() > 3 && cs[..3].chars().all(|c| c.is_ascii_alphabetic()) && cs[3..].starts_with(|c: char| c.is_ascii_digit()))
            .then(|| cs[..3].to_string());
        let airline = plan.as_ref().map(|p| p.airline.clone()).filter(|a| !a.is_empty()).or(from_callsign).unwrap_or_default();
        let number = if !flight_no.is_empty() {
            flight_no
        } else {
            plan.as_ref().map(|p| format!("{}{}", p.airline, p.flight_number)).filter(|n| !n.is_empty()).unwrap_or_default()
        };
        let row = [
            f.dep_icao.clone().unwrap_or_default(),
            f.arr_icao.clone().unwrap_or_default(),
            t(dep_ms),
            ((arr_ms - dep_ms).max(0) / 60_000).to_string(),
            airline,
            f.callsign.clone(),
            number,
            f.icao_type.clone(),
            f.registration.clone(),
            plan.as_ref().map(|p| p.route.clone()).unwrap_or_default(),
            t(arr_ms),
            format!("{:.0}", f.distance_nm),
            format!("{:.0}", f.fuel_used_kg),
        ];
        out.push_str(&row.iter().map(|v| csv_field(v)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    Ok(out)
}

/// Save the chosen flights (all if none) as one CSV file in Downloads.
#[tauri::command]
fn export_csv(app: tauri::AppHandle, state: tauri::State<AppState>, ids: Option<Vec<i64>>) -> CmdResult<ExportAll> {
    let ids = match ids {
        Some(ids) if !ids.is_empty() => ids,
        _ => state.db.lock().unwrap().all_flight_ids().map_err(err)?,
    };
    let stamp = volanta::format_time(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
        false,
    );
    let dir = export_dir(&app, None)?;
    let path = dir.join(format!("FlightDeck flights {} {}{}.csv", &stamp[0..10], &stamp[11..13], &stamp[14..16]));
    std::fs::write(&path, flights_csv(&state, &ids)?).map_err(err)?;
    Ok(ExportAll { folder: path.display().to_string(), count: ids.len() })
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

/// Settings the interface may change (anything else is refused).
const SETTING_KEYS: &[&str] = &[
    "distance_unit",
    "logbook_layout",
    "logbook_hand",
    "logbook_theme",
    "logbook_view",
    "passport_color",
    "map_3d",
    "map_layers",
    "map_airport_detail",
    "map_asp_filter",
    "map_roads_opacity",
    "map_rings_count",
    "map_rings_nm",
    "map_wind_level",
    "map_temp_level",
    "map_icing_fl",
    "show_demo",
    "backup_on_close",
    "simbrief_id",
];

#[tauri::command]
fn set_setting(state: tauri::State<AppState>, key: String, value: String) -> CmdResult<()> {
    if !SETTING_KEYS.contains(&key.as_str()) {
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

/// Everything on the pilot profile page.
#[derive(serde::Serialize)]
struct PilotProfile {
    pilot: Pilot,
    banner: Option<String>,
    totals: Totals,
    stats: store::pilot::PilotStats,
    rating: rating::Profile,
}

/// Work out the handling (and, where there is no landing report, the
/// touchdown) of flights that were imported or recorded before FlightDeck
/// kept handling, so they can be rated. Done once per flight: imported
/// flights from the positions in their original file, the others from
/// their saved track.
fn ensure_estimates(db: &Store, refdata: &refdata::RefData) -> CmdResult<()> {
    // Version 4: the approach and the speed limit.
    db.reset_estimates(4).map_err(err)?;
    for n in db.flights_needing_estimates().map_err(err)? {
        let file = if n.source.is_empty() { None } else { db.get_imported(n.id).map_err(err)?.map(|i| i.file) };
        let from_file = match (n.source.as_str(), &file) {
            (volanta::SOURCE, Some(text)) => volanta::samples(text),
            (elevatex::SOURCE, Some(text)) => elevatex::samples(text),
            _ => vec![],
        };
        let (samples, handling) = if from_file.len() >= 2 {
            let h = tracker::Handling::estimate(&from_file);
            (from_file, h)
        } else {
            let track: Vec<tracker::Sample> = db.get_track(n.id).map_err(err)?.iter().map(tracker::Sample::from_track).collect();
            let h = tracker::Handling::estimate_simplified(&track);
            (track, h)
        };
        let landing = if n.has_landing { None } else { tracker::estimate_landing(&samples) };
        let ground = n.arr_icao.as_deref().and_then(|c| refdata.airport(c)).map(|a| a.elevation_ft as f64);
        let handling = handling.map(|mut h| {
            h.approach = tracker::Handling::estimate_approach(&samples, ground);
            h
        });
        db.save_estimates(n.id, n.start_ms, &handling.unwrap_or_else(tracker::Handling::none), landing.as_ref())
            .map_err(err)?;
    }
    Ok(())
}

/// Every flight's score, oldest first in, newest first out.
fn rating_profile(state: &tauri::State<AppState>) -> CmdResult<rating::Profile> {
    let flights = {
        let db = state.db.lock().unwrap();
        ensure_estimates(&db, &state.refdata)?;
        db.rating_flights().map_err(err)?
    };
    Ok(rating::profile(&flights))
}

#[tauri::command]
fn get_pilot_profile(state: tauri::State<AppState>) -> CmdResult<PilotProfile> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let rating = rating_profile(&state)?;
    let db = state.db.lock().unwrap();
    Ok(PilotProfile {
        pilot: db.pilot().map_err(err)?,
        banner: db.banner().map_err(err)?,
        totals: db.totals().map_err(err)?,
        stats: db.pilot_stats(now).map_err(err)?,
        rating,
    })
}

/// Pictures arrive as data URLs; keep them to a sane size.
fn check_picture(data_url: &Option<String>) -> CmdResult<()> {
    match data_url {
        Some(d) if !d.starts_with("data:image/") => Err("That is not a picture.".into()),
        Some(d) if d.len() > 8 * 1024 * 1024 => Err("That picture is too large.".into()),
        _ => Ok(()),
    }
}

#[tauri::command]
fn set_banner(state: tauri::State<AppState>, data_url: Option<String>) -> CmdResult<()> {
    check_picture(&data_url)?;
    state.db.lock().unwrap().set_banner(data_url.as_deref()).map_err(err)
}

#[tauri::command]
fn set_pilot_photo(state: tauri::State<AppState>, data_url: Option<String>) -> CmdResult<()> {
    check_picture(&data_url)?;
    state.db.lock().unwrap().set_pilot_photo(data_url.as_deref()).map_err(err)
}

#[tauri::command]
fn get_logbook(state: tauri::State<AppState>) -> CmdResult<Vec<LogRow>> {
    let db = state.db.lock().unwrap();
    // Cross-country time follows the definition, whatever path a line came in by.
    let rd = &state.refdata;
    if let Err(e) = db.auto_cross_country(|icao| rd.airport(icao).map(|a| (a.lat, a.lon))) {
        log::warn!("cross-country: {e}");
    }
    db.logbook().map_err(err)
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
fn efb_info(app: tauri::AppHandle) -> efb::EfbInfo {
    efb::info(&app)
}

#[tauri::command]
fn efb_install(app: tauri::AppHandle, folder: Option<String>) -> efb::EfbInfo {
    efb::install(&app, folder)
}

#[tauri::command]
fn efb_uninstall(app: tauri::AppHandle) -> efb::EfbInfo {
    efb::uninstall(&app)
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
        "import_volanta" => ok(to_value(import_volanta(app.clone(), st(), arg(&a, "files")?, arg(&a, "force")?))),
        "preview_import" => ok(to_value(preview_import(st(), arg(&a, "kind")?, arg(&a, "files")?))),
        "import_elevatex" => ok(to_value(import_elevatex(app.clone(), st(), arg(&a, "files")?, arg(&a, "force")?))),
        "demo_routes" => ok(to_value(demo_routes())),
        "airport_at" => ok(to_value(airport_at(st(), arg(&a, "lat")?, arg(&a, "lon")?))),
        "search_airports" => ok(to_value(search_airports(st(), arg(&a, "query")?))),
        "map_airports" => ok(to_value(map_airports(st(), arg(&a, "detail")?))),
        "online_network" => ok(to_value(online_network(arg(&a, "net")?).await?)),
        "wx_categories" => ok(to_value(wx_categories().await?)),
        "wind_grid" => ok(to_value(wind_grid(arg(&a, "level")?).await?)),
        "temperature_grid" => ok(to_value(temperature_grid(arg(&a, "level").ok()).await?)),
        "sky_grid" => ok(to_value(sky_grid(arg(&a, "kind")?).await?)),
        "icing_grid" => ok(to_value(icing_grid(arg(&a, "fl")?).await?)),
        "icing_point" => ok(to_value(icing_point(arg(&a, "lat")?, arg(&a, "lon")?, arg(&a, "fl")?).await?)),
        "navaids" => ok(to_value(navaids(st(), arg(&a, "w")?, arg(&a, "s")?, arg(&a, "e")?, arg(&a, "n")?, arg(&a, "waypoints")?))),
        "navaids_refresh" => ok({
            navaids_refresh(st());
            to_value(())
        }),
        "us_airspace" => ok(to_value(us_airspace(st()).await?)),
        "sua_airspace" => ok(to_value(sua_airspace(st()).await?)),
        "resolve_fixes" => ok(to_value(resolve_fixes(st(), arg(&a, "idents")?).await?)),
        "airport_extras" => ok(to_value(airport_extras(st(), arg(&a, "w")?, arg(&a, "s")?, arg(&a, "e")?, arg(&a, "n")?).await?)),
        "world_airspace" => ok(to_value(world_airspace(st(), arg(&a, "w")?, arg(&a, "s")?, arg(&a, "e")?, arg(&a, "n")?).await?)),
        "pfd_watch" => ok(to_value(pfd_watch(st())?)),
        "us_obstacles" => ok(to_value(us_obstacles(st()).await?)),
        "pln_text" => ok(to_value(pln_text(st(), arg(&a, "route")?)?)),
        "wx_advisories" => ok(to_value(wx_advisories().await?)),
        "online_pilot" => ok(to_value(online_pilot(arg(&a, "net")?, arg(&a, "callsign")?))),
        "flight_screenshots" => ok(to_value(flight_screenshots(st(), arg(&a, "id")?)?)),
        "photos" => ok(to_value(photos(st())?)),
        "delete_screenshot" => ok(to_value(delete_screenshot(st(), arg(&a, "id")?, arg(&a, "file_too")?).await?)),
        "screenshots_since" => ok(to_value(screenshots_since(st(), arg(&a, "sinceMs")?)?)),
        "divert" => ok(to_value(divert(st(), arg(&a, "icao")?)?)),
        "set_landing_runway" => ok(to_value(set_landing_runway(st(), arg(&a, "runway")?)?)),
        "divert_options" => ok(to_value(divert_options(st(), arg(&a, "lat")?, arg(&a, "lon")?, arg(&a, "skip")?))),
        "airport_card" => ok(to_value(airport_card(st(), arg(&a, "ident")?)?)),
        "runway_ends" => ok(to_value(runway_ends(st(), arg(&a, "ident")?))),
        "airport_weather" => ok(to_value(airport_weather(st(), arg(&a, "ident")?).await?)),
        "get_settings" => ok(to_value(get_settings(st())?)),
        "set_setting" => ok(to_value(set_setting(st(), arg(&a, "key")?, arg(&a, "value")?)?)),
        "get_pilot" => ok(to_value(get_pilot(st())?)),
        "set_pilot" => ok(to_value(set_pilot(st(), arg(&a, "pilot")?)?)),
        "get_pilot_profile" => ok(to_value(get_pilot_profile(st())?)),
        "set_banner" => ok(to_value(set_banner(st(), arg(&a, "dataUrl")?)?)),
        "set_pilot_photo" => ok(to_value(set_pilot_photo(st(), arg(&a, "dataUrl")?)?)),
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
        .plugin(tauri_plugin_dialog::init())
        // Closing: refresh the backup folder first, if the pilot asked.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    backup::on_close(&window.state::<AppState>().db);
                }
            }
        })
        // Screenshots are read from where they are on this PC, by their id:
        // only pictures FlightDeck noted can be reached this way.
        .register_asynchronous_uri_scheme_protocol("fdshot", |ctx, request, responder| {
            let db = ctx.app_handle().state::<AppState>().db.clone();
            // "/<id>" the picture, "/<id>/thumb" its small copy.
            let path = request.uri().path().trim_matches('/').to_string();
            let (id, small) = match path.strip_suffix("/thumb") {
                Some(i) => (i.parse::<i64>().ok(), true),
                None => (path.parse::<i64>().ok(), false),
            };
            std::thread::spawn(move || {
                let found = id.and_then(|id| if small { screenshots::thumb(&db, id).map(|t| (t.to_vec(), "image/jpeg")) } else { screenshots::read(&db, id) });
                let res = match found {
                    Some((bytes, kind)) => tauri::http::Response::builder()
                        .header("Content-Type", kind)
                        .header("Cache-Control", "max-age=3600")
                        .body(bytes),
                    None => tauri::http::Response::builder().status(404).body(Vec::new()),
                };
                responder.respond(res.unwrap_or_default());
            });
        })
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            diag::set_folder(&data_dir);
            backup::apply_staged(&data_dir);
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
            let nav: navaids::Shared = Default::default();
            {
                let (nav, dir) = (nav.clone(), data_dir.clone());
                let _ = std::thread::Builder::new().name("navaids-load".into()).spawn(move || nav.lock().unwrap().load(&dir));
            }
            engine::spawn(app.handle().clone(), shared.clone(), db.clone(), refdata, data_dir.clone(), nav.clone(), rx);
            screenshots::spawn(app.handle().clone(), db.clone());
            app.manage(AppState { db, refdata: refdata_for_state, shared, tx: Mutex::new(tx), data_dir, photos: Default::default(), nav });
            app.manage(remote::Remote::default());
            remote::init(app.handle());
            efb::update_if_installed(app.handle());
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
            preview_import,
            export_csv,
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
            airport_at,
            get_settings,
            set_setting,
            get_pilot,
            get_pilot_profile,
            set_banner,
            set_pilot_photo,
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
            app_version,
            map_airports,
            online_network,
            wx_categories,
            wind_grid,
            temperature_grid,
            sky_grid,
            icing_grid,
            icing_point,
            navaids,
            navaids_refresh,
            us_airspace,
            sua_airspace,
            world_airspace,
            airport_extras,
            resolve_fixes,
            pfd_watch,
            us_obstacles,
            pln_text,
            save_pln,
            wx_advisories,
            online_pilot,
            airport_card,
            runway_ends,
            divert_options,
            divert,
            set_landing_runway,
            flight_screenshots,
            photos,
            delete_screenshot,
            screenshots_since,
            pick_screenshot_folder,
            backup_save,
            backup_choose_folder,
            backup_forget_folder,
            backup_now,
            backup_restore_pick,
            backup_restore_cancel,
            restart_app,
            set_screenshot_folder,
            airport_weather,
            efb_info,
            efb_install,
            efb_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlightDeck");
}

#[cfg(test)]
mod csv {
    #[test]
    fn fields_are_quoted_when_needed() {
        assert_eq!(super::csv_field("KSEA"), "KSEA");
        assert_eq!(super::csv_field("A, B"), "\"A, B\"");
        assert_eq!(super::csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}

#[cfg(test)]
mod setting_keys {
    /// Every setting the interface saves with setPref must be allowed here,
    /// or it is silently lost when the app closes.
    #[test]
    fn interface_settings_are_allowed() {
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "svelte" || x == "ts") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    for part in text.split("setPref(\"").skip(1) {
                        if let Some(end) = part.find('"') {
                            out.push(part[..end].to_string());
                        }
                    }
                }
            }
        }
        let mut keys = vec![];
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src"), &mut keys);
        assert!(keys.len() > 5);
        for k in keys {
            assert!(super::SETTING_KEYS.contains(&k.as_str()), "setting {k} is saved by the interface but not allowed");
        }
    }
}
