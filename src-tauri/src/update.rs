//! Self-update from GitHub Releases.
//!
//! Every push to the main branch builds a new installer and publishes it as
//! the latest release (see .github/workflows/release.yml). FlightDeck asks
//! GitHub for the latest release; if it is newer, it can download the
//! installer and run it. Flights live in the app-data folder, which the
//! installer never touches.

use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The repository FlightDeck updates from.
pub const REPO: &str = "dantesmante/flightdeck";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub notes: String,
    pub download_url: Option<String>,
    pub size_mb: f64,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// "v0.1.12" or "0.1.12" -> [0, 1, 12]
pub fn parse_version(s: &str) -> Vec<u64> {
    s.trim().trim_start_matches(['v', 'V']).split(['.', '-', '+']).map_while(|p| p.parse().ok()).collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let (a, b) = (parse_version(latest), parse_version(current));
    !a.is_empty() && a > b
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .user_agent(concat!("FlightDeck/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

pub fn check(current: &str) -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let mut res = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;
    let rel: Release = res.body_mut().read_json().map_err(|e| format!("Unexpected answer from GitHub: {e}"))?;
    let asset = rel.assets.iter().find(|a| a.name.to_ascii_lowercase().ends_with("-setup.exe"));
    let latest = rel.tag_name.trim_start_matches(['v', 'V']).to_string();
    Ok(UpdateInfo {
        available: is_newer(&latest, current) && asset.is_some(),
        current: current.to_string(),
        notes: rel.body.unwrap_or_default(),
        download_url: asset.map(|a| a.browser_download_url.clone()),
        size_mb: asset.map(|a| a.size as f64 / 1_048_576.0).unwrap_or(0.0),
        latest,
    })
}

/// Download the installer to the temp folder and return its path.
pub fn download(url: &str) -> Result<PathBuf, String> {
    if !url.starts_with(&format!("https://github.com/{REPO}/releases/download/")) {
        return Err("Refusing to download an installer from outside the FlightDeck releases.".into());
    }
    let res = agent().get(url).call().map_err(|e| format!("Download failed: {e}"))?;
    let mut bytes = Vec::new();
    res.into_body()
        .into_reader()
        .take(300 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Download failed: {e}"))?;
    let path = std::env::temp_dir().join("FlightDeck-Setup.exe");
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save the installer: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("v0.1.12", "0.1.9"));
        assert!(!is_newer("0.1.9", "0.1.9"));
        assert!(!is_newer("v0.1.3", "0.1.10"));
        assert!(is_newer("1.0.0", "0.9.99"));
        assert!(!is_newer("garbage", "0.1.0"));
    }
}
