//! Achtergrondworker: voert ALLE libburn-aanroepen uit op één thread.
//!
//! libburn is niet ontworpen voor gelijktijdige aanroepen; daarom praat de GUI
//! nooit rechtstreeks met de library maar verstuurt commando's naar deze worker
//! en ontvangt events terug. Elke stap wordt als log-event naar de UI gestuurd
//! voor duidelijke feedback, en elke event laat de UI hervappen.

use std::collections::VecDeque;
use std::ffi::{c_char, c_int, c_longlong, c_void};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::ffi;
use crate::ffi::{
    BURN_DRIVE_ADR_LEN, BURN_MSGS_MESSAGE_LEN, DiscStatus, DriveInfo, DriveStatus, Progress,
    RawLibburn, SpeedDescriptor, cbuf_to_string,
};
use crate::isofs::{ISO_MSGS_MESSAGE_LEN, ISO_SUCCESS, RawLibisofs};
use crate::logger::Level;
use crate::settings::BurnSettings;

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

/// Eén track uit de TOC van de ingelegde schijf.
#[derive(Clone, Debug)]
pub struct TocTrack {
    pub session: u32,
    pub track_no: u32,
    pub start_lba: i32,
    /// Aantal blokken (2048 bytes per blok); 0 = onbekend.
    pub blocks: i32,
    /// `control`-bit2 uit de TOC: true = datatrack, false = audio.
    pub is_data: bool,
    pub copy_permitted: bool,
}

/// Eén complete sessie uit de TOC.
#[derive(Clone, Debug)]
pub struct TocSession {
    pub index: usize,
    pub start_lba: i32,
    /// Eindadres volgens de lead-out (exclusief); -1 = onbekend.
    pub end_lba: i32,
    pub tracks: Vec<TocTrack>,
}

/// Resultaat van een media-inspectie (grab → status → profiel → TOC → snelheden → release).
#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub disc_status: DiscStatus,
    pub drive_status: DriveStatus,
    pub speeds: Vec<SpeedEntry>,
    /// Stap 2: SCSI-profielnummer (0 = geen media / onbekend).
    pub profile_no: i32,
    pub profile_name: String,
    /// Leesbare capaciteit in blokken van 2048 bytes (`burn_get_read_capacity`).
    pub read_capacity_blocks: Option<i32>,
    /// Herbeschrijfbaar (`burn_disc_erasable`).
    pub erasable: bool,
    pub sessions: Vec<TocSession>,
    pub incomplete_sessions: i32,
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
    /// `exclusive` = drives exclusief openen (O_EXCL); uitzetten als de
    /// bestandsbeheerder de schijf heeft aangekoppeld (automount).
    LoadLibrary {
        path: Option<String>,
        exclusive: bool,
    },
    Scan,
    InspectDrive(usize),
    /// Maak een schijfkopie (data-blokken) naar een bestand.
    ReadDisc {
        index: usize,
        path: String,
    },
    /// Bezig lopende kopie annuleren.
    CancelRead,
    /// Brand een ISO-bestand naar het station (instellingen uit de GUI).
    BurnDisc {
        index: usize,
        path: String,
        settings: BurnSettings,
    },
    /// Stel een data-image samen uit bestanden/mappen (libisofs) en brand die.
    BurnFiles {
        index: usize,
        paths: Vec<String>,
        volume_id: String,
        settings: BurnSettings,
    },
    /// Wis de media los van een brandjob (stap 6).
    EraseDisc {
        index: usize,
        fast: bool,
    },
    /// Formatteer de media los van een brandjob (stap 6).
    FormatDisc {
        index: usize,
    },
    /// Bezig lopende brandjob annuleren.
    CancelBurn,
    Shutdown,
}

/// Events van de worker naar de GUI.
pub enum Event {
    Log(Level, String),
    LibLoaded {
        version: String,
        path: String,
    },
    LibLoadFailed {
        error: String,
    },
    ScanStarted,
    ScanDone {
        drives: Vec<DriveEntry>,
    },
    ScanFailed {
        error: String,
    },
    InspectStarted {
        index: usize,
    },
    InspectDone {
        index: usize,
        media: MediaInfo,
    },
    InspectFailed {
        index: usize,
        error: String,
    },
    ReadStarted {
        index: usize,
        total_blocks: i32,
    },
    ReadProgress {
        index: usize,
        blocks_done: i64,
        total_blocks: i32,
        kbps: f64,
    },
    ReadDone,
    ReadFailed {
        index: usize,
        error: String,
    },
    ReadCancelled,
    BurnStarted {
        index: usize,
        total_sectors: i32,
        simulate: bool,
    },
    BurnProgress {
        index: usize,
        sector: i32,
        sectors: i32,
        kbps: f64,
        buffer_pct: f32,
        fifo_pct: f32,
        /// Huidige fase (bijv. "schrijven", "track afsluiten") voor de UI.
        phase: String,
        /// Verstreken tijd in seconden.
        elapsed_secs: f64,
        /// Verwachte resterende tijd in seconden (0 = onbekend).
        eta_secs: f64,
    },
    BurnDone,
    BurnFailed {
        index: usize,
        error: String,
    },
    BurnCancelled,
    /// Onderhoudsjob (stap 6): wissen of formatteren los van het branden.
    MaintStarted {
        kind: MaintKind,
        index: usize,
    },
    MaintProgress {
        kind: MaintKind,
        index: usize,
        pct: f32,
    },
    MaintDone {
        #[allow(dead_code)]
        kind: MaintKind,
        #[allow(dead_code)]
        index: usize,
    },
    MaintFailed {
        #[allow(dead_code)]
        kind: MaintKind,
        index: usize,
        error: String,
    },
    MaintCancelled {
        #[allow(dead_code)]
        kind: MaintKind,
        #[allow(dead_code)]
        index: usize,
    },
    WorkerStopped,
}

/// Soort onderhoudsjob.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaintKind {
    Erase,
    Format,
}

impl MaintKind {
    pub fn label(self) -> &'static str {
        match self {
            MaintKind::Erase => "wissen",
            MaintKind::Format => "formatteren",
        }
    }
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
    /// libisofs (optioneel): nodig voor “bestanden samenstellen”.
    isofs: Option<RawLibisofs>,
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
            Command::LoadLibrary { path, exclusive } => {
                load_library(&mut state, path, exclusive, &notify)
            }
            Command::Scan => scan(&mut state, &cmds, &mut pending, &notify),
            Command::InspectDrive(index) => inspect(state.as_ref(), index, &notify),
            Command::ReadDisc { index, path } => {
                read_disc(&state, index, &path, &cmds, &mut pending, &notify)
            }
            Command::CancelRead => {
                notify.log(
                    Level::Warning,
                    "Annuleren gevraagd, maar er is geen kopie bezig",
                );
            }
            Command::BurnDisc {
                index,
                path,
                settings,
            } => burn_job(
                &state,
                index,
                &path,
                &settings,
                &cmds,
                &mut pending,
                &notify,
            ),
            Command::BurnFiles {
                index,
                paths,
                volume_id,
                settings,
            } => burn_files_job(
                &state,
                index,
                &paths,
                &volume_id,
                &settings,
                &cmds,
                &mut pending,
                &notify,
            ),
            Command::CancelBurn => {
                notify.log(
                    Level::Warning,
                    "Annuleren gevraagd, maar er is geen job bezig",
                );
            }
            Command::EraseDisc { index, fast } => {
                erase_job(&state, index, fast, &cmds, &mut pending, &notify)
            }
            Command::FormatDisc { index } => {
                format_job(&state, index, &cmds, &mut pending, &notify)
            }
            Command::Shutdown => shutdown = true,
        }
    }

    shutdown_lib(state, &notify);
    let _ = events.send(Event::WorkerStopped);
}

fn load_library(
    state: &mut Option<WorkerState>,
    custom: Option<String>,
    exclusive: bool,
    notify: &Notifier,
) {
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
                // Apparaat-openingsbeleid (vlak na initialize, vóór de scan):
                // exclusief (O_EXCL) of niet — uitzetten als de bestandsbeheerder
                // de schijf heeft aangekoppeld (automount).
                (raw.preset_device_open)(if exclusive { 1 } else { 0 }, 0, 0);
                // libburn-meldingen: alles vanaf UPDATE in de queue, niets
                // rechtstreeks naar stderr; de worker haalt de queue leeg.
                (raw.msgs_set_severities)(
                    c"UPDATE".as_ptr(),
                    c"NEVER".as_ptr(),
                    c"libburn_gui: ".as_ptr(),
                );
                notify.log(
                    Level::Info,
                    format!(
                        "Apparaat-openingsmodus: {}",
                        if exclusive {
                            "exclusief (O_EXCL)"
                        } else {
                            "niet-exclusief (geschikt bij automount)"
                        }
                    ),
                );
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
                // libisofs erbij laden (optioneel — alleen nodig voor
                // “bestanden samenstellen”; ISO-branden werkt ook zonder).
                let isofs = load_isofs(notify);
                *state = Some(WorkerState {
                    raw,
                    isofs,
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
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = format!(
            "Grab mislukt voor {} — drive mogelijk bezet. Mogelijke oorzaak: de schijf is \
             door de bestandsbeheerder aangekoppeld (automount). Los dit op met \
             `udisksctl unmount -b {adr}` of zet “Exclusief openen” uit \
             (Instellingen → Apparaat).",
            if adr.is_empty() {
                format!("station {index}")
            } else {
                adr.clone()
            },
            adr = if adr.is_empty() { "/dev/srX" } else { &adr },
        );
        notify.log(Level::Error, format!("Station {index}: {msg}"));
        notify.send(Event::InspectFailed { index, error: msg });
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

    // Stap 2: profiel (mediatype) opvragen.
    let mut profile_no: c_int = 0;
    let mut profile_buf = [0 as c_char; 80];
    let profile_ok =
        unsafe { (st.raw.disc_get_profile)(di.drive, &mut profile_no, profile_buf.as_mut_ptr()) };
    let profile_name = if profile_ok == 1 {
        let name = cbuf_to_string(&profile_buf);
        if name.is_empty() {
            profile_fallback_name(profile_no).to_string()
        } else {
            name
        }
    } else {
        String::new()
    };
    if !profile_name.is_empty() {
        notify.log(
            Level::Info,
            format!(
                "Station {index}: mediatype = {} (profiel 0x{:02X})",
                profile_name, profile_no
            ),
        );
    }

    // Leesbare capaciteit (kan falen op lege media — geen probleem).
    let mut capacity: c_int = 0;
    let read_capacity =
        if unsafe { (st.raw.get_read_capacity)(di.drive, &mut capacity, 0) } == 1 && capacity > 0 {
            Some(capacity)
        } else {
            None
        };
    if let Some(blocks) = read_capacity {
        notify.log(
            Level::Info,
            format!(
                "Station {index}: leesbare capaciteit ≈ {}",
                format_blocks(blocks)
            ),
        );
    }

    let erasable = unsafe { (st.raw.disc_erasable)(di.drive) } != 0;

    // Stap 2: TOC-model van de schijf opbouwen (alleen zinvol bij media met
    // inhoud; bij lege media geeft de drive NULL of een leeg model).
    let (sessions, incomplete) = unsafe { read_toc(&st.raw, di.drive) };
    if !sessions.is_empty() {
        let total_tracks: usize = sessions.iter().map(|s| s.tracks.len()).sum();
        notify.log(
            Level::Info,
            format!(
                "Station {index}: TOC gelezen — {} sessie(s), {} track(s)",
                sessions.len(),
                total_tracks
            ),
        );
    }

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
            profile_no: if profile_ok == 1 { profile_no } else { 0 },
            profile_name,
            read_capacity_blocks: read_capacity,
            erasable,
            sessions,
            incomplete_sessions: incomplete,
        },
    });
}

/// Bouwt het TOC-model van de ingelegde schijf op via
/// burn_drive_get_disc → burn_disc_get_sessions → burn_session_get_tracks
/// → burn_track_get_entry / burn_session_get_leadout_entry.
/// De schijf moet gegrabbed zijn; het disc-model wordt hier netjes vrijgegeven.
///
/// Semantiek (uit libburn-bron, mmc.c): bij een track is `start_lba` het begin
/// en `track_blocks` de grootte. Bij de lead-out (point 0xA2) is `start_lba`
/// het begin van de lead-out-zone (= einde van de trackdata) en is
/// `track_blocks` 0. De sessiegrens is dus: start = eerste track, einde =
/// leadout.start_lba (exclusief).
unsafe fn read_toc(raw: &RawLibburn, drive: *mut ffi::BurnDrive) -> (Vec<TocSession>, i32) {
    // Edition 2024: de body van een `unsafe fn` is zelf safe; elke onveilige
    // operatie staat hier expliciet in één `unsafe`-blok.
    unsafe {
        let disc = (raw.drive_get_disc)(drive);
        if disc.is_null() {
            return (Vec::new(), 0);
        }

        let mut sessions_out: Vec<TocSession> = Vec::new();
        let mut num_sessions: c_int = 0;
        let session_array = (raw.disc_get_sessions)(disc, &mut num_sessions);
        let incomplete = (raw.disc_get_incomplete_sessions)(disc);

        if !session_array.is_null() && num_sessions > 0 {
            for s in 0..num_sessions as usize {
                let session = *session_array.add(s);
                if session.is_null() {
                    continue;
                }

                // Lead-out: einde van de sessie (start_lba = einde trackdata).
                let mut leadout = ffi::TocEntry::default();
                (raw.session_get_leadout_entry)(session, &mut leadout);

                let mut tracks = Vec::new();
                let mut num_tracks: c_int = 0;
                let track_array = (raw.session_get_tracks)(session, &mut num_tracks);
                if !track_array.is_null() && num_tracks > 0 {
                    for t in 0..num_tracks as usize {
                        let track = *track_array.add(t);
                        if track.is_null() {
                            continue;
                        }
                        let mut entry = ffi::TocEntry::default();
                        (raw.track_get_entry)(track, &mut entry);
                        tracks.push(TocTrack {
                            session: entry.session as u32 | ((entry.session_msb as u32) << 8),
                            track_no: entry.point as u32 | ((entry.point_msb as u32) << 8),
                            start_lba: entry.start_lba,
                            blocks: entry.track_blocks,
                            is_data: entry.control & 0x04 != 0,
                            copy_permitted: entry.control & 0x02 != 0,
                        });
                    }
                }

                // Sessiestart: begin van de eerste track; zonder tracks vallen
                // we terug op de lead-out.
                let start = tracks
                    .first()
                    .map(|t| t.start_lba)
                    .unwrap_or(leadout.start_lba);

                sessions_out.push(TocSession {
                    index: s,
                    start_lba: start,
                    end_lba: leadout.start_lba,
                    tracks,
                });
            }
        }

        (raw.disc_free)(disc);
        (sessions_out, incomplete)
    }
}

/// Grootte van één lees-chunk: 1024 blokken = 2 MiB.
const READ_CHUNK_BLOCKS: i64 = 1024;

/// FIFO-grootte voor het branden: 2048 chunks × 2048 bytes = 4 MiB.
const BURN_FIFO_CHUNKS: i32 = 2048;

/// Haalt alle opgequeuede libburn-meldingen op en logt ze.
fn drain_msgs(raw: &RawLibburn, notify: &Notifier) {
    unsafe {
        let mut min = b"NEVER\0".to_vec();
        let mut msg = [0 as c_char; BURN_MSGS_MESSAGE_LEN];
        let mut sev = [0 as c_char; 80];
        let mut code: c_int = 0;
        let mut os_errno: c_int = 0;
        while (raw.msgs_obtain)(
            min.as_mut_ptr() as *mut c_char,
            &mut code,
            msg.as_mut_ptr(),
            &mut os_errno,
            sev.as_mut_ptr(),
        ) == 1
        {
            let text = cbuf_to_string(&msg);
            let sev_name = cbuf_to_string(&sev);
            let level = sev_to_level(&sev_name);
            notify.log(level, format!("[libburn/{sev_name}] {text}"));
        }
    }
}

fn sev_to_level(sev: &str) -> Level {
    match sev {
        "ABORT" | "FATAL" | "FAILURE" | "SORRY" => Level::Error,
        "WARNING" => Level::Warning,
        _ => Level::Info,
    }
}

fn free_drive_list(st: &mut WorkerState) {
    if !st.infos.is_null() {
        unsafe { (st.raw.drive_info_free)(st.infos) };
        st.infos = std::ptr::null_mut();
        st.n_drives = 0;
    }
}

/// Brandt een ISO-bestand naar het station.
///
/// Flow: grab → media check → (evt. wissen) → disc-model (session/track) met
/// file-bron + 4 MiB FIFO → write-opts (snelheid, type, simulatie, multi) →
/// burn_disc_write → poll status/voortgang → resultaat. Annuleren kan via
/// burn_drive_cancel.
fn burn_job(
    state: &Option<WorkerState>,
    index: usize,
    path: &str,
    s: &BurnSettings,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    use crate::settings::WriteMode;

    let Some(st) = state else {
        notify.log(
            Level::Warning,
            "Branden aangevraagd, maar libburn is niet geladen",
        );
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(
            Level::Warning,
            format!("Branden aangevraagd voor onbekend station {index}"),
        );
        return;
    }
    let path = path.trim();
    if path.is_empty() {
        notify.log(Level::Warning, "Branden aangevraagd zonder ISO-bestand");
        return;
    }
    if s.write_mode == WriteMode::Raw {
        let msg = "Schrijfmodus RAW wordt niet ondersteund voor ISO-bestanden \
                   (vereist 2352-byte brondata)";
        notify.log(Level::Error, msg);
        notify.send(Event::BurnFailed {
            index,
            error: msg.to_string(),
        });
        return;
    }

    // Bronbestand controleren.
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("Kan ISO-bestand `{path}` niet openen: {e}");
            notify.log(Level::Error, &msg);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };
    let file_size = meta.len();
    if file_size == 0 {
        let msg = "ISO-bestand is leeg".to_string();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    if file_size % 2048 != 0 {
        notify.log(
            Level::Info,
            format!(
                "Bestandsgrootte is geen veelvoud van 2048 bytes; laatste sector wordt \
                 aangevuld met nullen"
            ),
        );
    }
    let expected_sectors = ((file_size as i64 + 2047) / 2048) as i32;

    let di = unsafe { *st.infos.add(index) };
    notify.log(
        Level::Info,
        format!(
            "Station {index}: brandjob starten — `{path}` ({} blokken)",
            expected_sectors
        ),
    );
    notify.log(
        Level::Info,
        format!(
            "Instellingen: simulatie={}, multi-session={}, padding={} KiB, \
             wissen vooraf={}, overburn={}",
            if s.simulate { "aan" } else { "uit" },
            s.multi_session.label(),
            s.padding_kib,
            if s.blank_first { "aan" } else { "uit" },
            if s.overburn { "aan" } else { "uit" }
        ),
    );

    // Grab + media-check + evt. wissen (gedeelde flow met bestands-branden).
    // Grab + media-check + evt. wissen (gedeelde flow).
    if grab_and_check_media(st, &di, index, s, cmds, pending, notify).is_err() {
        return;
    }

    // Ruimtecheck (indicatief; de echte precheck doet libburn zelf).
    let avail = unsafe { (st.raw.disc_available_space)(di.drive, std::ptr::null_mut()) };
    if avail >= 0 && (avail as i64) < file_size as i64 {
        notify.log(
            Level::Warning,
            format!(
                "ISO-bestand (≈ {}) lijkt groter dan de beschikbare ruimte (≈ {})",
                format_blocks(((file_size as i64 + 2047) / 2048) as i32),
                format_blocks(avail as i32)
            ),
        );
    }

    // Snelheid instellen (k/s; 0 = max).
    let write_speed = if s.speed_max { 0 } else { s.speed_kbps };
    unsafe { (st.raw.drive_set_speed)(di.drive, 0, write_speed) };
    notify.log(
        Level::Info,
        format!(
            "Snelheid: {}",
            if s.speed_max {
                "maximaal".to_string()
            } else {
                format!("{} kB/s", s.speed_kbps)
            }
        ),
    );

    // Disc-model: disc → session → track met file-bron + FIFO.
    let path_c = match std::ffi::CString::new(path) {
        Ok(c) => c,
        Err(_) => {
            let msg = "Ongeldig pad (bevat NUL-byte)".to_string();
            notify.log(Level::Error, &msg);
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };
    let (disc, session, track, fifo, opts) =
        match unsafe { build_burn_model(&st.raw, di.drive, path_c.as_ptr(), file_size, s, notify) }
        {
            Ok(model) => model,
            Err(msg) => {
                unsafe { (st.raw.drive_release)(di.drive, 0) };
                notify.send(Event::BurnFailed { index, error: msg });
                return;
            }
        };

    // Branden starten en poll-lus draaien (gedeelde helper).
    unsafe {
        run_write_poll(
            st,
            di.drive,
            opts,
            disc,
            session,
            track,
            fifo,
            index,
            expected_sectors,
            s,
            cmds,
            pending,
            notify,
        );
    }
}

/// Stelt een data-image samen uit bestanden/mappen (libisofs) en brandt die
/// direct naar het station — `iso_image_create_burn_source()` levert een
/// burn_source die aan de libburn-track wordt gekoppeld (geen tussenbestand).
#[allow(clippy::too_many_arguments)]
fn burn_files_job(
    state: &Option<WorkerState>,
    index: usize,
    paths: &[String],
    volume_id: &str,
    s: &BurnSettings,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    use crate::settings::WriteMode;

    let Some(st) = state else {
        notify.log(
            Level::Warning,
            "Branden aangevraagd, maar libburn is niet geladen",
        );
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(
            Level::Warning,
            format!("Branden aangevraagd voor onbekend station {index}"),
        );
        return;
    }
    let Some(iso) = st.isofs.as_ref() else {
        let msg =
            "libisofs is niet geladen — bestanden samenstellen is niet beschikbaar".to_string();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    };
    if paths.is_empty() {
        notify.log(Level::Warning, "Geen bestanden gekozen om te branden");
        return;
    }
    if s.write_mode == WriteMode::Raw {
        let msg = "Schrijfmodus RAW wordt niet ondersteund voor data-images".to_string();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    for p in paths {
        if !std::path::Path::new(p).exists() {
            let msg = format!("Bestand/map bestaat niet: `{p}`");
            notify.log(Level::Error, &msg);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    }

    let di = unsafe { *st.infos.add(index) };
    notify.log(
        Level::Info,
        format!(
            "Station {index}: data-image samenstellen uit {} item(s)…",
            paths.len()
        ),
    );
    notify.log(
        Level::Info,
        format!(
            "Instellingen: simulatie={}, multi-session={}, padding={} KiB, \
             wissen vooraf={}, overburn={}, tijdstempels={}",
            if s.simulate { "aan" } else { "uit" },
            s.multi_session.label(),
            s.padding_kib,
            if s.blank_first { "aan" } else { "uit" },
            if s.overburn { "aan" } else { "uit" },
            if s.keep_timestamps {
                "behouden"
            } else {
                "opnametijd"
            }
        ),
    );

    // Grab + media-check + evt. wissen (gedeelde flow).
    if grab_and_check_media(st, &di, index, s, cmds, pending, notify).is_err() {
        return;
    }

    // Snelheid instellen (k/s; 0 = max).
    let write_speed = if s.speed_max { 0 } else { s.speed_kbps };
    unsafe { (st.raw.drive_set_speed)(di.drive, 0, write_speed) };
    notify.log(
        Level::Info,
        format!(
            "Snelheid: {}",
            if s.speed_max {
                "maximaal".to_string()
            } else {
                format!("{} kB/s", s.speed_kbps)
            }
        ),
    );

    // ISO-image opbouwen in libisofs.
    let vol = if volume_id.trim().is_empty() {
        format!("Libburn{}", chrono::Local::now().format("%Y%m%d"))
    } else {
        volume_id.trim().to_string()
    };
    let vol_c = match std::ffi::CString::new(vol.as_str()) {
        Ok(c) => c,
        Err(_) => {
            let msg = "Ongeldige volumenaam (bevat NUL-byte)".to_string();
            notify.log(Level::Error, &msg);
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };
    let image = unsafe {
        let mut image: *mut crate::isofs::IsoImage = std::ptr::null_mut();
        if (iso.image_new)(vol_c.as_ptr(), &mut image) != ISO_SUCCESS || image.is_null() {
            let msg = "Kan de ISO-image niet aanmaken".to_string();
            notify.log(Level::Error, &msg);
            (st.raw.drive_release)(di.drive, 0);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
        image
    };

    // Bestanden/mappen toevoegen (met unieke namen bij dubbele basenames).
    // Mappen: eerst een map-node met de mapnaam, daarna de inhoud recursief
    // met iso_tree_add_dir_rec — iso_tree_add_new_node voegt alleen de lege
    // map-node zelf toe.
    let mut add_errors: Vec<String> = Vec::new();
    unsafe {
        let root = (iso.image_get_root)(image);
        let mut used_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for p in paths {
            let base = std::path::Path::new(p)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("item{}", used_names.len() + 1));
            let mut name = base.clone();
            let mut n = 1;
            while !used_names.insert(name.clone()) {
                n += 1;
                name = format!("{base} ({n})");
            }
            let path_c = match std::ffi::CString::new(p.as_str()) {
                Ok(c) => c,
                Err(_) => {
                    add_errors.push(format!("{p}: ongeldig pad (NUL-byte)"));
                    continue;
                }
            };
            let name_c = match std::ffi::CString::new(name) {
                Ok(c) => c,
                Err(_) => {
                    add_errors.push(format!("{p}: ongeldige naam"));
                    continue;
                }
            };
            if std::path::Path::new(p).is_dir() {
                // Map met de gekozen naam aanmaken en recursief vullen.
                // NB: iso_tree_add_new_dir geeft bij succes het AANTAL nodes in
                // de parent terug (geen ISO_SUCCESS); < 0 is een fout.
                let mut dir: *mut crate::isofs::IsoDir = std::ptr::null_mut();
                let r = (iso.tree_add_new_dir)(root, name_c.as_ptr(), &mut dir);
                if r < 0 || dir.is_null() {
                    add_errors.push(format!("{p}: map toevoegen mislukt (code {r})"));
                    continue;
                }
                let r = (iso.tree_add_dir_rec)(image, dir, path_c.as_ptr());
                if r < 0 {
                    add_errors.push(format!("{p}: mapinhoud toevoegen mislukt (code {r})"));
                } else {
                    notify.log(Level::Info, format!("Toegevoegd: {p} (map, recursief)"));
                }
            } else {
                // NB: retourwaarde = aantal nodes in parent bij succes, < 0 = fout.
                let mut node: *mut crate::isofs::IsoNode = std::ptr::null_mut();
                let r = (iso.tree_add_new_node)(
                    image,
                    root,
                    name_c.as_ptr(),
                    path_c.as_ptr(),
                    &mut node,
                );
                if r < 0 {
                    add_errors.push(format!("{p}: toevoegen mislukt (code {r})"));
                } else {
                    notify.log(Level::Info, format!("Toegevoegd: {p} (bestand)"));
                }
            }
        }
    }
    if !add_errors.is_empty() {
        for e in &add_errors {
            notify.log(Level::Error, format!("Item overgeslagen: {e}"));
        }
        if add_errors.len() == paths.len() {
            let msg = "Geen enkel item kon aan de image worden toegevoegd".to_string();
            notify.log(Level::Error, &msg);
            unsafe {
                (iso.image_unref)(image);
                (st.raw.drive_release)(di.drive, 0);
            }
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    }

    // libisofs write-opts: profiel 2 (DISTRIBUTION) = Rock Ridge + Joliet.
    let iso_opts = unsafe {
        let mut opts: *mut crate::isofs::IsoWriteOpts = std::ptr::null_mut();
        if (iso.write_opts_new)(&mut opts, 2) != ISO_SUCCESS || opts.is_null() {
            let msg = "Kan de libisofs write-opts niet aanmaken".to_string();
            notify.log(Level::Error, &msg);
            (iso.image_unref)(image);
            (st.raw.drive_release)(di.drive, 0);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
        (iso.write_opts_set_iso_level)(opts, 3);
        (iso.write_opts_set_rockridge)(opts, 1);
        (iso.write_opts_set_joliet)(opts, 1);
        // Tijdstempels: 0 = originele IsoNode-tijdstempels (bestands-mtime),
        // 1 = opnametijd. Plus mtime ook in de ECMA-119/Joliet-directoryrecords
        // (bitveld 7 = alle drie de bomen).
        if s.keep_timestamps {
            (iso.write_opts_set_replace_timestamps)(opts, 0);
            (iso.write_opts_set_dir_rec_mtime)(opts, 7);
        } else {
            (iso.write_opts_set_replace_timestamps)(opts, 1);
        }
        opts
    };

    // Image-layout berekenen en burn_source opvragen (kan even duren).
    notify.log(Level::Info, "Image-layout berekenen (kan even duren)…");
    let src_raw = unsafe {
        let mut src: *mut ffi::BurnSource = std::ptr::null_mut();
        let r = (iso.image_create_burn_source)(image, iso_opts, &mut src);
        drain_iso_msgs(iso, notify);
        if r != ISO_SUCCESS || src.is_null() {
            let msg = "Samenstellen van de image mislukt — zie de libisofs-meldingen".to_string();
            notify.log(Level::Error, &msg);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
            (st.raw.drive_release)(di.drive, 0);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
        src
    };

    // Wrap de libisofs-bron in een libburn-FIFO (4 MiB): gladstrijkt de
    // datastroom richting drive én maakt de fifo-vulling in de voortgang
    // betekenisvol (zonder fifo blijft die permanent 0).
    let fifo = unsafe { (st.raw.fifo_source_new)(src_raw, 2048, BURN_FIFO_CHUNKS, 0) };
    unsafe { (st.raw.source_free)(src_raw) }; // fifo heeft een eigen referentie
    if fifo.is_null() {
        let msg = "Kan de FIFO om de image-bron niet aanmaken".to_string();
        notify.log(Level::Error, &msg);
        unsafe {
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
            (st.raw.drive_release)(di.drive, 0);
        }
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    let src = fifo;

    // Grootte via de get_size-callback van de burn_source.
    let size = unsafe { burn_source_get_size(src) };
    if size <= 0 {
        let msg = "Imagegrootte onbekend — samenstellen afgebroken".to_string();
        notify.log(Level::Error, &msg);
        unsafe {
            (st.raw.source_free)(src);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
            (st.raw.drive_release)(di.drive, 0);
        }
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    let expected_sectors = ((size + 2047) / 2048) as i32;
    notify.log(
        Level::Info,
        format!(
            "Image klaar: {} blokken (≈ {}) uit {} item(s)",
            expected_sectors,
            format_blocks(expected_sectors),
            paths.len()
        ),
    );

    // Ruimtecheck (indicatief).
    let avail = unsafe { (st.raw.disc_available_space)(di.drive, std::ptr::null_mut()) };
    if avail >= 0 && (avail as i64) < size {
        notify.log(
            Level::Warning,
            format!(
                "Image (≈ {}) lijkt groter dan de beschikbare ruimte (≈ {})",
                format_blocks(expected_sectors),
                format_blocks(avail as i32)
            ),
        );
    }

    // libburn-model: disc → session → track met de libisofs-bron.
    let (disc, session, track) = unsafe {
        let disc = (st.raw.disc_create)();
        let session = (st.raw.session_create)();
        let track = (st.raw.track_create)();
        if disc.is_null()
            || session.is_null()
            || track.is_null()
            || (st.raw.disc_add_session)(disc, session, ffi::BURN_POS_END) != 1
            || (st.raw.session_add_track)(session, track, ffi::BURN_POS_END) != 1
        {
            let msg = "Kan het libburn-model niet opbouwen".to_string();
            notify.log(Level::Error, &msg);
            (st.raw.source_free)(src);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
            (st.raw.drive_release)(di.drive, 0);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
        (disc, session, track)
    };
    let model_ok = unsafe {
        (st.raw.track_set_source)(track, src) == 0 && (st.raw.track_set_size)(track, size) == 1
    };
    if !model_ok {
        let msg = "Kan de libisofs-bron niet aan de track koppelen".to_string();
        notify.log(Level::Error, &msg);
        unsafe {
            (st.raw.track_free)(track);
            (st.raw.session_free)(session);
            (st.raw.disc_free)(disc);
            (st.raw.source_free)(src);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
            (st.raw.drive_release)(di.drive, 0);
        }
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    unsafe { (st.raw.track_define_data)(track, 0, s.padding_kib * 1024, 1, ffi::BURN_MODE1) };

    let opts = match unsafe { make_write_opts(&st.raw, di.drive, disc, s, notify) } {
        Ok(o) => o,
        Err(msg) => {
            unsafe {
                (st.raw.source_free)(src);
                (st.raw.track_free)(track);
                (st.raw.session_free)(session);
                (st.raw.disc_free)(disc);
                (iso.write_opts_free)(iso_opts);
                (iso.image_unref)(image);
                (st.raw.drive_release)(di.drive, 0);
            }
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };

    // Branden (gedeelde poll-lus); daarna libisofs-objecten vrijgeven.
    unsafe {
        run_write_poll(
            st,
            di.drive,
            opts,
            disc,
            session,
            track,
            src,
            index,
            expected_sectors,
            s,
            cmds,
            pending,
            notify,
        );
        (iso.write_opts_free)(iso_opts);
        (iso.image_unref)(image);
    }
}

/// Geeft de imagegrootte terug via de get_size-callback van een burn_source
/// (libisofs vult deze met de uiteindelijke imagegrootte in bytes).
/// Layout van `struct burn_source` volgens libburn.h (1.5.6).
#[repr(C)]
struct BurnSourceLayout {
    refcount: c_int,
    read: Option<unsafe extern "C" fn(*mut ffi::BurnSource, *mut u8, c_int) -> c_int>,
    read_sub: Option<unsafe extern "C" fn(*mut ffi::BurnSource, *mut u8, c_int) -> c_int>,
    get_size: Option<unsafe extern "C" fn(*mut ffi::BurnSource) -> c_longlong>,
    set_size: Option<unsafe extern "C" fn(*mut ffi::BurnSource, c_longlong) -> c_int>,
    free_data: Option<unsafe extern "C" fn(*mut ffi::BurnSource)>,
    next: *mut ffi::BurnSource,
    data: *mut c_void,
    version: c_int,
    read_xt: Option<unsafe extern "C" fn(*mut ffi::BurnSource, *mut u8, c_int) -> c_int>,
    cancel: Option<unsafe extern "C" fn(*mut ffi::BurnSource) -> c_int>,
}

unsafe fn burn_source_get_size(src: *mut ffi::BurnSource) -> i64 {
    unsafe {
        let layout = src as *mut BurnSourceLayout;
        match (*layout).get_size {
            Some(f) => f(src),
            None => 0,
        }
    }
}

/// Grab + media-check + evt. wissen vooraf — gedeeld door ISO-branden en
/// bestands-branden. Bij fout: log + BurnFailed + drive vrijgeven, Err(()).
fn grab_and_check_media(
    st: &WorkerState,
    di: &DriveInfo,
    index: usize,
    s: &BurnSettings,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) -> Result<DiscStatus, ()> {
    use crate::settings::BlankMode;

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(di, &st.raw) };
        let msg = format!(
            "Grab mislukt voor {} — drive mogelijk bezet (bijv. automount door de \
             bestandsbeheerder). Zet “Exclusief openen” uit (Instellingen → Apparaat) \
             of unmount de schijf.",
            if adr.is_empty() {
                format!("station {index}")
            } else {
                adr.clone()
            }
        );
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return Err(());
    }

    // Media-status bepalen (poll tot bekend).
    let mut status = DiscStatus::Unready;
    for _ in 0..40 {
        status = DiscStatus::from_raw(unsafe { (st.raw.disc_get_status)(di.drive) });
        if status != DiscStatus::Unready {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    // Optioneel eerst wissen (herbeschrijfbare media). Twee slimme overslagen:
    // direct-overschrijfbare profielen (DVD+RW, DVD-RAM, BD-RE, geformatteerde
    // DVD-RW) hoeven niet gewist te worden — de drive weigert dat vaak zelfs —
    // en al-lege media hoeft niet gewist te worden. De erasable-bit melden
    // veel drives alleen voor CD; daarom ook het SCSI-profiel checken.
    if s.blank_first {
        let mut profile_no: c_int = 0;
        let mut pname = [0 as c_char; 80];
        let _ = unsafe { (st.raw.disc_get_profile)(di.drive, &mut profile_no, pname.as_mut_ptr()) };
        if profile_is_overwritable(profile_no) {
            notify.log(
                Level::Info,
                format!(
                    "Media (profiel 0x{:02X}) is direct overschrijfbaar — wissen \
                     is niet nodig en wordt overgeslagen",
                    profile_no
                ),
            );
        } else if status == DiscStatus::Blank {
            notify.log(Level::Info, "Media is al leeg — wissen overgeslagen");
        } else {
            let erasable = unsafe { (st.raw.disc_erasable)(di.drive) } != 0
                || profile_is_rewritable(profile_no);
            if !erasable {
                let msg = "“Media eerst wissen” staat aan, maar de media is niet \
                           herbeschrijfbaar"
                    .to_string();
                notify.log(Level::Error, &msg);
                unsafe { (st.raw.drive_release)(di.drive, 0) };
                notify.send(Event::BurnFailed { index, error: msg });
                return Err(());
            }
            let fast = s.blank_mode == BlankMode::Fast;
            notify.log(
                Level::Info,
                format!(
                    "Station {index}: media eerst wissen ({})…",
                    if fast { "snel" } else { "volledig" }
                ),
            );
            unsafe { (st.raw.disc_erase)(di.drive, fast as c_int) };
            let well = match wait_media_job(
                st,
                di.drive,
                "Wissen",
                DriveStatus::Erasing,
                None,
                cmds,
                pending,
                notify,
            ) {
                Ok(w) => w,
                Err(()) => {
                    unsafe { (st.raw.drive_release)(di.drive, 0) };
                    notify.log(Level::Warning, "Brandjob geannuleerd tijdens wissen");
                    notify.send(Event::BurnCancelled);
                    return Err(());
                }
            };
            unsafe { (st.raw.drive_re_assess)(di.drive, 0) };
            if !well {
                let msg = "Wissen mislukt — brandjob afgebroken".to_string();
                notify.log(Level::Error, &msg);
                unsafe { (st.raw.drive_release)(di.drive, 0) };
                notify.send(Event::BurnFailed { index, error: msg });
                return Err(());
            }
            notify.log(Level::Success, "Media gewist");
            let status = DiscStatus::from_raw(unsafe { (st.raw.disc_get_status)(di.drive) });
            return Ok(status);
        }
    }

    if status != DiscStatus::Blank && status != DiscStatus::Appendable {
        let msg = format!(
            "Media is niet beschrijfbaar (status: {}) — lege of onvolledige media nodig",
            disc_label(status)
        );
        notify.log(Level::Error, &msg);
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        notify.send(Event::BurnFailed { index, error: msg });
        return Err(());
    }
    if status == DiscStatus::Appendable {
        notify.log(
            Level::Warning,
            "Media is onvolledig (appendable) — de nieuwe sessie wordt eraan toegevoegd",
        );
    }
    Ok(status)
}

/// Maakt de write-opts aan en stelt alles in volgens de GUI-instellingen
/// (snelheid, simulatie, multi-session, underrun-proof, overburn, schrijfmodus
/// met precheck). Bij fout: log + Err(reden).
unsafe fn make_write_opts(
    raw: &RawLibburn,
    drive: *mut ffi::BurnDrive,
    disc: *mut ffi::BurnDisc,
    s: &BurnSettings,
    notify: &Notifier,
) -> Result<*mut ffi::BurnWriteOpts, String> {
    use crate::settings::{MultiSession, WriteMode};

    unsafe {
        let opts = (raw.write_opts_new)(drive);
        if opts.is_null() {
            let msg = "Kan de write-opts niet aanmaken".to_string();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        (raw.write_opts_set_perform_opc)(opts, 0);
        (raw.write_opts_set_simulate)(opts, s.simulate as c_int);
        let mut multi = match s.multi_session {
            MultiSession::KeepOpen => 1,
            _ => 0,
        };
        // Overwritbare media (DVD+RW, DVD-RAM, BD-RE, geformatteerde DVD-RW)
        // ondersteunt geen appendable-sessies — die media is inherent altijd
        // beschrijfbaar. libburn weigert anders het hele job
        // ("multi session capability lacking"); de vlag is daar zinloos.
        if multi == 1 {
            let mut profile_no: c_int = 0;
            let mut pname = [0 as c_char; 80];
            let _ = (raw.disc_get_profile)(drive, &mut profile_no, pname.as_mut_ptr());
            if profile_is_overwritable(profile_no) {
                notify.log(
                    Level::Info,
                    format!(
                        "Media (profiel 0x{:02X}) is direct overschrijfbaar — \
                         multi-session is niet van toepassing; de media blijft \
                         altijd beschrijfbaar",
                        profile_no
                    ),
                );
                multi = 0;
            }
        }
        (raw.write_opts_set_multi)(opts, multi);
        (raw.write_opts_set_underrun_proof)(opts, s.underrun_proof as c_int);
        if s.overburn {
            (raw.write_opts_set_force)(opts, 1);
            notify.log(
                Level::Info,
                "Overburn/force aan: enkele conformiteitschecks worden genegeerd",
            );
        }

        match s.write_mode {
            WriteMode::Auto => {
                let mut reasons = [0 as c_char; ffi::BURN_REASONS_LEN];
                let wt = (raw.write_opts_auto_write_type)(opts, disc, reasons.as_mut_ptr(), 0);
                if wt == ffi::BURN_WRITE_NONE {
                    let reasons = cbuf_to_string(&reasons);
                    let mut msg = format!(
                        "Geen geschikte schrijfmodus gevonden: {}",
                        if reasons.is_empty() {
                            "onbekende reden"
                        } else {
                            &reasons
                        }
                    );
                    notify.log(Level::Error, &msg);
                    if let Some(hint) = simulation_hint(s, &reasons) {
                        notify.log(Level::Warning, hint.clone());
                        msg.push_str(" — ");
                        msg.push_str(&hint);
                    }
                    (raw.write_opts_free)(opts);
                    return Err(msg);
                }
                notify.log(
                    Level::Info,
                    format!("Schrijfmodus (auto): {}", write_type_name(wt)),
                );
            }
            WriteMode::Tao => {
                if (raw.write_opts_set_write_type)(opts, ffi::BURN_WRITE_TAO, ffi::BURN_BLOCK_MODE1)
                    != 1
                {
                    let msg =
                        "TAO + MODE1 wordt niet door deze drive/media ondersteund".to_string();
                    notify.log(Level::Error, &msg);
                    (raw.write_opts_free)(opts);
                    return Err(msg);
                }
                notify.log(Level::Info, "Schrijfmodus: TAO (track-at-once)");
                if let Some(err) = precheck_write_opts(raw, opts, disc, s) {
                    notify.log(Level::Error, &err);
                    if let Some(hint) = simulation_hint(s, &err) {
                        notify.log(Level::Warning, hint);
                    }
                    (raw.write_opts_free)(opts);
                    return Err(err);
                }
            }
            WriteMode::Sao => {
                if (raw.write_opts_set_write_type)(opts, ffi::BURN_WRITE_SAO, ffi::BURN_BLOCK_SAO)
                    != 1
                {
                    let msg = "SAO + SAO-bloktype wordt niet door deze drive/media ondersteund"
                        .to_string();
                    notify.log(Level::Error, &msg);
                    (raw.write_opts_free)(opts);
                    return Err(msg);
                }
                notify.log(Level::Info, "Schrijfmodus: SAO (session-at-once)");
                if let Some(err) = precheck_write_opts(raw, opts, disc, s) {
                    notify.log(Level::Error, &err);
                    if let Some(hint) = simulation_hint(s, &err) {
                        notify.log(Level::Warning, hint);
                    }
                    (raw.write_opts_free)(opts);
                    return Err(err);
                }
            }
            WriteMode::Raw => unreachable!("RAW wordt al eerder afgewezen"),
        }

        Ok(opts)
    }
}

/// Start het branden en pollt tot de drive weer IDLE is; stuurt voortgang,
/// handelt annuleren af, ruimt het model op en geeft het resultaat als events.
unsafe fn run_write_poll(
    st: &WorkerState,
    drive: *mut ffi::BurnDrive,
    opts: *mut ffi::BurnWriteOpts,
    disc: *mut ffi::BurnDisc,
    session: *mut ffi::BurnSession,
    track: *mut ffi::BurnTrack,
    source: *mut ffi::BurnSource,
    index: usize,
    total_sectors: i32,
    s: &BurnSettings,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    use crate::settings::MultiSession;

    notify.send(Event::BurnStarted {
        index,
        total_sectors,
        simulate: s.simulate,
    });
    notify.log(
        Level::Info,
        format!(
            "Station {index}: branden gestart{}…",
            if s.simulate {
                " (SIMULATIE — laser uit)"
            } else {
                ""
            }
        ),
    );
    unsafe { (st.raw.disc_write)(opts, disc) };

    let started = std::time::Instant::now();
    let mut last_progress = started;
    let mut cancelled = false;
    loop {
        // Annuleren/afsluiten tussentijds mogelijk maken.
        match cmds.try_recv() {
            Ok(Command::CancelBurn) => {
                unsafe { (st.raw.drive_cancel)(drive) };
                cancelled = true;
            }
            Ok(Command::Shutdown) => {
                pending.push_front(Command::Shutdown);
                unsafe { (st.raw.drive_cancel)(drive) };
                cancelled = true;
            }
            Ok(other) => pending.push_back(other),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                unsafe { (st.raw.drive_cancel)(drive) };
                cancelled = true;
            }
        }

        thread::sleep(Duration::from_millis(150));
        drain_msgs(&st.raw, notify);

        let mut prog = Progress::default();
        let ds = DriveStatus::from_raw(unsafe { (st.raw.drive_get_status)(drive, &mut prog) });
        if ds == DriveStatus::Idle {
            break;
        }

        // Voortgang sturen tijdens schrijf-fasen.
        if matches!(
            ds,
            DriveStatus::Writing
                | DriveStatus::WritingLeadin
                | DriveStatus::WritingLeadout
                | DriveStatus::WritingPregap
                | DriveStatus::ClosingTrack
                | DriveStatus::ClosingSession
        ) && prog.sectors > 0
            && last_progress.elapsed() >= Duration::from_millis(200)
        {
            let elapsed = started.elapsed().as_secs_f64();
            let secs = elapsed.max(0.001);
            let kbps = (prog.sector as f64 * 2048.0) / secs / 1000.0;
            // ETA: verstreken tijd × (resterend / gedaan), alleen bij echte
            // voortgang (sector > 0 en nog niet klaar).
            let eta = if prog.sector > 0 && prog.sector < prog.sectors {
                elapsed * ((prog.sectors - prog.sector) as f64 / prog.sector as f64)
            } else {
                0.0
            };
            let buffer_pct = if prog.buffer_capacity > 0 {
                ((prog.buffer_capacity - prog.buffer_available) as f32
                    / prog.buffer_capacity as f32)
                    * 100.0
            } else {
                0.0
            };
            let fifo_pct = unsafe { fifo_fill_pct(&st.raw, source) };
            notify.send(Event::BurnProgress {
                index,
                sector: prog.sector,
                sectors: prog.sectors,
                kbps,
                buffer_pct,
                fifo_pct,
                phase: drive_status_label(ds).to_string(),
                elapsed_secs: elapsed,
                eta_secs: eta,
            });
            last_progress = std::time::Instant::now();
        }
    }

    // Resultaat en opruimen.
    let well = unsafe { (st.raw.drive_wrote_well)(drive) } == 1;
    drain_msgs(&st.raw, notify);
    unsafe {
        (st.raw.write_opts_free)(opts);
        (st.raw.source_free)(source);
        (st.raw.track_free)(track);
        (st.raw.session_free)(session);
        (st.raw.disc_free)(disc);
        (st.raw.drive_release)(drive, s.eject_after as c_int);
    }

    if cancelled {
        notify.log(Level::Warning, "Brandjob geannuleerd");
        notify.send(Event::BurnCancelled);
    } else if well {
        notify.log(
            Level::Success,
            format!(
                "Station {index}: brandjob klaar{} — media {}",
                if s.simulate { " (simulatie)" } else { "" },
                if s.multi_session == MultiSession::KeepOpen {
                    "blijft appendable"
                } else {
                    "is afgesloten"
                }
            ),
        );
        notify.send(Event::BurnDone);
    } else {
        let msg = "Brandjob mislukt — zie de libburn-meldingen hierboven".to_string();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
    }
}

/// Bouwt het disc-model op: disc → session → track met file-bron + FIFO,
/// plus de write-opts volgens de instellingen. Geeft None terug bij falen
/// (met logfeedback over de oorzaak).
unsafe fn build_burn_model(
    raw: &RawLibburn,
    drive: *mut ffi::BurnDrive,
    path_c: *const c_char,
    file_size: u64,
    s: &BurnSettings,
    notify: &Notifier,
) -> Result<
    (
        *mut ffi::BurnDisc,
        *mut ffi::BurnSession,
        *mut ffi::BurnTrack,
        *mut ffi::BurnSource,
        *mut ffi::BurnWriteOpts,
    ),
    String,
> {
    // Edition 2024: onveilige operaties expliciet in een `unsafe`-blok.
    unsafe {
        let disc = (raw.disc_create)();
        let session = (raw.session_create)();
        let track = (raw.track_create)();
        if disc.is_null() || session.is_null() || track.is_null() {
            let msg = "Kan disc/session/track-model niet aanmaken".to_string();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        if (raw.disc_add_session)(disc, session, ffi::BURN_POS_END) != 1
            || (raw.session_add_track)(session, track, ffi::BURN_POS_END) != 1
        {
            let msg = "Kan session/track niet aan het model toevoegen".to_string();
            notify.log(Level::Error, &msg);
            (raw.track_free)(track);
            (raw.session_free)(session);
            (raw.disc_free)(disc);
            return Err(msg);
        }

        // Bron: bestand → FIFO (4 MiB) → track.
        let file_source = (raw.file_source_new)(path_c, std::ptr::null());
        if file_source.is_null() {
            let msg = "Kan de file-bron niet aanmaken (bestand onleesbaar?)".to_string();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        let fifo = (raw.fifo_source_new)(file_source, 2048, BURN_FIFO_CHUNKS, 0);
        (raw.source_free)(file_source); // fifo heeft een eigen referentie
        if fifo.is_null() {
            let msg = "Kan de FIFO-bron niet aanmaken".to_string();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        if (raw.track_set_source)(track, fifo) != 0 {
            let msg = "Kan de bron niet aan de track koppelen".to_string();
            notify.log(Level::Error, &msg);
            (raw.source_free)(fifo);
            return Err(msg);
        }
        if (raw.track_set_size)(track, file_size as c_longlong) != 1 {
            let msg = "Kan de trackgrootte niet instellen".to_string();
            notify.log(Level::Error, &msg);
            (raw.source_free)(fifo);
            return Err(msg);
        }
        // Datatrack: geen offset, padding als staart-nullen, laatste sector vullen.
        (raw.track_define_data)(track, 0, s.padding_kib * 1024, 1, ffi::BURN_MODE1);

        // Write-opts + schrijfmodus (gedeelde helper).
        let opts = make_write_opts(raw, drive, disc, s, notify)?;

        Ok((disc, session, track, fifo, opts))
    }
}

/// Vulgraad van de FIFO in procenten (0 als onbekend).
unsafe fn fifo_fill_pct(raw: &RawLibburn, fifo: *mut ffi::BurnSource) -> f32 {
    unsafe {
        let mut size: c_int = 0;
        let mut free_bytes: c_int = 0;
        let mut status_text: *const c_char = std::ptr::null();
        if (raw.fifo_inquire_status)(fifo, &mut size, &mut free_bytes, &mut status_text) >= 0
            && size > 0
        {
            ((size - free_bytes) as f32 / size as f32 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    }
}

/// Naam van een burn_write_types-waarde.
pub fn write_type_name(wt: c_int) -> &'static str {
    match wt {
        ffi::BURN_WRITE_TAO => "TAO (track-at-once)",
        ffi::BURN_WRITE_SAO => "SAO (session-at-once)",
        ffi::BURN_WRITE_RAW => "RAW",
        _ => "onbekend",
    }
}

/// Voert de libburn-precheck uit op de zojuist gekozen schrijfmodus.
/// Geeft bij afwijzing de redenen (plus eventuele simulatie-hint) terug.
unsafe fn precheck_write_opts(
    raw: &RawLibburn,
    opts: *mut ffi::BurnWriteOpts,
    disc: *mut ffi::BurnDisc,
    s: &BurnSettings,
) -> Option<String> {
    unsafe {
        let mut reasons = [0 as c_char; ffi::BURN_REASONS_LEN];
        let ret = (raw.precheck_write)(opts, disc, reasons.as_mut_ptr(), 1);
        if ret > 0 {
            return None;
        }
        let reasons = cbuf_to_string(&reasons);
        let mut msg = format!(
            "Deze schrijfmodus wordt afgewezen voor deze drive/media: {}",
            if reasons.is_empty() {
                "onbekende reden"
            } else {
                &reasons
            }
        );
        if let Some(hint) = simulation_hint(s, &reasons) {
            msg.push_str(" — ");
            msg.push_str(&hint);
        }
        Some(msg)
    }
}

/// Herkent “simulatie niet ondersteund” in de afwijzingsredenen van libburn
/// en geeft een concrete tip terug. Simulatie wordt doorgaans alleen door
/// CD-drives/-media ondersteund; alle BD-media en DVD-R DL kunnen nooit
/// simuleren, net als overbeschrijfbare media (DVD+RW, DVD-RAM, DVD-RW RO).
fn simulation_hint(s: &BurnSettings, reasons: &str) -> Option<String> {
    if !s.simulate {
        return None;
    }
    let lower = reasons.to_ascii_lowercase();
    if lower.contains("simulation") || lower.contains("simul") {
        Some(
            "Tip: “Simulatie (laser uit)” wordt door deze drive/media niet \
             ondersteund. Simulatie werkt praktisch alleen bij CD-media; alle \
             BD-media, DVD-R DL en overbeschrijfbare media (DVD+RW, DVD-RAM, \
             DVD-RW RO) kunnen nooit simuleren. Zet “Simulatie” uit in \
             Instellingen → Branden — let op: zonder simulatie wordt er \
             écht geschreven."
                .to_string(),
        )
    } else {
        None
    }
}

/// Profielen waarop simuleren niet mogelijk is (MMC: geen test write).
pub fn profile_cant_simulate(pno: i32) -> bool {
    (0x40..=0x43).contains(&pno) // BD-ROM / BD-R / BD-RE
        || matches!(pno, 0x12 | 0x13 | 0x1A | 0x15 | 0x16)
    // DVD-RAM, DVD-RW RO, DVD+RW, DVD-R DL (seq + layer jump)
}

/// Profielen die herbeschrijfbaar zijn. De `erasable`-bit uit SCSI READ DISC
/// INFORMATION melden veel drives alleen voor CD; voor DVD/BD is het profiel
/// de betrouwbare indicatie (bijv. een afgesloten DVD-RW meldt erasable=0).
pub fn profile_is_rewritable(pno: i32) -> bool {
    matches!(pno, 0x0A | 0x12 | 0x13 | 0x14 | 0x1A | 0x43)
    // CD-RW, DVD-RAM, DVD-RW (RO + seq), DVD+RW, BD-RE
}

/// Profielen die direct overschrijfbaar zijn — wissen is daar niet nodig en
/// wordt door de drive vaak geweigerd (DVD+RW, DVD-RAM, BD-RE en geformatteerde
/// DVD-RW schrijven gewoon over de oude data heen).
pub fn profile_is_overwritable(pno: i32) -> bool {
    matches!(pno, 0x12 | 0x13 | 0x1A | 0x43)
}

/// Draait een wis- of format-job en pollt tot de drive weer IDLE is.
/// Stuurt MaintProgress-events (als kind/index bekend zijn), handelt
/// annuleren/afsluiten af en geeft de resultaatcheck terug. Err betekent
/// geannuleerd/verbinding weg (de drive is dan al gecanceld).
fn wait_media_job(
    st: &WorkerState,
    drive: *mut ffi::BurnDrive,
    label: &str,
    busy_status: DriveStatus,
    maint: Option<(MaintKind, usize)>,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) -> Result<bool, ()> {
    let mut last_pct: i32 = -1;
    loop {
        match cmds.try_recv() {
            Ok(Command::CancelBurn) => {
                unsafe { (st.raw.drive_cancel)(drive) };
                return Err(());
            }
            Ok(Command::Shutdown) => {
                pending.push_front(Command::Shutdown);
                unsafe { (st.raw.drive_cancel)(drive) };
                return Err(());
            }
            Ok(other) => pending.push_back(other),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                unsafe { (st.raw.drive_cancel)(drive) };
                return Err(());
            }
        }
        thread::sleep(Duration::from_millis(150));
        drain_msgs(&st.raw, notify);
        let mut prog = Progress::default();
        let ds = DriveStatus::from_raw(unsafe { (st.raw.drive_get_status)(drive, &mut prog) });
        if ds == DriveStatus::Idle {
            break;
        }
        if ds == busy_status && prog.sectors > 0 {
            let pct = ((prog.sector as f32 / prog.sectors as f32) * 100.0).min(100.0) as i32;
            if pct != last_pct {
                notify.log(Level::Info, format!("{label}… {pct}%"));
                if let Some((kind, index)) = maint {
                    notify.send(Event::MaintProgress {
                        kind,
                        index,
                        pct: pct as f32,
                    });
                }
                last_pct = pct;
            }
        }
    }
    Ok(unsafe { (st.raw.drive_wrote_well)(drive) } == 1)
}

/// Wis de media los van een brandjob (stap 6).
fn erase_job(
    state: &Option<WorkerState>,
    index: usize,
    fast: bool,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    let Some(st) = state else {
        notify.log(
            Level::Warning,
            "Wissen aangevraagd, maar libburn is niet geladen",
        );
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(
            Level::Warning,
            format!("Wissen aangevraagd voor onbekend station {index}"),
        );
        return;
    }
    let di = unsafe { *st.infos.add(index) };
    notify.send(Event::MaintStarted {
        kind: MaintKind::Erase,
        index,
    });
    notify.log(
        Level::Info,
        format!(
            "Station {index}: media wissen ({})…",
            if fast { "snel" } else { "volledig" }
        ),
    );

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = format!(
            "Grab mislukt voor {} — drive mogelijk bezet (bijv. automount). \
             Zet “Exclusief openen” uit of unmount de schijf.",
            if adr.is_empty() {
                format!("station {index}")
            } else {
                adr.clone()
            }
        );
        notify.log(Level::Error, &msg);
        notify.send(Event::MaintFailed {
            kind: MaintKind::Erase,
            index,
            error: msg,
        });
        return;
    }

    let mut profile_no: c_int = 0;
    let mut pname = [0 as c_char; 80];
    let _ = unsafe { (st.raw.disc_get_profile)(di.drive, &mut profile_no, pname.as_mut_ptr()) };
    if profile_is_overwritable(profile_no) {
        let msg = format!(
            "Media (profiel 0x{:02X}) is direct overschrijfbaar — wissen is niet \
             nodig; gebruik “Formatteren” om de schijf te herstellen",
            profile_no
        );
        notify.log(Level::Error, &msg);
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        notify.send(Event::MaintFailed {
            kind: MaintKind::Erase,
            index,
            error: msg,
        });
        return;
    }
    let erasable =
        unsafe { (st.raw.disc_erasable)(di.drive) } != 0 || profile_is_rewritable(profile_no);
    if !erasable {
        let msg = "Deze media is niet herbeschrijfbaar — wissen is niet mogelijk".to_string();
        notify.log(Level::Error, &msg);
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        notify.send(Event::MaintFailed {
            kind: MaintKind::Erase,
            index,
            error: msg,
        });
        return;
    }

    unsafe { (st.raw.disc_erase)(di.drive, fast as c_int) };
    let well = match wait_media_job(
        st,
        di.drive,
        "Wissen",
        DriveStatus::Erasing,
        Some((MaintKind::Erase, index)),
        cmds,
        pending,
        notify,
    ) {
        Ok(w) => w,
        Err(()) => {
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            notify.log(Level::Warning, "Wissen geannuleerd");
            notify.send(Event::MaintCancelled {
                kind: MaintKind::Erase,
                index,
            });
            return;
        }
    };
    unsafe { (st.raw.drive_re_assess)(di.drive, 0) };
    drain_msgs(&st.raw, notify);
    unsafe { (st.raw.drive_release)(di.drive, 0) };

    if well {
        notify.log(Level::Success, "Media gewist");
        notify.send(Event::MaintDone {
            kind: MaintKind::Erase,
            index,
        });
    } else {
        let msg = "Wissen mislukt — zie de libburn-meldingen hierboven".to_string();
        notify.log(Level::Error, &msg);
        notify.send(Event::MaintFailed {
            kind: MaintKind::Erase,
            index,
            error: msg,
        });
    }
}

/// Profielen waarop formatteren zinvol heeft (DVD-RW seq → RO, DVD-RW RO,
/// DVD+RW, DVD-RAM, BD-RE).
pub fn profile_is_formattable(pno: i32) -> bool {
    matches!(pno, 0x12 | 0x13 | 0x14 | 0x1A | 0x43)
}

/// Formatteer de media los van een brandjob (stap 6).
fn format_job(
    state: &Option<WorkerState>,
    index: usize,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    let Some(st) = state else {
        notify.log(
            Level::Warning,
            "Formatteren aangevraagd, maar libburn is niet geladen",
        );
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(
            Level::Warning,
            format!("Formatteren aangevraagd voor onbekend station {index}"),
        );
        return;
    }
    let di = unsafe { *st.infos.add(index) };
    notify.send(Event::MaintStarted {
        kind: MaintKind::Format,
        index,
    });
    notify.log(Level::Info, "Media formatteren (standaardgrootte)…");

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = format!(
            "Grab mislukt voor {} — drive mogelijk bezet (bijv. automount). \
             Zet “Exclusief openen” uit (Instellingen → Apparaat) \
             of unmount de schijf.",
            if adr.is_empty() {
                format!("station {index}")
            } else {
                adr.clone()
            }
        );
        notify.log(Level::Error, &msg);
        notify.send(Event::MaintFailed {
            kind: MaintKind::Format,
            index,
            error: msg,
        });
        return;
    }

    let mut profile_no: c_int = 0;
    let mut pname = [0 as c_char; 80];
    let _ = unsafe { (st.raw.disc_get_profile)(di.drive, &mut profile_no, pname.as_mut_ptr()) };
    if !profile_is_formattable(profile_no) {
        let msg = format!(
            "Formatteren is niet van toepassing op deze media (profiel 0x{:02X}) \
             — voor CD-RW/DVD-RW sequentieel is “Wissen” de juiste actie",
            profile_no
        );
        notify.log(Level::Error, &msg);
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        notify.send(Event::MaintFailed {
            kind: MaintKind::Format,
            index,
            error: msg,
        });
        return;
    }

    // Volledige format met enforce-re-format (bit4): bij DVD+RW/BD-RE/DVD-RAM
    // ziet libburn een al geformatteerde schijf anders als no-op
    // ("FORMAT UNIT ignored. Already completed."). Bit4 forceert de "de-ice"
    // — de bestaande data wordt gewist. size-mode 3 (bit1+2) = standaardgrootte.
    notify.log(
        Level::Info,
        "Volledige format — bestaande data wordt gewist; dit kan enkele \
         minuten duren…",
    );
    unsafe { (st.raw.disc_format)(di.drive, 0, (3 << 1) | (1 << 4)) };
    let well = match wait_media_job(
        st,
        di.drive,
        "Formatteren",
        DriveStatus::Formatting,
        Some((MaintKind::Format, index)),
        cmds,
        pending,
        notify,
    ) {
        Ok(w) => w,
        Err(()) => {
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            notify.log(Level::Warning, "Formatteren geannuleerd");
            notify.send(Event::MaintCancelled {
                kind: MaintKind::Format,
                index,
            });
            return;
        }
    };
    unsafe { (st.raw.drive_re_assess)(di.drive, 0) };
    drain_msgs(&st.raw, notify);
    unsafe { (st.raw.drive_release)(di.drive, 0) };

    if well {
        notify.log(Level::Success, "Media geformatteerd");
        notify.send(Event::MaintDone {
            kind: MaintKind::Format,
            index,
        });
    } else {
        let msg = "Formatteren mislukt — zie de libburn-meldingen hierboven".to_string();
        notify.log(Level::Error, &msg);
        notify.send(Event::MaintFailed {
            kind: MaintKind::Format,
            index,
            error: msg,
        });
    }
}

/// Maakt een schijfkopie van de datamedia naar een bestand.
///
/// Gebruikt `burn_read_data` (random access, 2048-byte blokken): geschikt voor
/// CD/DVD/BD-datamedia, niet voor CD-audio. De drive wordt voor de duur van de
/// kopie gegrabbed en daarna weer vrijgegeven. Annuleren kan tussentijds.
fn read_disc(
    state: &Option<WorkerState>,
    index: usize,
    path: &str,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    use std::io::Write;

    let Some(st) = state else {
        notify.log(
            Level::Warning,
            "Kopie aangevraagd, maar libburn is niet geladen",
        );
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(
            Level::Warning,
            format!("Kopie aangevraagd voor onbekend station {index}"),
        );
        return;
    }
    let path = path.trim();
    if path.is_empty() {
        notify.log(Level::Warning, "Kopie aangevraagd zonder uitvoerbestand");
        return;
    }

    let di = unsafe { *st.infos.add(index) };
    notify.log(
        Level::Info,
        format!("Station {index}: schijfkopie starten naar `{path}`…"),
    );

    // Niet overschrijven: kies een andere naam.
    if std::path::Path::new(path).exists() {
        let msg = format!("Uitvoerbestand bestaat al: `{path}` — kies een andere naam");
        notify.log(Level::Error, format!("Station {index}: {msg}"));
        notify.send(Event::ReadFailed { index, error: msg });
        return;
    }

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = format!(
            "Grab mislukt voor {} — drive mogelijk bezet (bijv. automount door de \
             bestandsbeheerder). Zet “Exclusief openen” uit (Instellingen → Apparaat) \
             of unmount de schijf.",
            if adr.is_empty() {
                format!("station {index}")
            } else {
                adr.clone()
            }
        );
        notify.log(Level::Error, format!("Station {index}: {msg}"));
        notify.send(Event::ReadFailed { index, error: msg });
        return;
    }

    // Capaciteit bepalen (blokken van 2048 bytes).
    let mut capacity: c_int = 0;
    if unsafe { (st.raw.get_read_capacity)(di.drive, &mut capacity, 0) } != 1 || capacity <= 0 {
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        let msg = "Geen leesbare capaciteit — media leeg of geen datamedia \
                   (CD-audio wordt nog niet ondersteund)"
            .to_string();
        notify.log(Level::Error, format!("Station {index}: {msg}"));
        notify.send(Event::ReadFailed { index, error: msg });
        return;
    }
    let total_blocks = capacity as i64;
    notify.log(
        Level::Info,
        format!(
            "Station {index}: {} blokken (≈ {}) lezen…",
            total_blocks,
            format_blocks(capacity)
        ),
    );
    notify.send(Event::ReadStarted {
        index,
        total_blocks: capacity,
    });

    let mut file = match std::fs::File::create_new(path) {
        Ok(f) => f,
        Err(e) => {
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            let msg = format!("Kan `{path}` niet aanmaken: {e}");
            notify.log(Level::Error, format!("Station {index}: {msg}"));
            notify.send(Event::ReadFailed { index, error: msg });
            return;
        }
    };

    let mut buf = vec![0u8; READ_CHUNK_BLOCKS as usize * 2048];
    let mut blocks_done: i64 = 0;
    let started = std::time::Instant::now();
    let mut last_progress = started;
    let mut cancelled = false;
    let mut error: Option<String> = None;

    while blocks_done < total_blocks {
        // Annuleren/afsluiten tussentijds mogelijk maken.
        match cmds.try_recv() {
            Ok(Command::CancelRead) => {
                cancelled = true;
                break;
            }
            Ok(Command::Shutdown) => {
                pending.push_front(Command::Shutdown);
                cancelled = true;
                break;
            }
            Ok(other) => pending.push_back(other),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                cancelled = true;
                break;
            }
        }

        let want_blocks = (total_blocks - blocks_done).min(READ_CHUNK_BLOCKS);
        let want_bytes = want_blocks * 2048;
        let byte_address = blocks_done * 2048;
        let mut got: c_longlong = 0;
        let ret = unsafe {
            (st.raw.read_data)(
                di.drive,
                byte_address,
                buf.as_mut_ptr() as *mut c_char,
                want_bytes,
                &mut got,
                0,
            )
        };

        if got > 0 {
            let got = got as usize;
            if let Err(e) = file.write_all(&buf[..got]) {
                error = Some(format!("Schrijffout naar `{path}`: {e}"));
                break;
            }
            blocks_done += (got / 2048) as i64;
        }

        if ret <= 0 {
            let fail_lba = (byte_address + got) / 2048;
            error = Some(format!(
                "Leesfout bij LBA {fail_lba} (blok {} van {}) — kopie afgebroken; \
                 het deelbestand blijft staan",
                blocks_done, total_blocks
            ));
            break;
        }

        // Voortgang max. ~5× per seconde sturen.
        if last_progress.elapsed() >= Duration::from_millis(200) || blocks_done >= total_blocks {
            let secs = started.elapsed().as_secs_f64().max(0.001);
            let kbps = (blocks_done * 2048) as f64 / secs / 1000.0;
            notify.send(Event::ReadProgress {
                index,
                blocks_done,
                total_blocks: capacity,
                kbps,
            });
            last_progress = std::time::Instant::now();
        }
    }

    drop(file);
    unsafe { (st.raw.drive_release)(di.drive, 0) };

    if cancelled {
        notify.log(
            Level::Warning,
            format!(
                "Station {index}: kopie geannuleerd na {} blokken; `{path}` blijft (onvolledig) staan",
                blocks_done
            ),
        );
        notify.send(Event::ReadCancelled);
    } else if let Some(err) = error {
        notify.log(Level::Error, format!("Station {index}: {err}"));
        notify.send(Event::ReadFailed { index, error: err });
    } else {
        notify.log(
            Level::Success,
            format!(
                "Station {index}: schijfkopie klaar — {} ({}) → `{path}`",
                blocks_done,
                format_blocks(blocks_done as i32)
            ),
        );
        notify.send(Event::ReadDone);
    }
}

fn shutdown_lib(state: Option<WorkerState>, notify: &Notifier) {
    let Some(mut st) = state else { return };
    free_drive_list(&mut st);
    unsafe { (st.raw.finish)() };
    notify.log(Level::Info, "libburn afgesloten (burn_finish)");
    if let Some(iso) = st.isofs {
        unsafe { (iso.finish)() };
        notify.log(Level::Info, "libisofs afgesloten (iso_finish)");
    }
}

/// Laadt libisofs van het systeem (non-fataal: zonder libisofs werkt alles
/// behalve “bestanden samenstellen”).
fn load_isofs(notify: &Notifier) -> Option<RawLibisofs> {
    let mut candidates = Vec::new();
    if let Ok(env) = std::env::var("LIBISOFS_SO") {
        if !env.is_empty() {
            candidates.push(env);
        }
    }
    candidates.push("libisofs.so.6".to_string());
    candidates.push("libisofs.so".to_string());

    let mut last_err = String::new();
    for cand in &candidates {
        match RawLibisofs::load(cand) {
            Ok(iso) => unsafe {
                if (iso.init)() != ISO_SUCCESS {
                    last_err = format!("`{cand}`: iso_init() mislukte");
                    continue;
                }
                (iso.set_msgs_severities)(
                    c"UPDATE".as_ptr(),
                    c"NEVER".as_ptr(),
                    c"libisofs_gui: ".as_ptr(),
                );
                let (mut maj, mut min, mut mic) = (0, 0, 0);
                (iso.version)(&mut maj, &mut min, &mut mic);
                notify.log(
                    Level::Success,
                    format!("libisofs {maj}.{min}.{mic} geladen via `{cand}`"),
                );
                return Some(iso);
            },
            Err(e) => last_err = format!("`{cand}`: {e}"),
        }
    }
    notify.log(
        Level::Warning,
        format!(
            "libisofs niet geladen — “bestanden samenstellen” is niet beschikbaar ({last_err})"
        ),
    );
    None
}

/// Haalt alle opgequeuede libisofs-meldingen op en logt ze.
fn drain_iso_msgs(iso: &RawLibisofs, notify: &Notifier) {
    unsafe {
        let mut min = b"NEVER\0".to_vec();
        let mut msg = [0 as c_char; ISO_MSGS_MESSAGE_LEN];
        let mut sev = [0 as c_char; 80];
        let mut code: c_int = 0;
        let mut imgid: c_int = 0;
        while (iso.obtain_msgs)(
            min.as_mut_ptr() as *mut c_char,
            &mut code,
            &mut imgid,
            msg.as_mut_ptr(),
            sev.as_mut_ptr(),
        ) == 1
        {
            let text = crate::isofs::cbuf_to_string(&msg);
            let sev_name = crate::isofs::cbuf_to_string(&sev);
            let level = sev_to_level(&sev_name);
            notify.log(level, format!("[libisofs/{sev_name}] {text}"));
        }
    }
}

/// Terugvalnamen voor bekende SCSI-profielen als libburn een lege naam geeft.
pub fn profile_fallback_name(pno: i32) -> &'static str {
    match pno {
        0x01 => "Non standard",
        0x08 => "CD-ROM",
        0x09 => "CD-R",
        0x0A => "CD-RW",
        0x10 => "DVD-ROM",
        0x11 => "DVD-R sequentieel",
        0x12 => "DVD-RAM",
        0x13 => "DVD-RW restricted overwrite",
        0x14 => "DVD-RW sequentieel",
        0x15 => "DVD-R DL sequentieel",
        0x16 => "DVD-R DL layer jump",
        0x1A => "DVD+RW",
        0x1B => "DVD+R",
        0x2B => "DVD+R DL",
        0x40 => "BD-ROM",
        0x41 => "BD-R random recording",
        0x42 => "BD-R sequentieel",
        0x43 => "BD-RE",
        0xFFFF => "stdio-bestand",
        _ => "",
    }
}

/// Formatteert een aantal 2048-byte blokken leesbaar (MiB/GiB).
pub fn format_blocks(blocks: i32) -> String {
    let bytes = blocks as f64 * 2048.0;
    let mib = bytes / (1024.0 * 1024.0);
    if mib >= 1024.0 {
        format!("{:.2} GiB", mib / 1024.0)
    } else {
        format!("{:.1} MiB", mib)
    }
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

        cmd_tx
            .send(Command::LoadLibrary {
                path: None,
                exclusive: true,
            })
            .unwrap();
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
