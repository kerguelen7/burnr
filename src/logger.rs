//! Feedback-log voor de UI: getijdstempelde regels met niveau en kleur,
//! plus een automatisch sessielogbestand voor foutopsporing (stap 9a).

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Success,
    Warning,
    Error,
}

impl Level {
    /// Volgorde van de filter-schakelaars in het logpaneel — tegelijk de
    /// indexvolgorde van [`LogFilter`] en van de persistentie (`[bool; 4]`).
    pub const FILTER_ORDER: [Level; 4] =
        [Level::Info, Level::Success, Level::Warning, Level::Error];

    /// Index in [`Level::FILTER_ORDER`] / [`LogFilter`] (declaratievolgorde).
    pub fn slot(self) -> usize {
        self as usize
    }

    /// Kolomtag per niveau (schermlog: via de i18n-catalogus; sessielog:
    /// deze methode, taalonafhankelijk).
    pub fn tag(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Success => "OK  ",
            Level::Warning => "WARN",
            Level::Error => "FOUT",
        }
    }
}

pub struct LogEntry {
    pub time: String,
    pub level: Level,
    pub msg: String,
}

/// Ring-buffer met een maximale lengte, plus een optioneel sessielogbestand
/// waarnaar élke regel wordt gespiegeld (incl. volledige datum voor
/// foutopsporing achteraf).
#[derive(Default)]
pub struct LogStore {
    entries: Vec<LogEntry>,
    file: Option<File>,
}

const MAX_ENTRIES: usize = 2000;

impl LogStore {
    /// Standaard lege store (geen bestand).
    pub fn new() -> Self {
        Self::default()
    }

    /// Logstore met sessielogbestand; het bestand wordt aangemaakt (incl.
    /// map) en opent met een kopregel. Faalt het openen, dan valt de GUI
    /// gewoon terug op schermlog zonder bestand.
    pub fn with_session_file(path: PathBuf) -> std::io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = File::create(&path)?;
        writeln!(
            file,
            "Burnr sessielog — gestart {}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;
        file.flush()?;
        Ok(Self {
            entries: Vec::new(),
            file: Some(file),
        })
    }

    pub fn push(&mut self, level: Level, msg: impl Into<String>) {
        let now = Local::now();
        let time = now.format("%H:%M:%S").to_string();
        self.entries.push(LogEntry {
            time: time.clone(),
            level,
            msg: msg.into(),
        });
        if self.entries.len() > MAX_ENTRIES {
            let excess = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(..excess);
        }
        // Sessielogbestand: volledige datum+tijd, ook voor foutopsporing.
        if let Some(f) = &mut self.file {
            let _ = writeln!(
                f,
                "{} [{:>4}] {}",
                now.format("%Y-%m-%d %H:%M:%S"),
                level.tag().trim(),
                self.entries.last().map(|e| e.msg.as_str()).unwrap_or("")
            );
            let _ = f.flush();
        }
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Welke logniveaus het logpaneel toont. De vlaggen staan in dezelfde
/// volgorde als [`Level::FILTER_ORDER`]; de persistentie blijft een
/// `[bool; 4]` (stap 8), dus oude opslagbestanden laden zonder migratie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogFilter(pub [bool; 4]);

impl Default for LogFilter {
    fn default() -> Self {
        Self([true; 4])
    }
}

impl LogFilter {
    /// Is dit niveau zichtbaar in het logpaneel?
    pub fn allows(self, level: Level) -> bool {
        self.0[level.slot()]
    }

    /// Zet de zichtbaarheid van een logniveau.
    pub fn set(&mut self, level: Level, on: bool) {
        self.0[level.slot()] = on;
    }
}
