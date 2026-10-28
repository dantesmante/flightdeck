//! Diagnostics: a log kept in memory and in `flightdeck.log` (in the data
//! folder), and a plain-text report the pilot can copy or save and send,
//! with the recent flights' timelines. The installed app has no terminal,
//! so this is how sim-specific behaviour gets diagnosed.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};

const KEEP_LINES: usize = 4000;
const MAX_FILE: u64 = 4 * 1024 * 1024;

struct DiagLog {
    lines: Mutex<VecDeque<String>>,
    file: Mutex<Option<File>>,
}

static LOG: OnceLock<DiagLog> = OnceLock::new();

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

impl Log for DiagLog {
    fn enabled(&self, m: &Metadata) -> bool {
        // FlightDeck's own messages at info and up; other libraries only warnings.
        let ours = m.target().starts_with("flightdeck")
            || m.target().starts_with("conn_")
            || m.target().starts_with("tracker")
            || m.target().starts_with("store");
        m.level() <= if ours { Level::Info } else { Level::Warn }
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let line = format!("{} {:<5} {}: {}", crate::volanta::format_time(now_ms(), true), r.level(), r.target(), r.args());
        eprintln!("{line}");
        if let Ok(mut f) = self.file.lock() {
            if let Some(f) = f.as_mut() {
                let _ = writeln!(f, "{line}");
            }
        }
        if let Ok(mut l) = self.lines.lock() {
            if l.len() >= KEEP_LINES {
                l.pop_front();
            }
            l.push_back(line);
        }
    }

    fn flush(&self) {
        if let Ok(mut f) = self.file.lock() {
            if let Some(f) = f.as_mut() {
                let _ = f.flush();
            }
        }
    }
}

/// Install the logger (call once, first thing).
pub fn init() {
    let l = LOG.get_or_init(|| DiagLog { lines: Mutex::new(VecDeque::new()), file: Mutex::new(None) });
    if log::set_logger(l).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

/// Start writing the log to `flightdeck.log` in the data folder. The
/// previous file is kept as `flightdeck.old.log` once it grows large.
pub fn set_folder(dir: &Path) {
    let path = dir.join("flightdeck.log");
    if std::fs::metadata(&path).map(|m| m.len() > MAX_FILE).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("flightdeck.old.log"));
    }
    if let Some(l) = LOG.get() {
        if let Ok(f) = OpenOptions::new().create(true).append(true).open(&path) {
            *l.file.lock().unwrap() = Some(f);
        }
    }
}

/// The log file's path.
pub fn log_path(dir: &Path) -> PathBuf {
    dir.join("flightdeck.log")
}

/// The last `n` log lines from this session.
pub fn recent(n: usize) -> Vec<String> {
    LOG.get()
        .and_then(|l| l.lines.lock().ok().map(|v| v.iter().rev().take(n).rev().cloned().collect()))
        .unwrap_or_default()
}

/// The last `n` lines of the log file, which also covers earlier sessions.
pub fn file_tail(dir: &Path, n: usize) -> Vec<String> {
    let text = std::fs::read_to_string(log_path(dir)).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|s| s.to_string()).collect()
}
