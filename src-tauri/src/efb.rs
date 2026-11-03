//! FlightDeck's app for the MSFS 2024 EFB (the tablet in the cockpit).
//!
//! The app is a Community package, `flightdeck-efb`, shipped inside
//! FlightDeck. Installing writes it into the sim's Community folder, found
//! through the sim's UserCfg.opt (its `InstalledPackagesPath` line), and
//! turns on FlightDeck's local server for it; uninstalling removes only that
//! folder. The app reaches FlightDeck on 127.0.0.1, this PC only.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use serde_json::json;
use tauri::AppHandle;

use crate::remote;

/// The package's folder name in Community.
pub const PACKAGE: &str = "flightdeck-efb";
/// A Community folder chosen by hand, when the sim's settings do not say.
pub const KEY_FOLDER: &str = "efb_community";
const VERSION_MARK: &str = "__FD_VERSION__";

const APP_DIR: &str = "html_ui/efb_ui/efb_apps/FlightDeck";
const MANIFEST: &str = include_str!("../../efb/flightdeck-efb/manifest.json");
/// (path in the package, contents)
const FILES: &[(&str, &str)] = &[
    ("FlightDeck.js", include_str!("../../efb/flightdeck-efb/html_ui/efb_ui/efb_apps/FlightDeck/FlightDeck.js")),
    ("Assets/app-icon.svg", include_str!("../../efb/flightdeck-efb/html_ui/efb_ui/efb_apps/FlightDeck/Assets/app-icon.svg")),
    ("Assets/bird-dark.svg", include_str!("../../efb/flightdeck-efb/html_ui/efb_ui/efb_apps/FlightDeck/Assets/bird-dark.svg")),
];

#[derive(Debug, Clone, Serialize)]
pub struct EfbInfo {
    pub installed: bool,
    /// FlightDeck version of the installed app.
    pub version: Option<String>,
    /// The Community folder in use, if found.
    pub community: Option<String>,
    /// The folder was typed in rather than found.
    pub manual: bool,
    pub error: Option<String>,
}

/// Where MSFS 2024 keeps UserCfg.opt: the Microsoft Store and Steam versions.
fn usercfg_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        out.push(PathBuf::from(local).join(r"Packages\Microsoft.Limitless_8wekyb3d8bbwe\LocalCache\UserCfg.opt"));
    }
    if let Some(roaming) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(roaming).join(r"Microsoft Flight Simulator 2024\UserCfg.opt"));
    }
    out
}

/// The packages folder from a UserCfg.opt.
fn packages_path(usercfg: &str) -> Option<PathBuf> {
    usercfg.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("InstalledPackagesPath")?;
        let value = rest.trim().trim_matches('"').trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

/// The sim's Community folder, from its own settings.
fn found_community() -> Option<PathBuf> {
    usercfg_paths().iter().find_map(|cfg| {
        let text = std::fs::read_to_string(cfg).ok()?;
        let dir = packages_path(&text)?.join("Community");
        dir.is_dir().then_some(dir)
    })
}

/// The Community folder to use, and whether it was chosen by hand.
fn community(app: &AppHandle) -> Option<(PathBuf, bool)> {
    if let Some(dir) = remote::setting(app, KEY_FOLDER).filter(|s| !s.trim().is_empty()) {
        let dir = PathBuf::from(dir.trim());
        if dir.is_dir() {
            return Some((dir, true));
        }
    }
    found_community().map(|d| (d, false))
}

/// The version of our package in a Community folder, if it is ours.
fn installed_version(community: &Path) -> Option<String> {
    let text = std::fs::read_to_string(community.join(PACKAGE).join("manifest.json")).ok()?;
    let m: serde_json::Value = serde_json::from_str(&text).ok()?;
    (m["creator"] == "FlightDeck" && m["title"] == "FlightDeck").then(|| m["package_version"].as_str().unwrap_or("").to_string())
}

fn filetime(path: &Path) -> u64 {
    let secs = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    ((secs + 11_644_473_600.0) * 10_000_000.0) as u64
}

/// Writes the package into `community`, replacing an older copy of it.
pub fn write_package(community: &Path, version: &str) -> std::io::Result<()> {
    let root = community.join(PACKAGE);
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    let mut content = Vec::new();
    for (rel, text) in FILES {
        let rel = format!("{APP_DIR}/{rel}");
        let path = root.join(&rel);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, text.replace(VERSION_MARK, version))?;
        content.push(json!({ "path": rel, "size": std::fs::metadata(&path)?.len(), "date": filetime(&path) }));
    }
    std::fs::write(root.join("manifest.json"), MANIFEST.replace(VERSION_MARK, version))?;
    let layout = serde_json::to_string_pretty(&json!({ "content": content })).unwrap_or_default();
    std::fs::write(root.join("layout.json"), layout + "\n")?;
    Ok(())
}

pub fn info(app: &AppHandle) -> EfbInfo {
    match community(app) {
        Some((dir, manual)) => {
            let version = installed_version(&dir);
            EfbInfo { installed: version.is_some(), version, community: Some(dir.display().to_string()), manual, error: None }
        }
        None => EfbInfo {
            installed: false,
            version: None,
            community: None,
            manual: false,
            error: Some("MSFS 2024's Community folder was not found. Choose it below.".into()),
        },
    }
}

pub fn install(app: &AppHandle, folder: Option<String>) -> EfbInfo {
    if let Some(f) = folder.map(|f| f.trim().trim_matches('"').to_string()).filter(|f| !f.is_empty()) {
        if !Path::new(&f).is_dir() {
            return EfbInfo { error: Some(format!("There is no folder {f}.")), ..info(app) };
        }
        remote::set(app, KEY_FOLDER, &f);
    }
    let Some((dir, _)) = community(app) else { return info(app) };
    let version = app.package_info().version.to_string();
    if let Err(e) = write_package(&dir, &version) {
        log::warn!("EFB app install into {}: {e}", dir.display());
        return EfbInfo { error: Some(format!("Could not write to {}: {e}", dir.display())), ..info(app) };
    }
    log::info!("EFB app {version} installed in {}", dir.display());
    remote::set(app, remote::KEY_EFB, "1");
    remote::refresh(app);
    info(app)
}

pub fn uninstall(app: &AppHandle) -> EfbInfo {
    if let Some((dir, _)) = community(app) {
        // Only ever our own folder.
        if installed_version(&dir).is_some() {
            if let Err(e) = std::fs::remove_dir_all(dir.join(PACKAGE)) {
                return EfbInfo { error: Some(format!("Could not remove the EFB app: {e}. Is MSFS running?")), ..info(app) };
            }
            log::info!("EFB app removed from {}", dir.display());
        }
    }
    remote::set(app, remote::KEY_EFB, "0");
    remote::refresh(app);
    info(app)
}

/// At start: bring an installed app up to this version of FlightDeck.
pub fn update_if_installed(app: &AppHandle) {
    if remote::setting(app, remote::KEY_EFB).as_deref() != Some("1") {
        return;
    }
    let Some((dir, _)) = community(app) else { return };
    let version = app.package_info().version.to_string();
    match installed_version(&dir) {
        Some(v) if v == version => {}
        Some(_) => match write_package(&dir, &version) {
            Ok(()) => log::info!("EFB app updated to {version}"),
            Err(e) => log::warn!("EFB app update: {e}"),
        },
        // Removed by hand: stop serving it.
        None => {
            remote::set(app, remote::KEY_EFB, "0");
            remote::refresh(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_packages_path() {
        let cfg = "{Graphics\n}\nInstalledPackagesPath \"D:\\MSFS 2024\\Packages\"\nOther 1\n";
        assert_eq!(packages_path(cfg), Some(PathBuf::from("D:\\MSFS 2024\\Packages")));
        assert_eq!(packages_path("Nothing here\n"), None);
        assert_eq!(packages_path("InstalledPackagesPath \"\"\n"), None);
    }

    #[test]
    fn writes_a_package_msfs_can_load() {
        let dir = std::env::temp_dir().join(format!("fd-efb-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_package(&dir, "9.9.9").unwrap();
        assert_eq!(installed_version(&dir).as_deref(), Some("9.9.9"));
        let root = dir.join(PACKAGE);
        let js = std::fs::read_to_string(root.join(APP_DIR).join("FlightDeck.js")).unwrap();
        assert!(js.contains("'9.9.9'") && !js.contains(VERSION_MARK));
        let layout: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("layout.json")).unwrap()).unwrap();
        let content = layout["content"].as_array().unwrap();
        assert_eq!(content.len(), FILES.len());
        for c in content {
            let p = root.join(c["path"].as_str().unwrap());
            assert_eq!(c["size"].as_u64().unwrap(), std::fs::metadata(&p).unwrap().len());
            assert!(c["date"].as_u64().unwrap() > 130_000_000_000_000_000);
        }
        // A second install replaces the first.
        write_package(&dir, "9.9.10").unwrap();
        assert_eq!(installed_version(&dir).as_deref(), Some("9.9.10"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leaves_other_packages_alone() {
        let dir = std::env::temp_dir().join(format!("fd-efb-other-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(PACKAGE)).unwrap();
        std::fs::write(dir.join(PACKAGE).join("manifest.json"), r#"{"title":"Something else","creator":"Someone"}"#).unwrap();
        assert_eq!(installed_version(&dir), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
