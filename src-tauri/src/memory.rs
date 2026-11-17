//! Memory: how much FlightDeck uses (shown in Settings → Updates and
//! diagnostics), and giving memory back while the window is minimized.
//!
//! FlightDeck is two parts: this core process, and the window, which is
//! drawn by Windows' web engine (WebView2) in its own group of processes.
//! While FlightDeck is minimized nothing in the window is looked at, so
//! WebView2 is asked to keep as little as it can (its "low" memory target:
//! caches dropped, unused memory handed back); restoring the window puts
//! it back to normal. Recording is done here in the core and is not
//! affected.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default, Serialize)]
pub struct Usage {
    /// This process (flightdeck.exe), MB.
    pub core_mb: Option<f64>,
    /// Every WebView2 process of the window together, MB.
    pub window_mb: Option<f64>,
    /// How many WebView2 processes that is.
    pub window_processes: u32,
    /// The window is in its low-memory state (minimized).
    pub low: bool,
}

static LOW: AtomicBool = AtomicBool::new(false);

const MB: f64 = 1024.0 * 1024.0;

#[cfg(windows)]
mod win {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment8, ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows::core::Interface;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    /// Private bytes, as Task Manager's memory column roughly shows.
    fn private_bytes(h: HANDLE) -> Option<u64> {
        let mut c = PROCESS_MEMORY_COUNTERS_EX { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32, ..Default::default() };
        // SAFETY: `c` is a valid, correctly sized counters struct for this call.
        unsafe { GetProcessMemoryInfo(h, &mut c as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS, c.cb) }.ok()?;
        Some(c.PrivateUsage as u64)
    }

    pub fn core_bytes() -> Option<u64> {
        // SAFETY: the pseudo handle of this process needs no closing.
        private_bytes(unsafe { GetCurrentProcess() })
    }

    pub fn pid_bytes(pid: u32) -> Option<u64> {
        // SAFETY: the handle is closed below.
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let r = private_bytes(h);
        // SAFETY: `h` came from OpenProcess.
        let _ = unsafe { CloseHandle(h) };
        r
    }

    /// The ids of the window's WebView2 processes (run on the main thread).
    pub fn webview_pids(wv: &tauri::webview::PlatformWebview) -> Vec<u32> {
        // SAFETY: COM calls on the WebView2 objects, on the thread that owns them.
        unsafe {
            let Ok(env) = wv.environment().cast::<ICoreWebView2Environment8>() else { return vec![] };
            let Ok(list) = env.GetProcessInfos() else { return vec![] };
            let mut n = 0u32;
            if list.Count(&mut n).is_err() {
                return vec![];
            }
            (0..n)
                .filter_map(|i| {
                    let p = list.GetValueAtIndex(i).ok()?;
                    let mut id = 0i32;
                    p.ProcessId(&mut id).ok()?;
                    u32::try_from(id).ok()
                })
                .collect()
        }
    }

    /// WebView2's memory target: low while minimized, normal otherwise.
    pub fn set_low(wv: &tauri::webview::PlatformWebview, low: bool) {
        // SAFETY: COM calls on the WebView2 objects, on the thread that owns them.
        unsafe {
            let Ok(core) = wv.controller().CoreWebView2() else { return };
            if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
                let level = if low { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL };
                let _ = c19.SetMemoryUsageTargetLevel(level);
            }
        }
    }
}

/// This process's memory (resident on other systems, for development).
fn core_bytes() -> Option<u64> {
    #[cfg(windows)]
    {
        win::core_bytes()
    }
    #[cfg(not(windows))]
    {
        let s = std::fs::read_to_string("/proc/self/statm").ok()?;
        let pages: u64 = s.split_whitespace().nth(1)?.parse().ok()?;
        Some(pages * 4096)
    }
}

/// What FlightDeck uses now. Call from an async command (it waits for the
/// main thread to list the window's processes).
pub fn usage(app: &tauri::AppHandle) -> Usage {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut u = Usage { core_mb: core_bytes().map(|b| b as f64 / MB), low: LOW.load(Ordering::Relaxed), ..Default::default() };
    #[cfg(windows)]
    {
        use tauri::Manager;
        if let Some(w) = app.get_webview_window("main") {
            let (tx, rx) = std::sync::mpsc::channel();
            if w.with_webview(move |wv| {
                let _ = tx.send(win::webview_pids(&wv));
            })
            .is_ok()
            {
                if let Ok(pids) = rx.recv_timeout(std::time::Duration::from_secs(3)) {
                    let bytes: Vec<u64> = pids.iter().filter_map(|p| win::pid_bytes(*p)).collect();
                    if !bytes.is_empty() {
                        u.window_processes = bytes.len() as u32;
                        u.window_mb = Some(bytes.iter().sum::<u64>() as f64 / MB);
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = app;
    u
}

/// The main window was resized: minimized → low memory, restored → normal.
pub fn on_resize(window: &tauri::Window) {
    let min = window.is_minimized().unwrap_or(false);
    if LOW.swap(min, Ordering::Relaxed) == min {
        return;
    }
    log::info!("window {}: WebView2 memory target {}", if min { "minimized" } else { "restored" }, if min { "low" } else { "normal" });
    #[cfg(windows)]
    {
        use tauri::Manager;
        if let Some(w) = window.app_handle().get_webview_window(window.label()) {
            let _ = w.with_webview(move |wv| win::set_low(&wv, min));
        }
    }
}
