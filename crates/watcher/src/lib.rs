//! Sim Watcher: finds running simulators by process name.
//!
//! Only process names are read (never memory), every few seconds, so the app
//! costs almost nothing while no sim is open.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use telemetry::SimKind;

/// A simulator seen in the process list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detected {
    /// MSFS; the exact version is confirmed by SimConnect after connecting.
    Msfs(SimKind),
    /// X-Plane; the version is confirmed by its own API (Milestone 2).
    XPlane,
}

/// Classify one executable name (case-insensitive).
pub fn classify(process_name: &str) -> Option<Detected> {
    let n = process_name.to_ascii_lowercase();
    let n = n.strip_suffix(".exe").unwrap_or(&n);
    match n {
        // MSFS 2024 executable name; confirm on a test install (see README).
        "flightsimulator2024" => Some(Detected::Msfs(SimKind::Msfs2024)),
        "flightsimulator" => Some(Detected::Msfs(SimKind::Msfs2020)),
        "x-plane" | "x-plane-x86_64" => Some(Detected::XPlane),
        _ => None,
    }
}

pub struct Watcher {
    sys: System,
}

impl Default for Watcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Watcher {
    pub fn new() -> Self {
        Watcher { sys: System::new_with_specifics(RefreshKind::nothing()) }
    }

    /// Refresh the process list (names only) and return the sims found.
    pub fn scan(&mut self) -> Vec<Detected> {
        self.sys
            .refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let mut found: Vec<Detected> = self
            .sys
            .processes()
            .values()
            .filter_map(|p| classify(&p.name().to_string_lossy()))
            .collect();
        found.sort_by_key(|d| format!("{d:?}"));
        found.dedup();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(classify("FlightSimulator.exe"), Some(Detected::Msfs(SimKind::Msfs2020)));
        assert_eq!(classify("FlightSimulator2024.exe"), Some(Detected::Msfs(SimKind::Msfs2024)));
        assert_eq!(classify("X-Plane.exe"), Some(Detected::XPlane));
        assert_eq!(classify("notepad.exe"), None);
    }

    #[test]
    fn scan_runs() {
        // No sim runs on a build machine; this just checks the call works.
        let _ = Watcher::new().scan();
    }
}
