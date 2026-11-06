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
/// Folders of small copies other programs keep beside the real pictures
/// (Steam writes one into "thumbnails" for every screenshot): never looked
/// into.
pub const SKIP_DIRS: &[&str] = &["thumbnails", "thumbnail", "thumbs", ".thumbnails"];

fn skipped(dir: &Path) -> bool {
    dir.file_name().and_then(|n| n.to_str()).is_some_and(|n| SKIP_DIRS.contains(&n.to_ascii_lowercase().as_str()))
}

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
                if depth < DEPTH && !skipped(&p) {
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
            // Small copies noted before they were left out.
            match db.lock().unwrap().forget_screenshots_in(SKIP_DIRS) {
                Ok(n) if n > 0 => log::info!("screenshots: forgot {n} thumbnail copies"),
                _ => {}
            }
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

/// Small copies of the pictures for the Photos tab, made when first shown
/// and kept in memory only (FlightDeck writes nothing beside your files).
static THUMBS: Mutex<Vec<(i64, Arc<Vec<u8>>)>> = Mutex::new(Vec::new());
const THUMBS_KEPT: usize = 400;
/// Wide enough for two to a row on a high-density screen.
const THUMB_W: u32 = 480;

/// A screenshot's small copy, as JPEG.
pub fn thumb(db: &Mutex<Store>, id: i64) -> Option<Arc<Vec<u8>>> {
    if let Some((_, t)) = THUMBS.lock().unwrap().iter().find(|(i, _)| *i == id) {
        return Some(t.clone());
    }
    let path = db.lock().unwrap().screenshot_path(id).ok()??;
    let img = image::ImageReader::open(&path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let h = (img.height() as u64 * THUMB_W as u64 / img.width().max(1) as u64).max(1) as u32;
    let small = image::DynamicImage::ImageRgba8(image::imageops::thumbnail(&img, THUMB_W.min(img.width()), h.min(img.height()))).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82).encode_image(&small).ok()?;
    let out = Arc::new(out);
    let mut cache = THUMBS.lock().unwrap();
    if cache.len() >= THUMBS_KEPT {
        cache.remove(0);
    }
    cache.push((id, out.clone()));
    Some(out)
}

/// Delete a screenshot: the file goes to the Recycle Bin (unless only
/// FlightDeck should forget it), and FlightDeck forgets it.
pub fn delete(db: &Mutex<Store>, id: i64, file_too: bool) -> Result<(), String> {
    let path = db.lock().unwrap().screenshot_path(id).map_err(|e| e.to_string())?;
    if file_too {
        if let Some(p) = path.as_deref().filter(|p| Path::new(p).exists()) {
            trash::delete(p).map_err(|e| format!("Could not move the picture to the Recycle Bin: {e}"))?;
            log::info!("screenshot moved to the Recycle Bin: {p}");
        }
    }
    db.lock().unwrap().forget_screenshot(id).map_err(|e| e.to_string())?;
    THUMBS.lock().unwrap().retain(|(i, _)| *i != id);
    Ok(())
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
        std::fs::create_dir_all(sub.join("thumbnails")).unwrap();
        std::fs::write(sub.join("thumbnails").join("c.jpg"), b"x").unwrap();
        std::fs::write(dir.join("a.png"), b"x").unwrap();
        std::fs::write(sub.join("b.JPG"), b"x").unwrap();
        std::fs::write(sub.join("notes.txt"), b"x").unwrap();
        let mut found: Vec<String> = scan(&dir).into_iter().map(|(p, _)| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        found.sort();
        assert_eq!(found, vec!["a.png", "b.JPG"], "Steam's thumbnails left out");
        assert_eq!(media_type("x/b.JPG"), "image/jpeg");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn small_copies_and_deleting() {
        let dir = std::env::temp_dir().join(format!("fd-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("shot.png");
        image::RgbImage::from_fn(1600, 900, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])).save(&file).unwrap();
        let db = Mutex::new(Store::open_in_memory().unwrap());
        let id = db.lock().unwrap().add_screenshot(&file.to_string_lossy(), 1).unwrap().unwrap();
        let t = thumb(&db, id).unwrap();
        let small = image::load_from_memory(&t).unwrap();
        assert_eq!((small.width(), small.height()), (480, 270));
        assert!(Arc::ptr_eq(&t, &thumb(&db, id).unwrap()), "kept for next time");
        // Forget only: the file stays.
        delete(&db, id, false).unwrap();
        assert!(file.exists());
        assert!(thumb(&db, id).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
