//! Aircraft photos from Planespotters.net, by registration.
//!
//! https://www.planespotters.net/photo/api — free, no key. Its rules, which
//! this follows: the JSON may be cached for up to 24 hours; images load
//! from the returned URLs in the viewer (never downloaded or stored here);
//! each photo is shown with its photographer's name and links to its page;
//! server-side requests send a descriptive User-Agent with a contact URL.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct SpotterPhoto {
    /// Image URL, used unchanged.
    pub src: String,
    pub width: u32,
    pub height: u32,
    /// The photo's page on Planespotters.net.
    pub link: String,
    pub photographer: String,
}

const KEEP: Duration = Duration::from_secs(24 * 3600);

#[derive(Default)]
pub struct Cache(Mutex<HashMap<String, (Instant, Option<SpotterPhoto>)>>);

/// Registrations look like "N320FD", "EI-IFF", "D-ABCD", "JA8089".
fn plausible(reg: &str) -> bool {
    (3..=10).contains(&reg.len())
        && reg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && reg.chars().any(|c| c.is_ascii_alphabetic())
}

fn fetch(reg: &str) -> Result<Option<SpotterPhoto>, String> {
    let url = format!("https://api.planespotters.net/pub/photos/reg/{reg}");
    let mut res = crate::update::agent()
        .get(&url)
        .header(
            "User-Agent",
            concat!("FlightDeck/", env!("CARGO_PKG_VERSION"), " (flight sim logbook; +https://github.com/dantesmante/flightdeck)"),
        )
        .call()
        .map_err(|e| format!("Planespotters: {e}"))?;
    let v: Value = res.body_mut().read_json().map_err(|e| format!("Planespotters: {e}"))?;
    let Some(p) = v.get("photos").and_then(Value::as_array).and_then(|a| a.first()) else { return Ok(None) };
    let thumb = p.get("thumbnail_large").or_else(|| p.get("thumbnail"));
    let src = thumb.and_then(|t| t.get("src")).and_then(Value::as_str).unwrap_or_default();
    let link = p.get("link").and_then(Value::as_str).unwrap_or_default();
    if src.is_empty() || link.is_empty() {
        return Ok(None);
    }
    let dim = |k: &str| thumb.and_then(|t| t.get("size")).and_then(|s| s.get(k)).and_then(Value::as_u64).unwrap_or(0) as u32;
    Ok(Some(SpotterPhoto {
        src: src.to_string(),
        width: dim("width"),
        height: dim("height"),
        link: link.to_string(),
        photographer: p.get("photographer").and_then(Value::as_str).unwrap_or("").to_string(),
    }))
}

impl Cache {
    /// The latest photo of an aircraft, or None when there is none (or the
    /// registration is not a real one).
    pub fn photo(&self, registration: &str) -> Result<Option<SpotterPhoto>, String> {
        let reg = registration.trim().to_uppercase();
        if !plausible(&reg) {
            return Ok(None);
        }
        if let Some((at, hit)) = self.0.lock().unwrap().get(&reg) {
            if at.elapsed() < KEEP {
                return Ok(hit.clone());
            }
        }
        let hit = fetch(&reg)?;
        self.0.lock().unwrap().insert(reg, (Instant::now(), hit.clone()));
        Ok(hit)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn registrations() {
        for ok in ["N320FD", "EI-IFF", "D-ABCD", "JA8089", "C-GFDK"] {
            assert!(super::plausible(ok), "{ok}");
        }
        for bad in ["", "AB", "1234", "N 320", "ATCCOM.AC_MODEL"] {
            assert!(!super::plausible(bad), "{bad}");
        }
    }
}
