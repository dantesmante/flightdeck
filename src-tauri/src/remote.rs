//! Access from other devices on the home network: a small web server inside
//! FlightDeck that serves the same interface to a browser on a laptop,
//! tablet or phone, with live updates.
//!
//! Local only, by design:
//! - It runs only while "Allow access from other devices" is on.
//! - It answers only addresses on private networks (home Wi-Fi), never the
//!   internet.
//! - A device must first be paired with the code shown on the PC; it then
//!   holds a token. Pairings can be forgotten at any time.
//! - Devices can view; editing (and flight controls) needs "Other devices
//!   can edit". Things that touch the PC itself (updates, exports into its
//!   folders, diagnostics) stay on the PC.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Listener, Manager};
use tokio::sync::{broadcast, oneshot};

use crate::AppState;

/// The port FlightDeck listens on for other devices.
pub const PORT: u16 = 8420;

/// Events forwarded to browsers as they happen.
const EVENTS: &[&str] = &["status", "flights-changed", "flight-saved", "touchdown"];

/// Settings keys (stored with the other settings).
pub const KEY_ENABLED: &str = "remote_enabled";
pub const KEY_EDIT: &str = "remote_edit";
const KEY_DEVICES: &str = "remote_devices";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub token: String,
    pub name: String,
    pub paired_ms: i64,
}

/// What the PC's Settings shows about access from other devices.
#[derive(Debug, Clone, Serialize)]
pub struct RemoteInfo {
    pub enabled: bool,
    pub running: bool,
    pub can_edit: bool,
    pub port: u16,
    /// Addresses to type into a browser, e.g. "http://192.168.1.20:8420".
    pub urls: Vec<String>,
    /// The first address as a QR code (SVG), for phones and tablets.
    pub qr_svg: Option<String>,
    pub code: String,
    pub devices: Vec<DeviceInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub paired_ms: i64,
}

/// The server's state, kept in the app.
#[derive(Default)]
pub struct Remote {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    stop: Option<oneshot::Sender<()>>,
    code: String,
    error: Option<String>,
    /// Failed pairing attempts per address, to slow down guessing.
    attempts: HashMap<IpAddr, (u32, Instant)>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    if getrandom::fill(&mut b).is_err() {
        // Extremely unlikely; fall back to time-based noise.
        let t = now_ms() as u64 ^ (std::process::id() as u64).rotate_left(17);
        for (i, x) in b.iter_mut().enumerate() {
            *x = (t.rotate_left((i * 7 % 64) as u32) & 0xff) as u8;
        }
    }
    b
}

fn new_code() -> String {
    let b = random_bytes::<4>();
    format!("{:06}", u32::from_le_bytes(b) % 1_000_000)
}

fn new_token() -> String {
    random_bytes::<32>().iter().map(|b| format!("{b:02x}")).collect()
}

/// Home-network addresses only: private IPv4 ranges, link-local, loopback
/// and IPv6 unique-local. Anything from the internet is refused.
fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return v4.is_private() || v4.is_loopback() || v4.is_link_local();
            }
            let first = v6.segments()[0];
            v6.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let st = app.state::<AppState>();
    let db = st.db.lock().unwrap();
    db.settings().ok().and_then(|s| s.get(key).cloned())
}

fn set(app: &AppHandle, key: &str, value: &str) {
    let st = app.state::<AppState>();
    let db = st.db.lock().unwrap();
    let _ = db.set_setting(key, value);
}

pub fn devices(app: &AppHandle) -> Vec<Device> {
    setting(app, KEY_DEVICES).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_devices(app: &AppHandle, list: &[Device]) {
    set(app, KEY_DEVICES, &serde_json::to_string(list).unwrap_or_else(|_| "[]".into()));
}

fn can_edit(app: &AppHandle) -> bool {
    setting(app, KEY_EDIT).as_deref() == Some("1")
}

/// This PC's addresses on the home network.
fn local_urls() -> Vec<String> {
    let mut out: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            IpAddr::V4(v4) if v4.is_private() => Some(format!("http://{v4}:{PORT}")),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn qr(url: &str) -> Option<String> {
    let code = qrcode::QrCode::new(url.as_bytes()).ok()?;
    Some(
        code.render::<qrcode::render::svg::Color>()
            .min_dimensions(180, 180)
            .dark_color(qrcode::render::svg::Color("#111214"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build(),
    )
}

pub fn info(app: &AppHandle) -> RemoteInfo {
    let remote = app.state::<Remote>();
    let inner = remote.inner.lock().unwrap();
    let urls = local_urls();
    RemoteInfo {
        enabled: setting(app, KEY_ENABLED).as_deref() == Some("1"),
        running: inner.stop.is_some(),
        can_edit: can_edit(app),
        port: PORT,
        qr_svg: urls.first().and_then(|u| qr(u)),
        urls,
        code: inner.code.clone(),
        devices: devices(app).into_iter().map(|d| DeviceInfo { name: d.name, paired_ms: d.paired_ms }).collect(),
        error: inner.error.clone(),
    }
}

/// Turn the server on or off (and remember the choice).
pub fn set_enabled(app: &AppHandle, on: bool) {
    set(app, KEY_ENABLED, if on { "1" } else { "0" });
    if on {
        start(app.clone());
    } else {
        stop(app);
    }
}

pub fn set_edit(app: &AppHandle, on: bool) {
    set(app, KEY_EDIT, if on { "1" } else { "0" });
    log::info!("other devices can {}", if on { "edit" } else { "only view" });
}

pub fn new_pairing_code(app: &AppHandle) {
    app.state::<Remote>().inner.lock().unwrap().code = new_code();
}

pub fn forget_devices(app: &AppHandle) {
    save_devices(app, &[]);
    log::info!("all paired devices forgotten");
}

fn stop(app: &AppHandle) {
    if let Some(tx) = app.state::<Remote>().inner.lock().unwrap().stop.take() {
        let _ = tx.send(());
        log::info!("access from other devices stopped");
    }
}

/// Start the server if it is switched on (at launch, or when turned on).
pub fn start_if_enabled(app: &AppHandle) {
    if setting(app, KEY_ENABLED).as_deref() == Some("1") {
        start(app.clone());
    }
}

#[derive(Clone)]
struct Ctx {
    app: AppHandle,
    events: broadcast::Sender<String>,
}

fn start(app: AppHandle) {
    let remote = app.state::<Remote>();
    {
        let mut inner = remote.inner.lock().unwrap();
        if inner.stop.is_some() {
            return;
        }
        if inner.code.is_empty() {
            inner.code = new_code();
        }
        inner.error = None;
    }
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    remote.inner.lock().unwrap().stop = Some(stop_tx);

    // Forward the app's live events to connected browsers.
    let (events, _) = broadcast::channel::<String>(256);
    let mut ids = Vec::new();
    for name in EVENTS {
        let tx = events.clone();
        let n = (*name).to_string();
        ids.push(app.listen_any(*name, move |e| {
            let payload: Value = serde_json::from_str(e.payload()).unwrap_or(Value::Null);
            let _ = tx.send(json!({ "event": n, "payload": payload }).to_string());
        }));
    }

    let ctx = Ctx { app: app.clone(), events };
    let api = Router::new()
        .route("/api/session", get(session))
        .route("/api/pair", post(pair))
        .route("/api/call/{cmd}", post(call))
        .route("/api/events", get(events_ws))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024));
    let router = Router::new()
        .merge(api)
        .fallback(get(asset))
        .layer(middleware::from_fn(local_only))
        .with_state(ctx);

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], PORT))).await {
            Ok(l) => l,
            Err(e) => {
                let msg = format!("Could not open port {PORT}: {e}");
                log::warn!("{msg}");
                let remote = app2.state::<Remote>();
                let mut inner = remote.inner.lock().unwrap();
                inner.error = Some(msg);
                inner.stop = None;
                return;
            }
        };
        log::info!("access from other devices on port {PORT}: {:?}", local_urls());
        let server = axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move {
                let _ = stop_rx.await;
            });
        if let Err(e) = server.await {
            log::warn!("remote server stopped: {e}");
        }
        for id in ids {
            app2.unlisten(id);
        }
    });
}

/// Refuse anything that is not from the home network.
async fn local_only(ConnectInfo(addr): ConnectInfo<SocketAddr>, req: Request<Body>, next: Next) -> Response {
    if !is_local(addr.ip()) {
        log::warn!("refused a connection from outside the home network: {}", addr.ip());
        return (StatusCode::FORBIDDEN, "FlightDeck only answers devices on your home network.").into_response();
    }
    next.run(req).await
}

fn token_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string)
}

fn paired(app: &AppHandle, token: Option<&str>) -> bool {
    let Some(t) = token.filter(|t| t.len() == 64) else { return false };
    devices(app).iter().any(|d| d.token == t)
}

async fn session(State(ctx): State<Ctx>, headers: HeaderMap) -> Json<Value> {
    let ok = paired(&ctx.app, token_of(&headers).as_deref());
    Json(json!({
        "paired": ok,
        "can_edit": ok && can_edit(&ctx.app),
        "version": ctx.app.package_info().version.to_string(),
    }))
}

#[derive(Deserialize)]
struct PairReq {
    code: String,
    #[serde(default)]
    name: String,
}

async fn pair(State(ctx): State<Ctx>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Json(req): Json<PairReq>) -> Response {
    let remote = ctx.app.state::<Remote>();
    {
        let mut inner = remote.inner.lock().unwrap();
        // Five tries a minute per device.
        let entry = inner.attempts.entry(addr.ip()).or_insert((0, Instant::now()));
        if entry.1.elapsed() > Duration::from_secs(60) {
            *entry = (0, Instant::now());
        }
        if entry.0 >= 5 {
            return (StatusCode::TOO_MANY_REQUESTS, "Too many tries. Wait a minute and try again.").into_response();
        }
        let good = !inner.code.is_empty() && req.code.trim() == inner.code;
        if !good {
            inner.attempts.get_mut(&addr.ip()).unwrap().0 += 1;
            return (StatusCode::FORBIDDEN, "That code is not right. Check Settings on the flight sim PC.").into_response();
        }
        // Each code pairs one device.
        inner.code = new_code();
        inner.attempts.remove(&addr.ip());
    }
    let token = new_token();
    let mut list = devices(&ctx.app);
    let name = if req.name.trim().is_empty() { format!("Device at {}", addr.ip()) } else { req.name.trim().chars().take(60).collect() };
    list.push(Device { token: token.clone(), name: name.clone(), paired_ms: now_ms() });
    save_devices(&ctx.app, &list);
    log::info!("paired a new device: {name}");
    Json(json!({ "token": token })).into_response()
}

#[derive(Deserialize)]
struct WsQuery {
    token: Option<String>,
}

async fn events_ws(State(ctx): State<Ctx>, Query(q): Query<WsQuery>, ws: WebSocketUpgrade) -> Response {
    if !paired(&ctx.app, q.token.as_deref()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let rx = ctx.events.subscribe();
    ws.on_upgrade(move |socket| forward(socket, rx))
}

async fn forward(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    loop {
        tokio::select! {
            msg = rx.recv() => match msg {
                Ok(text) => {
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
}

/// Commands a viewing device may run.
const VIEW: &[&str] = &[
    "get_status", "list_flights", "get_totals", "get_flight", "current_briefing", "demo_routes", "get_settings",
    "get_pilot", "get_logbook", "find_duplicates", "aircraft_with_registration", "get_passport", "get_hangar",
    "get_routes", "spotter_photo", "aircraft_flights", "get_aircraft_photo", "app_version", "simbrief_latest",
];
/// Commands that change something: only when editing is allowed.
const EDIT: &[&str] = &[
    "delete_flight", "start_demo", "stop_demo", "end_flight", "cancel_flight", "start_flight", "set_plan", "set_setting",
    "set_pilot", "update_log_entry", "update_flight", "add_manual_entry", "update_aircraft", "add_aircraft",
    "delete_aircraft", "delete_flights", "set_aircraft_favorite", "set_aircraft_photo", "import_volanta",
    "import_elevatex", "add_sample_flights", "remove_sample_flights",
];

fn fail(code: StatusCode, msg: &str) -> Response {
    (code, Json(json!({ "error": msg }))).into_response()
}

async fn call(State(ctx): State<Ctx>, headers: HeaderMap, Path(cmd): Path<String>, body: Option<Json<Value>>) -> Response {
    let app = &ctx.app;
    if !paired(app, token_of(&headers).as_deref()) {
        return fail(StatusCode::UNAUTHORIZED, "This device is not paired with FlightDeck.");
    }
    let args = body.map(|b| b.0).unwrap_or(Value::Null);
    if EDIT.contains(&cmd.as_str()) {
        if !can_edit(app) {
            return fail(
                StatusCode::FORBIDDEN,
                "This device can only view. To make changes from here, turn on \"Other devices can edit\" in Settings on the flight sim PC.",
            );
        }
        // Access settings are only ever changed on the PC itself.
        if cmd == "set_setting" && args.get("key").and_then(Value::as_str).is_some_and(|k| k.starts_with("remote_")) {
            return fail(StatusCode::FORBIDDEN, "Change access from other devices on the flight sim PC.");
        }
    } else if !VIEW.contains(&cmd.as_str()) {
        return fail(StatusCode::FORBIDDEN, "That can only be done on the flight sim PC.");
    }
    match crate::dispatch(app, &cmd, args).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => fail(StatusCode::BAD_REQUEST, &e),
    }
}

/// The interface itself: the same files the desktop window shows.
async fn asset(req: Request<Body>) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() || !path.contains('.') { "index.html" } else { path };
    match load_asset(path) {
        Some((bytes, mime)) => {
            let bytes = if path == "index.html" {
                // Tell the interface it is running in a browser on another device.
                let html = String::from_utf8_lossy(&bytes).replacen(
                    "<head>",
                    "<head><script>window.__FD_REMOTE__=true</script><meta name=\"apple-mobile-web-app-capable\" content=\"yes\">",
                    1,
                );
                html.into_bytes()
            } else {
                bytes
            };
            let cache = if path == "index.html" { "no-cache" } else { "public, max-age=31536000, immutable" };
            ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache.to_string())], bytes).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

fn load_asset(path: &str) -> Option<(Vec<u8>, String)> {
    let app = APP.get()?;
    if let Some(a) = app.asset_resolver().get(path.to_string()) {
        return Some((a.bytes().to_vec(), a.mime_type().to_string()));
    }
    // Development builds load the interface from the dev server; serve the
    // built files from disk instead.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/dist");
    let file = dir.join(path);
    if !file.starts_with(&dir) || path.contains("..") {
        return None;
    }
    let bytes = std::fs::read(&file).ok()?;
    let mime = match file.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html",
        "js" => "text/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "json" => "application/json",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };
    Some((bytes, mime.to_string()))
}
