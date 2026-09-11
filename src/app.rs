//! Applicatiestatus en hoofdlus van de GUI.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use egui::Color32;

use crate::logger::{Level, LogStore};
use crate::settings::BurnSettings;
use crate::ui;
use crate::worker::{self, Command, DriveEntry, Event};

/// Status van de libburn-library zelf.
#[derive(Clone, Debug)]
pub enum LibState {
    Loading,
    Loaded { version: String, path: String },
    Failed { error: String },
}

/// Status van de drive-scan.
#[derive(Clone, Debug, PartialEq)]
pub enum ScanState {
    Idle,
    Scanning,
    Done,
    Failed { error: String },
}

/// Lopende schijfkopie.
#[derive(Clone, Debug)]
pub struct ActiveRead {
    pub index: usize,
    pub blocks_done: i64,
    pub total_blocks: i32,
    pub kbps: f64,
}

impl ActiveRead {
    pub fn fraction(&self) -> f32 {
        if self.total_blocks <= 0 {
            return 0.0;
        }
        (self.blocks_done as f32 / self.total_blocks as f32).clamp(0.0, 1.0)
    }
}

pub struct App {
    pub log: LogStore,
    pub lib_state: LibState,
    pub scan_state: ScanState,
    pub drives: Vec<DriveEntry>,
    pub selected: Option<usize>,
    /// Station waar momenteel een inspectie op draait (grab in de worker).
    pub busy_drive: Option<usize>,
    /// Lopende schijfkopie (één tegelijk; de worker is serieel).
    pub active_read: Option<ActiveRead>,
    /// Pad voor de volgende schijfkopie.
    pub read_path: String,
    pub settings: BurnSettings,
    /// Drives exclusief openen (O_EXCL)? Uitzetten bij automount-conflicten.
    pub exclusive_open: bool,

    cmd_tx: Sender<Command>,
    events: Receiver<Event>,
    worker: Option<JoinHandle<()>>,

    /// Automatisch scannen zodra de library geladen is.
    auto_scan_queued: bool,
    /// Pad naar een libburn-shared object, ingevuld door de gebruiker.
    pub custom_so: String,
    pub log_filter: [bool; 4],
    pub auto_scroll: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.visuals.panel_fill = Color32::from_rgb(24, 26, 31);
        });

        let (cmd_tx, cmd_rx) = channel::<Command>();
        let (event_tx, event_rx) = channel::<Event>();
        let handle = worker::spawn(cmd_rx, event_tx, cc.egui_ctx.clone());

        let mut app = Self {
            log: LogStore::default(),
            lib_state: LibState::Loading,
            scan_state: ScanState::Idle,
            drives: Vec::new(),
            selected: None,
            busy_drive: None,
            active_read: None,
            read_path: String::new(),
            settings: BurnSettings::default(),
            exclusive_open: true,
            cmd_tx,
            events: event_rx,
            worker: Some(handle),
            auto_scan_queued: true,
            custom_so: String::new(),
            log_filter: [true, true, true, true],
            auto_scroll: true,
        };
        app.log.push(
            Level::Info,
            "LibBurn GUI gestart — libburn wordt van het systeem geladen…".to_string(),
        );
        app.send(Command::LoadLibrary {
            path: None,
            exclusive: app.exclusive_open,
        });
        app
    }

    fn send(&mut self, cmd: Command) {
        if self.cmd_tx.send(cmd).is_err() {
            self.log
                .push(Level::Error, "Worker-thread is gestopt".to_string());
        }
    }

    pub fn request_scan(&mut self) {
        if self.scan_state == ScanState::Scanning {
            return;
        }
        self.log.push(Level::Info, "Scan aangevraagd".to_string());
        self.send(Command::Scan);
    }

    pub fn request_inspect(&mut self, index: usize) {
        if self.busy_drive.is_some() || self.active_read.is_some() {
            return;
        }
        self.log.push(
            Level::Info,
            format!("Inspectie aangevraagd voor station {index}"),
        );
        self.send(Command::InspectDrive(index));
    }

    pub fn reload_library(&mut self, path: Option<String>) {
        self.lib_state = LibState::Loading;
        self.auto_scan_queued = true;
        self.log
            .push(Level::Info, "libburn (opnieuw) laden…".to_string());
        self.send(Command::LoadLibrary {
            path,
            exclusive: self.exclusive_open,
        });
    }

    /// Wijzig de exclusief-openen-modus: vereist herladen + nieuwe scan.
    pub fn set_exclusive_open(&mut self, on: bool) {
        if self.exclusive_open == on {
            return;
        }
        self.exclusive_open = on;
        self.reload_library(None);
    }

    pub fn request_read(&mut self, index: usize, path: String) {
        if self.active_read.is_some() || self.busy_drive.is_some() {
            return;
        }
        let expanded = expand_home(&path);
        if expanded.trim().is_empty() {
            self.log
                .push(Level::Warning, "Geen uitvoerbestand opgegeven".to_string());
            return;
        }
        self.log.push(
            Level::Info,
            format!(
                "Schijfkopie aangevraagd voor station {index} → `{}`",
                expanded
            ),
        );
        self.send(Command::ReadDisc {
            index,
            path: expanded,
        });
    }

    pub fn cancel_read(&mut self) {
        if self.active_read.is_some() {
            self.log
                .push(Level::Info, "Kopie annuleren aangevraagd".to_string());
            self.send(Command::CancelRead);
        }
    }

    fn handle_event(&mut self, ev: Event) {
        match ev {
            Event::Log(level, msg) => self.log.push(level, msg),
            Event::LibLoaded { version, path } => {
                self.lib_state = LibState::Loaded { version, path };
                if self.auto_scan_queued {
                    self.auto_scan_queued = false;
                    self.send(Command::Scan);
                }
            }
            Event::LibLoadFailed { error } => {
                self.lib_state = LibState::Failed { error };
                self.auto_scan_queued = false;
            }
            Event::ScanStarted => {
                self.scan_state = ScanState::Scanning;
                self.drives.clear();
                self.selected = None;
                self.busy_drive = None;
            }
            Event::ScanDone { drives } => {
                self.scan_state = ScanState::Done;
                self.selected = if drives.is_empty() { None } else { Some(0) };
                self.drives = drives;
            }
            Event::ScanFailed { error } => {
                self.scan_state = ScanState::Failed { error };
            }
            Event::InspectStarted { index } => {
                self.busy_drive = Some(index);
                if let Some(d) = self.drives.get_mut(index) {
                    // Oude waarden direct weg: zo kan een nieuwe inspectie
                    // nooit verward worden met de vorige.
                    d.media = None;
                    d.inspect_error = None;
                }
            }
            Event::InspectDone { index, media } => {
                self.busy_drive = None;
                if let Some(d) = self.drives.get_mut(index) {
                    d.media = Some(media);
                    d.inspect_error = None;
                }
            }
            Event::InspectFailed { index, error } => {
                self.busy_drive = None;
                if let Some(d) = self.drives.get_mut(index) {
                    d.inspect_error = Some(error);
                }
            }
            Event::ReadStarted {
                index,
                total_blocks,
            } => {
                self.active_read = Some(ActiveRead {
                    index,
                    blocks_done: 0,
                    total_blocks,
                    kbps: 0.0,
                });
            }
            Event::ReadProgress {
                index,
                blocks_done,
                total_blocks,
                kbps,
            } => {
                if let Some(r) = self.active_read.as_mut() {
                    if r.index == index {
                        r.blocks_done = blocks_done;
                        r.total_blocks = total_blocks;
                        r.kbps = kbps;
                    }
                }
            }
            Event::ReadDone => {
                self.active_read = None;
            }
            Event::ReadFailed { index, error } => {
                self.active_read = None;
                if let Some(d) = self.drives.get_mut(index) {
                    d.inspect_error = Some(error);
                }
            }
            Event::ReadCancelled => {
                self.active_read = None;
            }
            Event::WorkerStopped => {}
        }
    }
}

impl eframe::App for App {
    /// Statusupdates: events van de worker verwerken (wordt ook aangeroepen
    /// als de UI verborgen is).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(ev) = self.events.try_recv() {
            self.handle_event(ev);
        }

        // Actief bezig? Dan blijft de UI het event-kanaal pollen.
        let busy = matches!(self.lib_state, LibState::Loading)
            || self.scan_state == ScanState::Scanning
            || self.busy_drive.is_some()
            || self.active_read.is_some();
        if busy {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Volgorde is belangrijk: CentralPanel als laatste.
        ui::top_bar::show(ui, self);
        ui::drives_panel::show(ui, self);
        ui::settings_panel::show(ui, self);
        ui::log_panel::show(ui, self);
        ui::details_panel::show(ui, self);
    }
}

/// Zet een pad dat met `~` begint om naar een absoluut pad via $HOME.
fn expand_home(path: &str) -> String {
    let p = path.trim();
    if p == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| p.to_string());
    }
    if let Some(rest) = p.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            let mut s = home;
            if !s.ends_with('/') {
                s.push('/');
            }
            s.push_str(rest);
            return s;
        }
    }
    p.to_string()
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(Command::Shutdown);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}
