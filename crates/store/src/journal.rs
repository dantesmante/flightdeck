//! Crash-safe journal of the flight in progress.
//!
//! While a flight is active, every sample the tracker keeps for the track is
//! appended to `active-flight.jsonl` (about 1 line per second). If the sim or
//! the app crashes, the next start replays the journal through a fresh tracker
//! and saves what was flown as a partial flight.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use telemetry::{AircraftInfo, SimKind, Telemetry};

pub const JOURNAL_FILE: &str = "active-flight.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalHeader {
    pub sim: SimKind,
    pub aircraft: AircraftInfo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "snake_case")]
enum Line {
    Header(JournalHeader),
    Sample(Telemetry),
}

pub struct Journal {
    path: PathBuf,
    w: LineWriter<File>,
}

impl Journal {
    /// Start a new journal, replacing any previous one.
    pub fn create(dir: &Path, header: JournalHeader) -> std::io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join(JOURNAL_FILE);
        let f = OpenOptions::new().create(true).write(true).truncate(true).open(&path)?;
        let mut j = Journal { path, w: LineWriter::new(f) };
        j.write(&Line::Header(header))?;
        Ok(j)
    }

    fn write(&mut self, line: &Line) -> std::io::Result<()> {
        let mut s = serde_json::to_string(line).map_err(std::io::Error::other)?;
        s.push('\n');
        self.w.write_all(s.as_bytes())
    }

    pub fn append(&mut self, s: &Telemetry) -> std::io::Result<()> {
        self.write(&Line::Sample(*s))
    }

    /// Update the aircraft (written as a new header line).
    pub fn set_aircraft(&mut self, header: JournalHeader) -> std::io::Result<()> {
        self.write(&Line::Header(header))
    }

    /// The flight was saved: remove the journal.
    pub fn finish(self) -> std::io::Result<()> {
        drop(self.w);
        fs::remove_file(&self.path)
    }
}

/// A journal left behind by a crash.
pub struct Recovered {
    pub header: JournalHeader,
    pub samples: Vec<Telemetry>,
}

/// Read and delete a leftover journal, if there is one. Damaged lines (for
/// example a half-written last line) are skipped.
pub fn recover(dir: &Path) -> Option<Recovered> {
    let path = dir.join(JOURNAL_FILE);
    let f = File::open(&path).ok()?;
    let mut header = None;
    let mut samples = Vec::new();
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        match serde_json::from_str::<Line>(&line) {
            Ok(Line::Header(h)) => header = Some(h),
            Ok(Line::Sample(s)) => samples.push(s),
            Err(_) => continue,
        }
    }
    let _ = fs::remove_file(&path);
    Some(Recovered { header: header?, samples })
}
