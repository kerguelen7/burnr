//! Feedback-log voor de UI: getijdstempelde regels met niveau en kleur.

use chrono::Local;
use egui::Color32;

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

/// Ring-buffer met een maximale lengte, zodat de UI niet onbeperkt groeit.
#[derive(Default)]
pub struct LogStore {
    entries: Vec<LogEntry>,
}

const MAX_ENTRIES: usize = 2000;

impl LogStore {
    pub fn push(&mut self, level: Level, msg: impl Into<String>) {
        let time = Local::now().format("%H:%M:%S").to_string();
        self.entries.push(LogEntry {
            time,
            level,
            msg: msg.into(),
        });
        if self.entries.len() > MAX_ENTRIES {
            let excess = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(..excess);
        }
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
