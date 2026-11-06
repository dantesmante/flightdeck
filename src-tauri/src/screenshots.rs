//! Screenshots: FlightDeck watches the folder the pilot chose (Settings) for
//! new pictures and remembers where each one is and when it was taken. A
//! picture taken during a flight joins that flight when it is saved, and
//! the map places it from the flight's track at that moment.
//!
//! The pictures stay where they are: FlightDeck keeps only their paths and
//! reads the files when they are shown, so moving or deleting a file (or
//! the folder) breaks the link.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use store::Store;
use tauri::{AppHandle, Emitter};

/// The setting holding the folder.
pub const KEY_DIR: &str = "screenshot_dir";
/// How often the folder is looked at.
const EVERY: Duration = Duration::from_secs(4);
/// How deep into sub-folders to look (game bars and launchers file their
/// pictures by game).
const DEPTH: usize = 3;
/// Pictures a browser can show.
const KINDS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif"];

fn ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn is_picture(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| KINDS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Every picture under a folder, with when it was taken (created, or last
/// written where creation time is not kept).
pub fn scan(dir: &Path) -> Vec<(PathBuf, i64)> {
    let mut out = vec![];
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                if depth < DEPTH {
                    stack.push((p, depth + 1));
                }
            } else if is_picture(&p) {
                let t = meta.created().or_else(|_| meta.modified()).map(ms).unwrap_or(0);
                out.push((p, t));
            }
        }
    }
    out
}

/// Watch the chosen folder for new pictures, for as long as the app runs.
/// Pictures already there when the folder is chosen (or the app starts)
/// are not new; the first look only learns them.
pub fn spawn(app: AppHandle, db: Arc<Mutex<Store>>) {
    std::thread::Builder::new()
        .name("screenshots".into())
        .spawn(move || {
            let mut dir: Option<PathBuf> = None;
            let mut seen: HashSet<PathBuf> = HashSet::new();
            let mut since = 0;
            let mut last_prune = 0;
            loop {
                std::thread::sleep(EVERY);
                let want = db.lock().unwrap().settings().ok().and_then(|s| s.get(KEY_DIR).cloned()).filter(|s| !s.trim().is_empty()).map(PathBuf::from);
                if want != dir {
                    dir = want;
                    seen.clear();
                    since = ms(SystemTime::now());
                    if let Some(d) = &dir {
                        seen.extend(scan(d).into_iter().map(|(p, _)| p));
                        log::info!("screenshots: watching {} ({} pictures there already)", d.display(), seen.len());
                    }
                    continue;
                }
                let Some(d) = &dir else { continue };
                for (p, t) in scan(d) {
                    if !seen.insert(p.clone()) {
                        continue;
                    }
                    // A file copied in keeps its old dates: count it from now.
                    let taken = if t >= since - 60_000 { t } else { ms(SystemTime::now()) };
                    let path = p.to_string_lossy().to_string();
                    match db.lock().unwrap().add_screenshot(&path, taken) {
                        Ok(Some(id)) => {
                            log::info!("screenshot noted: {path}");
                            let _ = app.emit("screenshot", serde_json::json!({ "id": id, "taken_ms": taken }));
                        }
                        Ok(None) => {}
                        Err(e) => log::warn!("screenshot {path}: {e}"),
                    }
                }
                // Pictures not taken during any flight are forgotten after a few days.
                let now = ms(SystemTime::now());
                if now - last_prune > 3_600_000 {
                    last_prune = now;
                    let _ = db.lock().unwrap().prune_screenshots(now - 3 * 86_400_000);
                }
            }
        })
        .expect("screenshot thread");
}

/// The bytes and media type of a screenshot, read from where it is.
pub fn read(db: &Mutex<Store>, id: i64) -> Option<(Vec<u8>, &'static str)> {
    let path = db.lock().unwrap().screenshot_path(id).ok()??;
    let bytes = std::fs::read(&path).ok()?;
    Some((bytes, media_type(&path)))
}

pub fn media_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_pictures_in_sub_folders_only() {
        let dir = std::env::temp_dir().join(format!("fd-shots-{}", std::process::id()));
        let sub = dir.join("Microsoft Flight Simulator 2024");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(dir.join("a.png"), b"x").unwrap();
        std::fs::write(sub.join("b.JPG"), b"x").unwrap();
        std::fs::write(sub.join("notes.txt"), b"x").unwrap();
        let mut found: Vec<String> = scan(&dir).into_iter().map(|(p, _)| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        found.sort();
        assert_eq!(found, vec!["a.png", "b.JPG"]);
        assert_eq!(media_type("x/b.JPG"), "image/jpeg");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
