//! Achtergrondworker: voert ALLE libburn-aanroepen uit op één thread.
//!
//! libburn is niet ontworpen voor gelijktijdige aanroepen; daarom praat de GUI
//! nooit rechtstreeks met de library maar verstuurt commando's naar deze worker
//! en ontvangt events terug. Elke stap wordt als log-event naar de UI gestuurd
//! voor duidelijke feedback, en elke event laat de UI hervappen.

use std::collections::VecDeque;
use std::ffi::{c_char, c_int};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::ffi::{
    BURN_DRIVE_ADR_LEN, DiscStatus, DriveInfo, DriveStatus, Progress, RawLibburn, SpeedDescriptor,
    cbuf_to_string,
};
use crate::logger::Level;

/// 1× CD-snelheid ≈ 176,4 kB/s (75 sectoren × 2352 bytes).
pub const CD_1X_KBPS: f32 = 176.4;
/// 1× DVD-snelheid ≈ 1385 kB/s.
pub const DVD_1X_KBPS: f32 = 1385.0;
/// 1× BD-snelheid ≈ 4500 kB/s.
pub const BD_1X_KBPS: f32 = 4500.0;

/// Capability-vlaggen uit `burn_drive_info.flags` (bitvelden, LSB-first).
#[derive(Clone, Copy, Debug, Default)]
pub struct DriveCaps {
    pub read_dvdram: bool,
    pub read_dvdr: bool,
    pub read_dvdrom: bool,
    pub read_cdr: bool,
    pub read_cdrw: bool,
    pub write_dvdram: bool,
    pub write_dvdr: bool,
    pub write_cdr: bool,
    pub write_cdrw: bool,
    pub write_simulate: bool,
    pub c2_errors: bool,
}

/// Eén snelheidsdescriptor uit de lijst van de drive.
#[derive(Clone, Debug)]
pub struct SpeedEntry {
    pub source: i32,
    pub profile_loaded: i32,
    pub profile_name: String,
    pub end_lba: i32,
    /// kB/s (libburn-eenheid: 1000 bytes/s).
    pub write_speed: i32,
    pub read_speed: i32,
}

/// Resultaat van een media-inspectie (grab → status → snelheden → release).
#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub disc_status: DiscStatus,
    pub drive_status: DriveStatus,
    pub speeds: Vec<SpeedEntry>,
}

/// Kopie van een `burn_drive_info` — bevat geen pointers naar libburn,
/// zodat de UI er veilig mee kan werken.
#[derive(Clone, Debug)]
pub struct DriveEntry {
    pub index: usize,
    pub vendor: String,
    pub product: String,
    pub revision: String,
    pub adr: String,
    pub buffer_size_kb: i32,
    pub caps: DriveCaps,
    pub tao_block_types: i32,
    pub sao_block_types: i32,
    pub raw_block_types: i32,
    pub packet_block_types: i32,
    pub media: Option<MediaInfo>,
    /// Reden waarom de laatste inspectie mislukte (voor feedback in de UI).
    pub inspect_error: Option<String>,
}

impl DriveEntry {
    pub fn display_name(&self) -> String {
        let name = format!("{} {}", self.vendor, self.product)
            .trim()
            .to_string();
        if name.is_empty() {
            format!("Station {}", self.index)
        } else {
            name
        }
    }
}

/// Commando's van de GUI naar de worker.
pub enum Command {
    /// `None` = automatisch zoeken (`LIBBURN_SO`, daarna standaardnamen).
    LoadLibrary(Option<String>),
    Scan,
    InspectDrive(usize),
    Shutdown,
}

/// Events van de worker naar de GUI.
pub enum Event {
    Log(Level, String),
    LibLoaded { version: String, path: String },
    LibLoadFailed { error: String },
    ScanStarted,
    ScanDone { drives: Vec<DriveEntry> },
    ScanFailed { error: String },
    InspectStarted { index: usize },
    InspectDone { index: usize, media: MediaInfo },
    InspectFailed { index: usize, error: String },
    WorkerStopped,
}

/// Start de worker-thread.
pub fn spawn(cmds: Receiver<Command>, events: Sender<Event>, ctx: egui::Context) -> JoinHandle<()> {
    thread::Builder::new()
        .name("libburn-worker".to_string())
        .spawn(move || run(cmds, events, ctx))
        .expect("libburn-worker-thread starten")
}

/// Stuurt een event naar de UI en wekt de UI (anders merkt de GUI pas bij de
/// volgende muisbeweging dat er iets gebeurd is).
struct Notifier<'a> {
    tx: &'a Sender<Event>,
    ctx: egui::Context,
}

impl Notifier<'_> {
    fn send(&self, ev: Event) {
        let _ = self.tx.send(ev);
        self.ctx.request_repaint();
    }

    fn log(&self, level: Level, msg: impl Into<String>) {
        self.send(Event::Log(level, msg.into()));
    }
}

struct WorkerState {
    raw: RawLibburn,
    infos: *mut DriveInfo,
    n_drives: usize,
}

fn run(cmds: Receiver<Command>, events: Sender<Event>, ctx: egui::Context) {
    let notify = Notifier { tx: &events, ctx };
    let mut state: Option<WorkerState> = None;
    let mut pending: VecDeque<Command> = VecDeque::new();
    let mut shutdown = false;

    while !shutdown {
        let cmd = match pending.pop_front() {
            Some(c) => c,
            None => match cmds.recv_timeout(Duration::from_millis(200)) {
                Ok(c) => c,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            },
        };
        match cmd {
            Command::LoadLibrary(path) => load_library(&mut state, path, &notify),
            Command::Scan => scan(&mut state, &cmds, &mut pending, &notify),
            Command::InspectDrive(index) => inspect(state.as_ref(), index, &notify),
            Command::Shutdown => shutdown = true,
        }
    }

    shutdown_lib(state, &notify);
    let _ = events.send(Event::WorkerStopped);
}

fn load_library(state: &mut Option<WorkerState>, custom: Option<String>, notify: &Notifier) {
    // Een eventueel eerdere sessie netjes afsluiten.
    shutdown_lib(state.take(), notify);

    let candidates: Vec<String> = match custom {
        Some(p) => vec![p],
        None => {
            let mut v = Vec::new();
            if let Ok(env) = std::env::var("LIBBURN_SO") {
                if !env.is_empty() {
                    v.push(env);
                }
            }
            v.push("libburn.so.4".to_string());
            v.push("libburn.so".to_string());
            v
        }
    };

    let mut last_err = String::new();
    for cand in &candidates {
        match RawLibburn::load(cand) {
            Ok(raw) => unsafe {
                if (raw.initialize)() == 0 {
                    last_err = format!("`{cand}`: burn_initialize() mislukte");
                    continue;
                }
                let (mut maj, mut min, mut mic) = (0, 0, 0);
                (raw.version)(&mut maj, &mut min, &mut mic);
                let version = format!("{maj}.{min}.{mic}");
                notify.log(
                    Level::Success,
                    format!("libburn {version} geladen via `{cand}`"),
                );
                notify.send(Event::LibLoaded {
                    version,
                    path: cand.clone(),
                });
                *state = Some(WorkerState {
                    raw,
                    infos: std::ptr::null_mut(),
                    n_drives: 0,
                });
                return;
            },
            Err(e) => last_err = format!("`{cand}`: {e}"),
        }
    }

    notify.log(Level::Error, format!("libburn niet geladen — {last_err}"));
    notify.send(Event::LibLoadFailed { error: last_err });
}

fn scan(
    state: &mut Option<WorkerState>,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    let Some(st) = state.as_mut() else {
        notify.log(
            Level::Warning,
            "Scan aangevraagd, maar libburn is niet geladen",
        );
        return;
    };

    // Volgens libburn.h verplicht: de oude drive-info-array vrijgeven vóór een
    // nieuwe scan (alle pointers worden ongeldig).
    free_drive_list(st);

    notify.log(Level::Info, "Scannen naar schijfstations…");
    notify.send(Event::ScanStarted);

    let mut infos: *mut DriveInfo = std::ptr::null_mut();
    let mut n: u32 = 0;

    // burn_drive_scan() moet herhaaldelijk aangeroepen worden tot hij ongelijk
    // nul teruggeeft (0 = scan nog niet klaar).
    enum Outcome {
        Done(c_int),
        Aborted,
    }
    let outcome = loop {
        let ret = unsafe { (st.raw.drive_scan)(&mut infos, &mut n) };
        if ret != 0 {
            break Outcome::Done(ret);
        }
        match cmds.try_recv() {
            Ok(Command::Shutdown) => {
                pending.push_front(Command::Shutdown);
                break Outcome::Aborted;
            }
            Ok(other) => pending.push_back(other),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break Outcome::Aborted,
        }
        thread::sleep(Duration::from_millis(40));
    };

    match outcome {
        Outcome::Aborted => return,
        Outcome::Done(ret) if ret < 0 => {
            let msg = format!("Scan mislukt (libburn-foutcode {ret})");
            notify.log(Level::Error, msg.clone());
            notify.send(Event::ScanFailed { error: msg });
            return;
        }
        Outcome::Done(_) => {}
    }

    let count = n as usize;
    let drives: Vec<DriveEntry> = if count > 0 && !infos.is_null() {
        unsafe { std::slice::from_raw_parts(infos, count) }
            .iter()
            .enumerate()
            .map(|(i, di)| drive_entry_from_raw(i, di, &st.raw))
            .collect()
    } else {
        Vec::new()
    };

    st.infos = infos;
    st.n_drives = count;

    notify.log(
        Level::Success,
        format!("Scan voltooid: {count} station(s) gevonden"),
    );
    notify.send(Event::ScanDone { drives });
}

fn drive_entry_from_raw(index: usize, di: &DriveInfo, raw: &RawLibburn) -> DriveEntry {
    let f = di.flags as u32;
    let bit = |n: u32| (f >> n) & 1 != 0;

    let adr = unsafe { adr_of(di, raw) };

    DriveEntry {
        index,
        vendor: cbuf_to_string(&di.vendor),
        product: cbuf_to_string(&di.product),
        revision: cbuf_to_string(&di.revision),
        adr,
        buffer_size_kb: di.buffer_size,
        caps: DriveCaps {
            read_dvdram: bit(0),
            read_dvdr: bit(1),
            read_dvdrom: bit(2),
            read_cdr: bit(3),
            read_cdrw: bit(4),
            write_dvdram: bit(5),
            write_dvdr: bit(6),
            write_cdr: bit(7),
            write_cdrw: bit(8),
            write_simulate: bit(9),
            c2_errors: bit(10),
        },
        tao_block_types: di.tao_block_types,
        sao_block_types: di.sao_block_types,
        raw_block_types: di.raw_block_types,
        packet_block_types: di.packet_block_types,
        media: None,
        inspect_error: None,
    }
}

/// Vraag het device-adres op via `burn_drive_get_adr` (BURN_DRIVE_ADR_LEN-buffer).
unsafe fn adr_of(di: &DriveInfo, raw: &RawLibburn) -> String {
    let mut buf = [0 as c_char; BURN_DRIVE_ADR_LEN];
    // Edition 2024: ook binnen een `unsafe fn` moet elke onveilige aanroep
    // expliciet in een `unsafe`-blok.
    let ret =
        unsafe { (raw.drive_get_adr)(di as *const DriveInfo as *mut DriveInfo, buf.as_mut_ptr()) };
    if ret > 0 {
        cbuf_to_string(&buf)
    } else {
        String::new()
    }
}

fn inspect(state: Option<&WorkerState>, index: usize, notify: &Notifier) {
    let Some(st) = state else {
        notify.log(
            Level::Warning,
            "Inspectie aangevraagd, maar libburn is niet geladen",
        );
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(
            Level::Warning,
            format!("Inspectie aangevraagd voor onbekend station {index}"),
        );
        return;
    }

    notify.send(Event::InspectStarted { index });
    let di = unsafe { *st.infos.add(index) };
    let name = format!(
        "{} {}",
        cbuf_to_string(&di.vendor),
        cbuf_to_string(&di.product)
    )
    .trim()
    .to_string();
    notify.log(Level::Info, format!("Station {index} ({name}): grabben…"));

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let msg = "Grab mislukt — station mogelijk in gebruik door een ander programma";
        notify.log(Level::Error, format!("Station {index}: {msg}"));
        notify.send(Event::InspectFailed {
            index,
            error: msg.to_string(),
        });
        return;
    }
    notify.log(
        Level::Info,
        format!("Station {index}: grab gelukt; media-status bepalen…"),
    );

    // burn_disc_get_status geeft UNREADY terug zolang de drive nog bezig is;
    // een paar keer pollen met korte pauzes (max. ~2 s).
    let mut disc = DiscStatus::Unready;
    for _ in 0..40 {
        disc = DiscStatus::from_raw(unsafe { (st.raw.disc_get_status)(di.drive) });
        if disc != DiscStatus::Unready {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    notify.log(
        Level::Info,
        format!("Station {index}: media = {}", disc_label(disc)),
    );

    // Snelheden: burn_drive_get_speedlist geeft een kopie van de lijst terug,
    // die we via .next doorlopen en daarna vrijgeven.
    let mut speeds = Vec::new();
    let mut list: *mut SpeedDescriptor = std::ptr::null_mut();
    let r = unsafe { (st.raw.drive_get_speedlist)(di.drive, &mut list) };
    if r > 0 && !list.is_null() {
        let mut p = list;
        while !p.is_null() {
            let d = unsafe { *p };
            speeds.push(SpeedEntry {
                source: d.source,
                profile_loaded: d.profile_loaded,
                profile_name: cbuf_to_string(&d.profile_name),
                end_lba: d.end_lba,
                write_speed: d.write_speed,
                read_speed: d.read_speed,
            });
            p = d.next;
        }
        unsafe { (st.raw.drive_free_speedlist)(&mut list) };
        notify.log(
            Level::Info,
            format!("Station {index}: {} snelheid(s) gelezen", speeds.len()),
        );
    }

    let mut prog = Progress::default();
    let drive_status =
        DriveStatus::from_raw(unsafe { (st.raw.drive_get_status)(di.drive, &mut prog) });

    unsafe { (st.raw.drive_release)(di.drive, 0) };
    notify.log(
        Level::Success,
        format!("Station {index}: inspectie klaar, station vrijgegeven"),
    );
    notify.send(Event::InspectDone {
        index,
        media: MediaInfo {
            disc_status: disc,
            drive_status,
            speeds,
        },
    });
}

fn free_drive_list(st: &mut WorkerState) {
    if !st.infos.is_null() {
        unsafe { (st.raw.drive_info_free)(st.infos) };
        st.infos = std::ptr::null_mut();
        st.n_drives = 0;
    }
}

fn shutdown_lib(state: Option<WorkerState>, notify: &Notifier) {
    let Some(mut st) = state else { return };
    free_drive_list(&mut st);
    unsafe { (st.raw.finish)() };
    notify.log(Level::Info, "libburn afgesloten (burn_finish)");
}

/// Nederlandse omschrijving van `burn_disc_status`.
pub fn disc_label(s: DiscStatus) -> &'static str {
    match s {
        DiscStatus::Unready => "nog niet bekend",
        DiscStatus::Blank => "leeg — klaar om te beschrijven",
        DiscStatus::Empty => "geen schijf",
        DiscStatus::Appendable => "onvolledig — extra sessie mogelijk",
        DiscStatus::Full => "vol / afgesloten (alleen lezen)",
        DiscStatus::Ungrabbed => "niet gegrabbed (interne fout)",
        DiscStatus::Unsuitable => "onbruikbare media",
        DiscStatus::Unknown(_) => "onbekend",
    }
}

/// Nederlandse omschrijving van `burn_drive_status`.
pub fn drive_status_label(s: DriveStatus) -> &'static str {
    match s {
        DriveStatus::Idle => "inactief",
        DriveStatus::Spawning => "bezig met starten",
        DriveStatus::Reading => "lezen",
        DriveStatus::Writing => "schrijven",
        DriveStatus::WritingLeadin => "lead-in schrijven",
        DriveStatus::WritingLeadout => "lead-out schrijven",
        DriveStatus::Erasing => "wissen",
        DriveStatus::Grabbing => "grabben",
        DriveStatus::WritingPregap => "pregap schrijven",
        DriveStatus::ClosingTrack => "track afsluiten",
        DriveStatus::ClosingSession => "sessie afsluiten",
        DriveStatus::Formatting => "formatteren",
        DriveStatus::ReadingSync => "synchroon lezen",
        DriveStatus::WritingSync => "synchroon schrijven",
        DriveStatus::Other(_) => "onbekend",
    }
}

/// Bron van een snelheidsdescriptor (zie libburn.h).
pub fn speed_source_label(source: i32) -> &'static str {
    match source {
        1 => "mode page 2Ah",
        2 => "GET PERFORMANCE",
        3 => "GET PERFORMANCE (lees)",
        _ => "overig",
    }
}

/// Zet kB/s om naar een leesbare ×CD/×DVD/×BD-indicatie op basis van het
/// actieve SCSI-profiel (CD: 0x08–0x0A, DVD: 0x10–0x2F, BD: 0x40–0x42).
pub fn speed_multiplier_label(profile_loaded: i32, kbps: i32) -> String {
    if kbps <= 0 {
        return "—".to_string();
    }
    let (unit, base) = if (0x08..=0x0A).contains(&profile_loaded) {
        ("CD", CD_1X_KBPS)
    } else if (0x10..=0x2F).contains(&profile_loaded) {
        ("DVD", DVD_1X_KBPS)
    } else if (0x40..=0x42).contains(&profile_loaded) {
        ("BD", BD_1X_KBPS)
    } else {
        return "—".to_string();
    };
    format!("≈ {:.1}× {unit}", kbps as f32 / base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    use crate::ffi::tests::LIBBURN_LOCK;

    /// Headless rooktest van de volledige worker-flow: library laden → scannen
    /// → drive-info converteren (controleert de struct-layout tegen de echte
    /// library). Slaat over als libburn niet geïnstalleerd is.
    #[test]
    fn worker_load_and_scan() {
        let _guard = LIBBURN_LOCK.lock().unwrap();

        let (cmd_tx, cmd_rx) = channel::<Command>();
        let (event_tx, event_rx) = channel::<Event>();
        let _handle = spawn(cmd_rx, event_tx, egui::Context::default());

        cmd_tx.send(Command::LoadLibrary(None)).unwrap();
        cmd_tx.send(Command::Scan).unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut terminal = false;
        while std::time::Instant::now() < deadline {
            match event_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Event::LibLoaded { version, path }) => {
                    eprintln!("libburn {version} geladen via {path}");
                }
                Ok(Event::ScanDone { drives }) => {
                    eprintln!("ScanDone: {} station(s)", drives.len());
                    for d in &drives {
                        eprintln!("  - {} @ {}", d.display_name(), d.adr);
                    }
                    terminal = true;
                    break;
                }
                Ok(Event::ScanFailed { error }) => {
                    eprintln!("scan mislukt (omgeving): {error}");
                    terminal = true;
                    break;
                }
                Ok(Event::LibLoadFailed { error }) => {
                    eprintln!("overgeslagen: libburn niet beschikbaar ({error})");
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        drop(cmd_tx); // worker netjes laten stoppen
        assert!(terminal, "worker moest met ScanDone of ScanFailed eindigen");
    }
}
