//! Feedback-log voor de UI: getijdstempelde regels met niveau en kleur,
//! plus een automatisch sessielogbestand voor foutopsporing (stap 9a).

use chrono::Local;
use egui::Color32;
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
    pub fn tag(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Success => "OK  ",
            Level::Warning => "WARN",
            Level::Error => "FOUT",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Level::Info => Color32::from_rgb(140, 158, 178),
            Level::Success => Color32::from_rgb(108, 200, 128),
            Level::Warning => Color32::from_rgb(232, 182, 92),
            Level::Error => Color32::from_rgb(238, 112, 112),
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
            "LibBurn GUI sessielog — gestart {}",
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
