//! Backup Data: everything FlightDeck knows (flights and tracks, logbook,
//! hangar and its pictures, pilot, passport, settings) is in one database,
//! so a backup is one consistent copy of it, a ".fdbackup" file. It can be
//! saved anywhere, kept in a folder (a cloud-synced one, say) and refreshed
//! each time FlightDeck closes, and restored on this PC or a new one.
//!
//! Screenshots are not in it: FlightDeck only links to them.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use store::Store;

/// The folder kept up to date, if any.
pub const KEY_DIR: &str = "backup_dir";
/// "1": back up to the folder whenever FlightDeck closes.
pub const KEY_ON_CLOSE: &str = "backup_on_close";
/// When the folder was last written (Unix ms).
pub const KEY_LAST: &str = "backup_last_ms";

pub const EXT: &str = "fdbackup";
const NAME: &str = "FlightDeck backup.fdbackup";
const PREVIOUS: &str = "FlightDeck backup (previous).fdbackup";
/// A backup waiting to replace the database at the next start.
const STAGED: &str = "restore.fdbackup";
const DB: &str = "flightdeck.db";

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Write the backup into the folder, keeping the one before it as
/// "(previous)" in case the newest is ever not wanted.
pub fn to_folder(db: &Mutex<Store>, dir: &Path) -> Result<PathBuf, String> {
    if !dir.is_dir() {
        return Err(format!("The backup folder {} is not there (a cloud drive not signed in?).", dir.display()));
    }
    let file = dir.join(NAME);
    if file.exists() {
        let _ = std::fs::rename(&file, dir.join(PREVIOUS));
    }
    let db = db.lock().unwrap();
    db.backup_to(&file).map_err(|e| format!("Could not write the backup: {e}"))?;
    let _ = db.set_setting(KEY_LAST, &now_ms().to_string());
    Ok(file)
}

/// When FlightDeck closes: refresh the folder's backup, if the pilot asked.
pub fn on_close(db: &Mutex<Store>) {
    let s = db.lock().unwrap().settings().unwrap_or_default();
    if s.get(KEY_ON_CLOSE).map(String::as_str) != Some("1") {
        return;
    }
    let Some(dir) = s.get(KEY_DIR).filter(|d| !d.trim().is_empty()) else { return };
    match to_folder(db, Path::new(dir)) {
        Ok(f) => log::info!("backed up to {} on close", f.display()),
        Err(e) => log::warn!("backup on close: {e}"),
    }
}

/// Put a checked backup aside to replace the database at the next start.
pub fn stage(data_dir: &Path, from: &Path) -> Result<(), String> {
    std::fs::copy(from, data_dir.join(STAGED)).map_err(|e| format!("Could not copy the backup: {e}"))?;
    Ok(())
}

pub fn unstage(data_dir: &Path) {
    let _ = std::fs::remove_file(data_dir.join(STAGED));
}

/// At start, before the database opens: swap in a staged backup. The
/// database it replaces is kept as flightdeck.before-restore.db.
pub fn apply_staged(data_dir: &Path) {
    let staged = data_dir.join(STAGED);
    if !staged.exists() {
        return;
    }
    if let Err(e) = Store::check_backup(&staged) {
        log::warn!("restore: the staged backup is not usable ({e}); left as it is");
        let _ = std::fs::rename(&staged, data_dir.join("restore.unusable.fdbackup"));
        return;
    }
    let db = data_dir.join(DB);
    if db.exists() {
        let _ = std::fs::rename(&db, data_dir.join("flightdeck.before-restore.db"));
    }
    for side in ["flightdeck.db-wal", "flightdeck.db-shm"] {
        let _ = std::fs::remove_file(data_dir.join(side));
    }
    match std::fs::rename(&staged, &db) {
        Ok(()) => log::info!("restored a backup"),
        Err(e) => log::error!("restore: could not put the backup in place: {e}"),
    }
}

/// A file name for a backup saved by hand, dated.
pub fn dated_name() -> String {
    let s = crate::volanta::format_time(now_ms(), false);
    format!("FlightDeck backup {}.{EXT}", &s[0..10])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_backup_keeps_the_previous_and_restores() {
        let root = std::env::temp_dir().join(format!("fd-bk-{}", std::process::id()));
        let data = root.join("data");
        let cloud = root.join("cloud");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&cloud).unwrap();
        let db = Mutex::new(Store::open(&data.join(DB)).unwrap());
        db.lock().unwrap().set_setting("distance_unit", "km").unwrap();
        to_folder(&db, &cloud).unwrap();
        to_folder(&db, &cloud).unwrap();
        assert!(cloud.join(NAME).exists() && cloud.join(PREVIOUS).exists());
        assert!(to_folder(&db, &root.join("nowhere")).is_err());

        // Restore onto a new PC: an empty data folder.
        let new_pc = root.join("new");
        std::fs::create_dir_all(&new_pc).unwrap();
        stage(&new_pc, &cloud.join(NAME)).unwrap();
        apply_staged(&new_pc);
        let back = Store::open(&new_pc.join(DB)).unwrap();
        assert_eq!(back.settings().unwrap().get("distance_unit").map(String::as_str), Some("km"));
        assert!(!new_pc.join(STAGED).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
