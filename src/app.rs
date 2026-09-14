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

/// Bron voor het branden: een ISO-bestand of een eigen bestandsselectie.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum BurnSourceKind {
    #[default]
    IsoFile,
    FileSet,
}

/// Status die tussen sessies bewaard wordt (stap 8, eframe-persistence).
/// Velden zijn Option zodat oudere opslagbestanden zonder fouten laden.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct PersistedState {
    pub settings: Option<BurnSettings>,
    pub exclusive_open: Option<bool>,
    pub burn_source_kind: Option<BurnSourceKind>,
    pub burn_path: Option<String>,
    pub read_path: Option<String>,
    pub log_filter: Option<[bool; 4]>,
}

impl PersistedState {
    fn from_app(app: &App) -> Self {
        Self {
            settings: Some(app.settings.clone()),
            exclusive_open: Some(app.exclusive_open),
            burn_source_kind: Some(app.burn_source_kind),
            burn_path: Some(app.burn_path.clone()),
            read_path: Some(app.read_path.clone()),
            log_filter: Some(app.log_filter),
        }
    }
}

/// Status-LED voor het brand-eiland: houdt de uitkomst van de laatste job
/// vast tot de volgende start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobLed {
    Idle,
    Busy,
    Ok,
    Error,
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

/// Lopend brandjob.
#[derive(Clone, Debug)]
pub struct ActiveBurn {
    pub index: usize,
    pub sector: i32,
    pub sectors: i32,
    pub kbps: f64,
    pub buffer_pct: f32,
    pub fifo_pct: f32,
    pub simulate: bool,
    /// Huidige fase ("schrijven", "track afsluiten", …).
    pub phase: String,
    /// Verstreken tijd in seconden.
    pub elapsed_secs: f64,
    /// Verwachte resterende tijd in seconden (0 = onbekend).
    pub eta_secs: f64,
}

impl ActiveBurn {
    pub fn fraction(&self) -> f32 {
        if self.sectors <= 0 {
            return 0.0;
        }
        (self.sector as f32 / self.sectors as f32).clamp(0.0, 1.0)
    }
}

/// Lopende onderhoudsjob (wissen/formatteren, stap 6).
#[derive(Clone, Debug)]
pub struct MaintJob {
    pub kind: crate::worker::MaintKind,
    pub index: usize,
    pub pct: f32,
}

pub struct App {
    pub log: LogStore,
    pub log_file: Option<std::path::PathBuf>,
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
    /// Lopende brandjobs — meerdere stations tegelijk (stap 9b).
    pub active_burns: Vec<ActiveBurn>,
    /// Lopende onderhoudsjobs (wissen/formatteren) per station.
    pub active_maints: Vec<MaintJob>,
    /// Pad naar het ISO-bestand om te branden.
    pub burn_path: String,
    /// Bronkeuze voor het branden.
    pub burn_source_kind: BurnSourceKind,
    /// Gekozen bestanden/mappen voor een data-image (stap 5, libisofs).
    pub burn_files: Vec<String>,
    /// Geschatte totale grootte van de selectie (bytes).
    pub burn_files_size: u64,
    /// Volumenaam van de samen te stellen image.
    pub volume_id: String,
    /// Status-LED van het brand-eiland.
    pub burn_led: JobLed,
    pub settings: BurnSettings,
    /// Drives exclusief openen (O_EXCL)? Uitzetten bij automount-conflicten.
    pub exclusive_open: bool,
    /// Aantal stations dat de LAATSTE voltooide scan vond — gebruikt om de
    /// "automount blokkeert de drive"-hint alleen te tonen als we weten dat
    /// er eerder wél een station was.
    pub last_scan_found: usize,

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

        // Stap 9a: sessielogbestand in de data-map van de gebruiker.
        let (log_store, log_path) = match session_log_file() {
            Some(path) => match LogStore::with_session_file(path.clone()) {
                Ok(store) => (store, Some(path)),
                Err(_) => (LogStore::new(), None),
            },
            None => (LogStore::new(), None),
        };

        let (cmd_tx, cmd_rx) = channel::<Command>();
        let (event_tx, event_rx) = channel::<Event>();
        let handle = worker::spawn(cmd_rx, event_tx, cc.egui_ctx.clone());

        let mut app = Self {
            log: log_store,
            log_file: log_path,
            lib_state: LibState::Loading,
            scan_state: ScanState::Idle,
            drives: Vec::new(),
            selected: None,
            busy_drive: None,
            active_read: None,
            read_path: String::new(),
            active_burns: Vec::new(),
            active_maints: Vec::new(),
            burn_path: String::new(),
            burn_source_kind: BurnSourceKind::IsoFile,
            burn_files: Vec::new(),
            burn_files_size: 0,
            volume_id: format!("Libburn{}", chrono::Local::now().format("%Y%m%d")),
            burn_led: JobLed::Idle,
            settings: BurnSettings::default(),
            exclusive_open: true,
            last_scan_found: 0,
            cmd_tx,
            events: event_rx,
            worker: Some(handle),
            auto_scan_queued: true,
            custom_so: String::new(),
            log_filter: [true, true, true, true],
            auto_scroll: true,
        };

        // Stap 8: opgeslagen instellingen laden (velden zijn Option; ontbrekende
        // velden krijgen de defaults hierboven).
        if let Some(storage) = cc.storage {
            let persisted: Option<PersistedState> = eframe::get_value(storage, eframe::APP_KEY);
            if let Some(p) = persisted {
                if let Some(s) = p.settings {
                    app.settings = s;
                }
                if let Some(v) = p.exclusive_open {
                    app.exclusive_open = v;
                }
                if let Some(v) = p.burn_source_kind {
                    app.burn_source_kind = v;
                }
                if let Some(v) = p.burn_path {
                    app.burn_path = v;
                }
                if let Some(v) = p.read_path {
                    app.read_path = v;
                }
                if let Some(v) = p.log_filter {
                    app.log_filter = v;
                }
                app.log.push(
                    Level::Info,
                    "Instellingen geladen uit vorige sessie".to_string(),
                );
            }
        }

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
        if self.busy_drive.is_some()
            || self.active_read.is_some()
            || self.job_active_on(index)
        {
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
        self.log.push(
            Level::Info,
            "Automatische scan volgt na het herladen.".to_string(),
        );
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
        // Lezen blijft sequentieel: één kopie tegelijk (blokkeert de worker).
        if self.active_read.is_some() || self.busy_drive.is_some() || self.job_active_on(index) {
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

    pub fn request_burn(&mut self, index: usize, path: String) {
        if self.active_read.is_some() || self.busy_drive.is_some() || self.job_active_on(index) {
            return;
        }
        let expanded = expand_home(&path);
        if expanded.trim().is_empty() {
            self.log
                .push(Level::Warning, "Geen ISO-bestand opgegeven".to_string());
            return;
        }
        if !std::path::Path::new(&expanded).is_file() {
            self.log.push(
                Level::Warning,
                format!("ISO-bestand bestaat niet: `{}`", expanded),
            );
            return;
        }
        self.log.push(
            Level::Info,
            format!(
                "Brandjob aangevraagd voor station {index} met `{}`",
                expanded
            ),
        );
        self.send(Command::BurnDisc {
            index,
            path: expanded,
            settings: self.settings.clone(),
        });
    }

    pub fn cancel_burn(&mut self, index: usize) {
        if self.active_burns.iter().any(|b| b.index == index) {
            self.log
                .push(Level::Info, "Brandjob annuleren aangevraagd".to_string());
            self.send(Command::CancelBurn { index });
        }
    }

    pub fn cancel_maint(&mut self, index: usize) {
        if self.active_maints.iter().any(|m| m.index == index) {
            self.log.push(
                Level::Info,
                "Onderhoudsjob annuleren aangevraagd".to_string(),
            );
            self.send(Command::CancelBurn { index });
        }
    }

    pub fn request_erase(&mut self, index: usize, fast: bool) {
        if self.active_read.is_some() || self.busy_drive.is_some() || self.job_active_on(index) {
            return;
        }
        self.log.push(
            Level::Info,
            format!(
                "Wissen aangevraagd voor station {index} ({})",
                if fast { "snel" } else { "volledig" }
            ),
        );
        self.send(Command::EraseDisc { index, fast });
    }

    pub fn request_format(&mut self, index: usize) {
        if self.active_read.is_some() || self.busy_drive.is_some() || self.job_active_on(index) {
            return;
        }
        self.log
            .push(Level::Info, "Formatteren aangevraagd".to_string());
        self.send(Command::FormatDisc {
            index,
            settings: self.settings.clone(),
        });
    }

    /// Draait er op dit station al een job (brand/wis/format/inspect)?
    pub fn job_active_on(&self, index: usize) -> bool {
        self.active_burns.iter().any(|b| b.index == index)
            || self.active_maints.iter().any(|m| m.index == index)
            || self.busy_drive == Some(index)
    }

    /// Voeg een bestand/map toe aan de data-selectie (geen duplicaten).
    pub fn add_burn_file(&mut self, path: String) {
        if path.is_empty() || self.burn_files.contains(&path) {
            return;
        }
        self.burn_files.push(path);
        self.recompute_burn_files_size();
    }

    pub fn remove_burn_file(&mut self, idx: usize) {
        if idx < self.burn_files.len() {
            self.burn_files.remove(idx);
            self.recompute_burn_files_size();
        }
    }

    /// Geschatte grootte van de selectie (recursief, met een tellingslimiet
    /// zodat de UI niet blokkeert op reusachtige bomen).
    fn recompute_burn_files_size(&mut self) {
        const MAX_FILES: usize = 100_000;
        fn walk(p: &std::path::Path, total: &mut u64, count: &mut usize) {
            if *count >= MAX_FILES {
                return;
            }
            match std::fs::symlink_metadata(p) {
                Ok(md) if md.is_dir() => {
                    if let Ok(rd) = std::fs::read_dir(p) {
                        for e in rd.flatten() {
                            walk(&e.path(), total, count);
                        }
                    }
                }
                Ok(md) => {
                    *total += md.len();
                    *count += 1;
                }
                Err(_) => {}
            }
        }
        let mut total = 0u64;
        let mut count = 0usize;
        for p in &self.burn_files {
            walk(std::path::Path::new(p), &mut total, &mut count);
        }
        self.burn_files_size = total;
    }

    /// Zet het volumelabel door naar het volgende nummer van vandaag
    /// (LibburnYYYYMMDD → LibburnYYYYMMDD-2 → -3 …). Een door de gebruiker
    /// zelf ingevulde naam blijft onaangeroerd.
    fn bump_volume_label(&mut self) {
        let today = chrono::Local::now().format("%Y%m%d").to_string();
        let prefix = format!("Libburn{today}");
        let cur = self.volume_id.trim();
        if cur == prefix {
            self.volume_id = format!("{prefix}-2");
        } else if let Some(rest) = cur.strip_prefix(&format!("{prefix}-")) {
            if let Ok(n) = rest.parse::<u32>() {
                self.volume_id = format!("{prefix}-{}", n + 1);
            }
        }
    }

    pub fn request_burn_files(&mut self, index: usize) {
        if self.active_read.is_some() || self.busy_drive.is_some() || self.job_active_on(index) {
            return;
        }
        if self.burn_files.is_empty() {
            self.log.push(
                Level::Warning,
                "Geen bestanden gekozen om te branden".to_string(),
            );
            return;
        }
        let volume = if self.volume_id.trim().is_empty() {
            format!("Libburn{}", chrono::Local::now().format("%Y%m%d"))
        } else {
            self.volume_id.trim().to_string()
        };
        // Multi-session import (stap 7): als de media appendable is en de
        // inspectie sessies vond, importeren we de laatste sessie zodat de
        // nieuwe bestanden bij de bestaande inhoud komen.
        let import_active = self
            .selected
            .and_then(|i| self.drives.get(i))
            .and_then(|d| d.media.as_ref())
            .is_some_and(|m| m.disc_status == crate::ffi::DiscStatus::Appendable);
        let import_start_block = self
            .selected
            .and_then(|i| self.drives.get(i))
            .and_then(|d| d.media.as_ref())
            .and_then(|m| m.sessions.last())
            .map(|s| s.start_lba)
            .filter(|&b| b >= 0)
            .filter(|_| import_active);
        self.log.push(
            Level::Info,
            format!(
                "Data-image branden aangevraagd voor station {index} — {} item(s), \
                 ≈ {}",
                self.burn_files.len(),
                crate::worker::format_blocks(((self.burn_files_size + 2047) / 2048) as i32)
            ),
        );
        self.send(Command::BurnFiles {
            index,
            paths: self.burn_files.clone(),
            volume_id: volume,
            settings: self.settings.clone(),
            import_start_block,
        });
    }

    fn handle_event(&mut self, ev: Event) {
        match ev {
            Event::Log(level, msg) => self.log.push(level, msg),
            Event::LibLoaded { version, path } => {
                self.lib_state = LibState::Loaded { version, path };
                if self.auto_scan_queued {
                    self.auto_scan_queued = false;
                    self.log.push(
                        Level::Info,
                        "Automatische scan na herladen gestart".to_string(),
                    );
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
                self.last_scan_found = drives.len();
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
            Event::BurnStarted {
                index,
                total_sectors,
                simulate,
            } => {
                self.burn_led = JobLed::Busy;
                // Vervang een evt. oude entry voor dit station (hoort niet
                // voor te komen, maar robustheid eerste).
                self.active_burns.retain(|b| b.index != index);
                self.active_burns.push(ActiveBurn {
                    index,
                    sector: 0,
                    sectors: total_sectors,
                    kbps: 0.0,
                    buffer_pct: 0.0,
                    fifo_pct: 0.0,
                    simulate,
                    phase: "starten…".to_string(),
                    elapsed_secs: 0.0,
                    eta_secs: 0.0,
                });
            }
            Event::BurnProgress {
                index,
                sector,
                sectors,
                kbps,
                buffer_pct,
                fifo_pct,
                phase,
                elapsed_secs,
                eta_secs,
            } => {
                if let Some(b) = self
                    .active_burns
                    .iter_mut()
                    .find(|b| b.index == index)
                {
                    b.sector = sector;
                    b.sectors = sectors;
                    b.kbps = kbps;
                    b.buffer_pct = buffer_pct;
                    b.fifo_pct = fifo_pct;
                    b.phase = phase;
                    b.elapsed_secs = elapsed_secs;
                    b.eta_secs = eta_secs;
                }
            }
            Event::BurnDone { index } => {
                self.burn_led = JobLed::Ok;
                self.active_burns.retain(|b| b.index != index);
                // Bestandenlijst wissen na een geslaagde brand — een nieuwe
                // run begint met een schone selectie — en het volumelabel door
                // zetten naar het volgende nummer van vandaag.
                if !self.burn_files.is_empty() {
                    self.burn_files.clear();
                    self.burn_files_size = 0;
                    self.log.push(
                        Level::Info,
                        "Bestandenlijst gewist na geslaagde brand".to_string(),
                    );
                }
                self.bump_volume_label();
            }
            Event::BurnFailed { index, error } => {
                self.burn_led = JobLed::Error;
                self.active_burns.retain(|b| b.index != index);
                if let Some(d) = self.drives.get_mut(index) {
                    d.inspect_error = Some(error);
                }
            }
            Event::BurnCancelled { index } => {
                self.active_burns.retain(|b| b.index != index);
                if self.active_burns.is_empty() {
                    self.burn_led = JobLed::Idle;
                }
            }
            Event::MaintStarted { kind, index } => {
                self.active_maints
                    .retain(|m| !(m.kind == kind && m.index == index));
                self.active_maints.push(MaintJob { kind, index, pct: 0.0 });
            }
            Event::MaintProgress { kind, index, pct } => {
                if let Some(m) = self
                    .active_maints
                    .iter_mut()
                    .find(|m| m.kind == kind && m.index == index)
                {
                    m.pct = pct;
                }
            }
            Event::MaintDone { index, .. } => {
                self.active_maints.retain(|m| m.index != index);
                // Na wissen/formatteren is de getoonde mediastatus verouderd;
                // automatisch opnieuw inspecteren voor verse gegevens.
                self.log.push(
                    Level::Info,
                    "Automatische herinspectie na onderhoud…".to_string(),
                );
                self.request_inspect(index);
            }
            Event::MaintFailed { index, error, .. } => {
                self.active_maints.retain(|m| m.index != index);
                if let Some(d) = self.drives.get_mut(index) {
                    d.inspect_error = Some(error);
                }
            }
            Event::MaintCancelled { index, .. } => {
                self.active_maints.retain(|m| m.index != index);
            }
            Event::MediaEjected { index } => {
                if let Some(d) = self.drives.get_mut(index) {
                    d.media = None;
                    d.inspect_error = None;
                }
                self.log
                    .push(Level::Info, "Mediagegevens gewist na eject".to_string());
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
            || self.active_read.is_some()
            || !self.active_burns.is_empty()
            || !self.active_maints.is_empty();
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

    /// Stap 8: instellingen persistent opslaan (eframe schrijft dit weg naar
    /// de config-map van de gebruiker bij afsluiten).
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &PersistedState::from_app(self));
    }
}

/// Pad van het sessielogbestand (stap 9a): `$XDG_DATA_HOME` of
/// `$HOME/.local/share` + `/libburn_gui/logs/sessie-<tijdstempel>.log`.
fn session_log_file() -> Option<std::path::PathBuf> {
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| format!("{h}/.local/share"))
        })?;
    let ts = chrono::Local::now()
        .format("sessie-%Y%m%d-%H%M%S.log")
        .to_string();
    Some(
        std::path::PathBuf::from(base)
            .join("libburn_gui")
            .join("logs")
            .join(ts),
    )
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
