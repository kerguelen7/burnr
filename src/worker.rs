//! Achtergrondworker: voert ALLE libburn-aanroepen uit op één thread.
//!
//! libburn is niet ontworpen voor gelijktijdige aanroepen; daarom praat de GUI
//! nooit rechtstreeks met de library maar verstuurt commando's naar deze worker
//! en ontvangt events terug. Elke stap wordt als log-event naar de UI gestuurd
//! voor duidelijke feedback, en elke event laat de UI hervappen.

use std::collections::VecDeque;
use std::ffi::{c_char, c_int, c_longlong, c_uint, c_void};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::ffi;
use crate::ffi::{
    BURN_DRIVE_ADR_LEN, BURN_MSGS_MESSAGE_LEN, DiscStatus, DriveInfo, DriveStatus, Progress,
    RawLibburn, SpeedDescriptor, cbuf_to_string,
};
use crate::i18n::Lang;
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
    /// Stap 8: mediacode van de schijf (ADIP/ATIP), incl. fabrikant-schatting.
    pub media_id: Option<MediaId>,
    /// Stap 8: BD spare-info (defect management): (toegewezen, vrij) blokken.
    pub bd_spare: Option<(i32, i32)>,
}

/// Mediacode van de ingelegde schijf (via `burn_disc_get_media_id`).
#[derive(Clone, Debug)]
pub struct MediaId {
    /// Printbare combinatie van fabrikant + media-id, bijv. "PHILIP R04".
    pub product_id: String,
    /// Book type-tekst (alleen DVD/BD; NULL bij CD).
    pub book_type: Option<String>,
    /// Fabrikantnaam via `burn_guess_manufacturer` (geen match = geen).
    pub manufacturer: Option<String>,
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
    /// Profielcodes die de drive ondersteunt (o.a. de enige bron voor
    /// DVD+R/DVD+RW/DVD-R DL/BD-capabilities — de burn_drive_info-bitvelden
    /// bevatten die niet).
    pub supported_profiles: Vec<i32>,
    pub media: Option<MediaInfo>,
    /// Reden waarom de laatste inspectie mislukte (voor feedback in de UI).
    pub inspect_error: Option<String>,
}

impl DriveEntry {
    pub fn display_name(&self, unnamed_prefix: &str) -> String {
        let name = format!("{} {}", self.vendor, self.product)
            .trim()
            .to_string();
        if name.is_empty() {
            format!("{unnamed_prefix} {}", self.index)
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
    /// `import_start_block` = beg blok van de te importeren sessie (multi-session
    /// op schrijf-eenmalige media).
    BurnFiles {
        index: usize,
        paths: Vec<String>,
        volume_id: String,
        settings: BurnSettings,
        import_start_block: Option<i32>,
    },
    /// Wis de media los van een brandjob (stap 6).
    EraseDisc {
        index: usize,
        fast: bool,
    },
    /// Formatteer de media los van een brandjob (stap 6).
    FormatDisc {
        index: usize,
        settings: BurnSettings,
    },
    /// Actieve job op een station annuleren (brand/wis/format).
    CancelBurn {
        index: usize,
    },
    /// Interfacetaal bijwerken; de worker gebruikt deze voor zijn logregels
    /// (stap 10b-6).
    SetLanguage(Lang),
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
        phase: DriveStatus,
        /// Verstreken tijd in seconden.
        elapsed_secs: f64,
        /// Verwachte resterende tijd in seconden (0 = onbekend).
        eta_secs: f64,
    },
    BurnDone {
        index: usize,
    },
    BurnFailed {
        index: usize,
        error: String,
    },
    BurnCancelled {
        index: usize,
    },
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
    /// De schijf is uit de drive genomen (eject) — mediagegevens zijn verouderd.
    MediaEjected {
        index: usize,
    },
    WorkerStopped,
}

/// Soort onderhoudsjob (UI-labels staan in de i18n-catalogus, MaintTexts).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaintKind {
    Erase,
    Format,
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
    /// Interfacetaal voor de logregels van de worker (stap 10b-6 gebruikt
    /// dit; hier wordt alleen de taal bijgehouden).
    #[allow(dead_code)]
    lang: Lang,
}

/// Soort actieve job.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum JobKind {
    Write { simulate: bool },
    Erase,
    Format,
}

/// libburn/libisofs-resources van een write-job die pas na afloop
/// vrijgegeven worden.
struct WriteRes {
    opts: *mut ffi::BurnWriteOpts,
    disc: *mut ffi::BurnDisc,
    session: *mut ffi::BurnSession,
    track: *mut ffi::BurnTrack,
    source: *mut ffi::BurnSource,
    /// libisofs data-bron bij multi-session import (blijft alive tot na de
    /// brand: oude bestandsdata wordt tijdens het schrijven gelezen).
    data_src: *mut crate::isofs::IsoDataSource,
    iso_opts: *mut crate::isofs::IsoWriteOpts,
    image: *mut crate::isofs::IsoImage,
}

/// Actieve job die niet-blokkerend wordt gepolld (stap 9b: meerdere stations
/// tegelijk). libburn draait brandjobs in eigen threads; onze poll leest
/// alleen de status voor voortgang en afronding.
struct ActiveJob {
    index: usize,
    adr: String,
    drive: *mut ffi::BurnDrive,
    kind: JobKind,
    write: Option<WriteJob>,
    started: std::time::Instant,
    last_progress: std::time::Instant,
    last_pct: Option<i32>,
    /// Laatst geziene payload-sector — basis voor de gemiddelde snelheid
    /// die bij het afronden van de job in het log komt.
    last_sector: i32,
    cancelled: bool,
}

struct WriteJob {
    s: BurnSettings,
    res: WriteRes,
}

fn run(cmds: Receiver<Command>, events: Sender<Event>, ctx: egui::Context) {
    let notify = Notifier { tx: &events, ctx };
    let mut state: Option<WorkerState> = None;
    let mut pending: VecDeque<Command> = VecDeque::new();
    let mut active: Vec<ActiveJob> = Vec::new();
    let mut shutdown = false;
    // Actieve taal in de worker; overleeft ook een herladning van libburn.
    let mut lang = Lang::default();

    // Stap 9b: meerdere stations tegelijk. libburn draait brandjobs in eigen
    // threads; deze lus polt alle actieve jobs voor voortgang en afronding.
    while !shutdown || !active.is_empty() {
        // 1) Eén commando ophalen. Met actieve jobs niet-blokkerend.
        let cmd = if let Some(c) = pending.pop_front() {
            Some(c)
        } else if !active.is_empty() {
            cmds.try_recv().ok()
        } else if !shutdown {
            match cmds.recv_timeout(Duration::from_millis(200)) {
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => {
                    shutdown = true;
                    None
                }
            }
        } else {
            None
        };

        if let Some(cmd) = cmd {
            match cmd {
                Command::SetLanguage(l) => {
                    lang = l;
                    if let Some(st) = state.as_mut() {
                        st.lang = l;
                    }
                }
                Command::LoadLibrary { path, exclusive } => {
                    if active.is_empty() {
                        load_library(&mut state, path, exclusive, lang, &notify);
                    } else {
                        notify.log(Level::Warning, lang.w_reload_busy());
                    }
                }
                Command::Scan => {
                    if active.is_empty() {
                        scan(&mut state, lang, &cmds, &mut pending, &notify);
                    } else {
                        notify.log(Level::Warning, lang.w_scan_busy());
                    }
                }
                Command::InspectDrive(index) => {
                    if active.iter().any(|j| j.index == index) {
                        notify.log(Level::Warning, lang.w_inspect_busy_job(index));
                    } else {
                        inspect(state.as_ref(), lang, index, &notify);
                    }
                }
                Command::ReadDisc { index, path } => {
                    if !active.is_empty() {
                        notify.log(Level::Warning, lang.w_read_busy());
                    } else {
                        read_disc(&state, lang, index, &path, &cmds, &mut pending, &notify)
                    }
                }
                Command::CancelRead => {
                    notify.log(Level::Warning, lang.w_cancel_no_read());
                }
                Command::BurnDisc {
                    index,
                    path,
                    settings,
                } => burn_job(&state, lang, index, &path, &settings, &mut active, &notify),
                Command::BurnFiles {
                    index,
                    paths,
                    volume_id,
                    settings,
                    import_start_block,
                } => burn_files_job(
                    &state,
                    lang,
                    index,
                    &paths,
                    &volume_id,
                    import_start_block,
                    &settings,
                    &mut active,
                    &notify,
                ),
                Command::CancelBurn { index } => {
                    if let Some(st) = state.as_ref() {
                        if let Some(job) = active.iter_mut().find(|j| j.index == index) {
                            unsafe { (st.raw.drive_cancel)(job.drive) };
                            job.cancelled = true;
                            notify.log(Level::Info, lang.w_cancel_requested(index));
                        } else {
                            notify.log(Level::Warning, lang.w_cancel_no_job(index));
                        }
                    }
                }
                Command::EraseDisc { index, fast } => {
                    erase_job(&state, lang, index, fast, &mut active, &notify)
                }
                Command::FormatDisc { index, settings } => {
                    format_job(&state, lang, index, &settings, &mut active, &notify)
                }
                Command::Shutdown => {
                    shutdown = true;
                    if let Some(st) = state.as_ref() {
                        for job in &mut active {
                            unsafe { (st.raw.drive_cancel)(job.drive) };
                            job.cancelled = true;
                        }
                    }
                }
            }
        }

        // 2) Actieve jobs pollen (voortgang + afronding).
        if !active.is_empty() {
            let Some(st) = state.as_ref() else {
                active.clear();
                continue;
            };
            // libburn-meldingen live doorgeven (fouten zichtbaar tijdens de
            // job, niet pas bij de afronding).
            drain_msgs(&st.raw, &notify);
            let mut keep: Vec<ActiveJob> = Vec::new();
            while let Some(mut job) = active.pop() {
                let done = unsafe { poll_job(st, &mut job, &notify) };
                if done {
                    unsafe { finalize_job(st, &mut job, &notify) };
                } else {
                    keep.push(job);
                }
            }
            keep.reverse();
            active = keep;
            if !active.is_empty() {
                thread::sleep(Duration::from_millis(120));
            }
        }
    }

    shutdown_lib(state, lang, &notify);
    let _ = events.send(Event::WorkerStopped);
}

fn load_library(
    state: &mut Option<WorkerState>,
    custom: Option<String>,
    exclusive: bool,
    lang: Lang,
    notify: &Notifier,
) {
    // Een eventueel eerdere sessie netjes afsluiten.
    shutdown_lib(state.take(), lang, notify);

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
                    last_err = lang.w_burn_init_failed(cand);
                    continue;
                }
                // Apparaat-openingsbeleid (vlak na initialize, vóór de scan):
                // exclusief (O_EXCL) of niet — uitzetten als de bestandsbeheerder
                // de schijf heeft aangekoppeld (automount).
                (raw.preset_device_open)(if exclusive { 1 } else { 0 }, 0, 0);
                // libburn-meldingen: vanaf DEBUG in de queue, niets
                // rechtstreeks naar stderr; de worker haalt de queue leeg.
                // DEBUG is bewust inbegrepen: SCSI-error-condities op losse
                // commando's meldt libburn alleen als DEBUG — zonder deze
                // drempel blijven format-/wis-fouten onzichtbaar.
                (raw.msgs_set_severities)(
                    c"DEBUG".as_ptr(),
                    c"NEVER".as_ptr(),
                    c"burnr: ".as_ptr(),
                );
                notify.log(Level::Info, lang.w_device_open_mode(exclusive));
                let (mut maj, mut min, mut mic) = (0, 0, 0);
                (raw.version)(&mut maj, &mut min, &mut mic);
                let version = format!("{maj}.{min}.{mic}");
                notify.log(Level::Success, lang.w_lib_loaded(&version, cand));
                notify.send(Event::LibLoaded {
                    version,
                    path: cand.clone(),
                });
                // libisofs erbij laden (optioneel — alleen nodig voor
                // “bestanden samenstellen”; ISO-branden werkt ook zonder).
                let isofs = load_isofs(lang, notify);
                *state = Some(WorkerState {
                    raw,
                    isofs,
                    lang,
                    infos: std::ptr::null_mut(),
                    n_drives: 0,
                });
                return;
            },
            Err(e) => last_err = format!("`{cand}`: {e}"),
        }
    }

    notify.log(Level::Error, lang.w_lib_failed(&last_err));
    notify.send(Event::LibLoadFailed { error: last_err });
}

fn scan(
    state: &mut Option<WorkerState>,
    lang: Lang,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    let Some(st) = state.as_mut() else {
        notify.log(Level::Warning, lang.w_scan_no_lib());
        return;
    };

    // Volgens libburn.h verplicht: de oude drive-info-array vrijgeven vóór een
    // nieuwe scan (alle pointers worden ongeldig).
    free_drive_list(st);

    notify.log(Level::Info, lang.w_scanning());
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
            let msg = lang.w_scan_failed(ret);
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

    notify.log(Level::Success, lang.w_scan_done(count));
    notify.send(Event::ScanDone { drives });
}

fn drive_entry_from_raw(index: usize, di: &DriveInfo, raw: &RawLibburn) -> DriveEntry {
    let f = di.flags;
    let bit = |n: u32| (f >> n) & 1 != 0;

    let adr = unsafe { adr_of(di, raw) };

    // Profiellijst van de drive: de enige bron voor BD/DVD+R-capabilities
    // (zie libburn.h bij burn_drive_info). Caller-provided arrays, geen free.
    let mut supported_profiles: Vec<i32> = Vec::new();
    unsafe {
        let mut num: c_int = 0;
        let mut profiles = [0 as c_int; 64];
        let mut is_current = [0 as c_char; 64];
        if (raw.drive_get_all_profiles)(
            di.drive,
            &mut num,
            profiles.as_mut_ptr(),
            is_current.as_mut_ptr(),
        ) == 1
            && num > 0
        {
            supported_profiles.extend_from_slice(&profiles[..num as usize]);
        }
    }

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
        supported_profiles,
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

fn inspect(state: Option<&WorkerState>, lang: Lang, index: usize, notify: &Notifier) {
    let Some(st) = state else {
        notify.log(Level::Warning, lang.w_inspect_no_lib());
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(Level::Warning, lang.w_inspect_unknown(index));
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
    notify.log(Level::Info, lang.w_inspect_grab(index, &name));

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = lang.w_grab_failed(index, &adr);
        notify.log(Level::Error, msg.clone());
        notify.send(Event::InspectFailed { index, error: msg });
        return;
    }
    notify.log(Level::Info, lang.w_inspect_grabbed(index));

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
    notify.log(Level::Info, lang.w_inspect_media(index, disc));

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
            lang.w_inspect_profile(index, &profile_name, profile_no),
        );
    }

    // Leesbare capaciteit (kan falen op lege media — geen probleem).
    // Let op: libburn meldt na een mislukte READ CAPACITY nog "1 blok"
    // als succes (quirk in burn_get_read_capacity_v2); daarom > 1.
    let mut capacity: c_int = 0;
    let read_capacity =
        if unsafe { (st.raw.get_read_capacity)(di.drive, &mut capacity, 0) } == 1 && capacity > 1 {
            Some(capacity)
        } else {
            None
        };
    if let Some(blocks) = read_capacity {
        notify.log(
            Level::Info,
            lang.w_inspect_capacity(index, &format_blocks(blocks)),
        );
    }

    let erasable = unsafe { (st.raw.disc_erasable)(di.drive) } != 0;

    // Stap 8: mediacode (ADIP/ATIP) en BD spare-info (defect management).
    let mut media_id: Option<MediaId> = None;
    let mut bd_spare: Option<(i32, i32)> = None;
    if disc != DiscStatus::Empty && disc != DiscStatus::Unready {
        unsafe {
            let mut product: *mut c_char = std::ptr::null_mut();
            let mut m1: *mut c_char = std::ptr::null_mut();
            let mut m2: *mut c_char = std::ptr::null_mut();
            let mut bt: *mut c_char = std::ptr::null_mut();
            let r =
                (st.raw.disc_get_media_id)(di.drive, &mut product, &mut m1, &mut m2, &mut bt, 0);
            if r == 1 {
                let product_id = ffi::take_cstring(product);
                let code1 = ffi::take_cstring(m1);
                let code2 = ffi::take_cstring(m2);
                let book = ffi::take_cstring(bt);

                // Fabrikant-schatting via libburn's ingebouwde lijst.
                let manuf = match (&code1, &code2) {
                    (Some(c1), Some(c2)) => {
                        let c1c = std::ffi::CString::new(c1.as_str()).ok();
                        let c2c = std::ffi::CString::new(c2.as_str()).ok();
                        if let (Some(a), Some(b)) = (c1c, c2c) {
                            let p = (st.raw.guess_manufacturer)(
                                profile_no,
                                a.as_ptr() as *mut c_char,
                                b.as_ptr() as *mut c_char,
                                0,
                            );
                            if p.is_null() {
                                None
                            } else {
                                let t = std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned();
                                ffi::free_c(p);
                                if t.starts_with("Unknown") {
                                    None
                                } else {
                                    Some(t)
                                }
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(pid) = &product_id {
                    notify.log(
                        Level::Info,
                        lang.w_inspect_media_code(index, pid, manuf.as_deref().unwrap_or("")),
                    );
                }
                media_id = Some(MediaId {
                    product_id: product_id.unwrap_or_default(),
                    book_type: book,
                    manufacturer: manuf,
                });
                // code1/code2 zijn alleen lokaal gebruikt voor de
                // fabrikant-schatting en worden hieronder verwijderd.
                drop(code1);
                drop(code2);
            }

            // Defect management-status bij BD-media (spare-gebieden).
            if matches!(profile_no, 0x41..=0x43) {
                let (mut alloc, mut free_b): (c_int, c_int) = (0, 0);
                let r = (st.raw.disc_get_bd_spare_info)(di.drive, &mut alloc, &mut free_b, 0);
                if r == 1 && alloc > 0 {
                    bd_spare = Some((alloc, free_b));
                    notify.log(Level::Info, lang.w_inspect_dm_active(index, free_b, alloc));
                } else {
                    notify.log(Level::Info, lang.w_inspect_dm_off());
                }
            }
        }
    }

    // Stap 2: TOC-model van de schijf opbouwen (alleen zinvol bij media met
    // inhoud; bij lege media geeft de drive NULL of een leeg model).
    let (sessions, incomplete) = unsafe { read_toc(&st.raw, di.drive) };
    if !sessions.is_empty() {
        let total_tracks: usize = sessions.iter().map(|s| s.tracks.len()).sum();
        notify.log(
            Level::Info,
            lang.w_inspect_toc(index, sessions.len(), total_tracks),
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
        notify.log(Level::Info, lang.w_inspect_speeds(index, speeds.len()));
    }

    let mut prog = Progress::default();
    let drive_status =
        DriveStatus::from_raw(unsafe { (st.raw.drive_get_status)(di.drive, &mut prog) });

    unsafe { (st.raw.drive_release)(di.drive, 0) };
    notify.log(Level::Success, lang.w_inspect_done(index));
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
            media_id,
            bd_spare,
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
    lang: Lang,
    index: usize,
    path: &str,
    s: &BurnSettings,
    active: &mut Vec<ActiveJob>,
    notify: &Notifier,
) {
    use crate::settings::WriteMode;

    let Some(st) = state else {
        notify.log(Level::Warning, lang.w_burn_no_lib());
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(Level::Warning, lang.w_burn_unknown(index));
        return;
    }
    let path = path.trim();
    if path.is_empty() {
        notify.log(Level::Warning, lang.w_burn_no_iso());
        return;
    }
    if s.write_mode == WriteMode::Raw {
        let msg = lang.w_raw_iso_unsupported();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }

    // Bronbestand controleren.
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            let msg = lang.w_burn_open_failed(path, &e.to_string());
            notify.log(Level::Error, &msg);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };
    let file_size = meta.len();
    if file_size == 0 {
        let msg = lang.w_empty_iso();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    if file_size % 2048 != 0 {
        notify.log(Level::Info, lang.w_padding_note());
    }
    let expected_sectors = ((file_size as i64 + 2047) / 2048) as i32;

    let di = unsafe { *st.infos.add(index) };
    notify.log(
        Level::Info,
        lang.w_burn_starting(index, path, expected_sectors),
    );
    notify.log(
        Level::Info,
        lang.w_settings_line(s.simulate, s.multi_session, s.padding_kib, s.overburn, None),
    );

    // Grab + media-check (gedeelde flow).
    if grab_and_check_media(st, &di, index, notify).is_err() {
        return;
    }

    // Ruimtecheck (indicatief; de echte precheck doet libburn zelf).
    let avail = unsafe { (st.raw.disc_available_space)(di.drive, std::ptr::null_mut()) };
    if avail >= 0 && (avail as i64) < file_size as i64 {
        notify.log(
            Level::Warning,
            lang.w_iso_size_warning(
                &format_blocks(((file_size as i64 + 2047) / 2048) as i32),
                &format_blocks(avail as i32),
            ),
        );
    }

    // Snelheid: maximaal (0) of een door de user gekozen maximum (kB/s).
    // libburn brandt nooit sneller dan dit maximum en ook nooit sneller dan
    // media/drive toelaten.
    let write_speed = if s.speed_max { 0 } else { s.speed_kbps };
    unsafe { (st.raw.drive_set_speed)(di.drive, 0, write_speed) };
    notify.log(Level::Info, lang.w_speed(s.speed_max, s.speed_kbps));

    // Disc-model: disc → session → track met file-bron + FIFO.
    let path_c = match std::ffi::CString::new(path) {
        Ok(c) => c,
        Err(_) => {
            let msg = lang.w_bad_path_nul();
            notify.log(Level::Error, &msg);
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };
    let (disc, session, track, fifo, opts) = match unsafe {
        build_burn_model(
            &st.raw,
            di.drive,
            path_c.as_ptr(),
            file_size,
            s,
            st.lang,
            notify,
        )
    } {
        Ok(model) => model,
        Err(msg) => {
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };

    // Branden starten (asynchroon — libburn draait de job in eigen threads);
    // de job wordt aan de actieve lijst toegevoegd en door de hoofdlus gepolld.
    unsafe { (st.raw.disc_write)(opts, disc) };
    notify.send(Event::BurnStarted {
        index,
        total_sectors: expected_sectors,
        simulate: s.simulate,
    });
    notify.log(Level::Info, lang.w_burn_started(index, s.simulate));
    let adr = unsafe { adr_of(&di, &st.raw) };
    active.push(ActiveJob {
        index,
        adr,
        drive: di.drive,
        kind: JobKind::Write {
            simulate: s.simulate,
        },
        write: Some(WriteJob {
            s: s.clone(),
            res: WriteRes {
                opts,
                disc,
                session,
                track,
                source: fifo,
                data_src: std::ptr::null_mut(),
                iso_opts: std::ptr::null_mut(),
                image: std::ptr::null_mut(),
            },
        }),
        started: std::time::Instant::now(),
        last_progress: std::time::Instant::now(),
        last_pct: Some(i32::MIN),
        last_sector: 0,
        cancelled: false,
    });
}

/// Stelt een data-image samen uit bestanden/mappen (libisofs) en brandt die
/// direct naar het station — `iso_image_create_burn_source()` levert een
/// burn_source die aan de libburn-track wordt gekoppeld (geen tussenbestand).
#[allow(clippy::too_many_arguments)]
fn burn_files_job(
    state: &Option<WorkerState>,
    lang: Lang,
    index: usize,
    paths: &[String],
    volume_id: &str,
    import_start_block: Option<i32>,
    s: &BurnSettings,
    active: &mut Vec<ActiveJob>,
    notify: &Notifier,
) {
    use crate::settings::WriteMode;

    let Some(st) = state else {
        notify.log(Level::Warning, lang.w_burn_no_lib());
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(Level::Warning, lang.w_burn_unknown(index));
        return;
    }
    let Some(iso) = st.isofs.as_ref() else {
        let msg = lang.w_isofs_missing();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    };
    if paths.is_empty() {
        notify.log(Level::Warning, lang.w_burn_no_files());
        return;
    }
    if s.write_mode == WriteMode::Raw {
        let msg = lang.w_raw_data_unsupported();
        notify.log(Level::Error, &msg);
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    for p in paths {
        if !std::path::Path::new(p).exists() {
            let msg = lang.w_burn_file_missing(p);
            notify.log(Level::Error, &msg);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    }

    let di = unsafe { *st.infos.add(index) };
    let adr = unsafe { adr_of(&di, &st.raw) };
    notify.log(Level::Info, lang.w_data_image_starting(index, paths.len()));
    notify.log(
        Level::Info,
        lang.w_settings_line(
            s.simulate,
            s.multi_session,
            s.padding_kib,
            s.overburn,
            Some(s.keep_timestamps),
        ),
    );

    // ISO-image opbouwen in libisofs. Bij multi-session op schrijf-eenmalige
    // media wordt de bestaande sessie EERST geïmporteerd (vóór de grab, zodat
    // libisofs het device zelf kan openen); de nieuwe bestanden komen dan in
    // dezelfde boom als de bestaande inhoud.
    let vol = if volume_id.trim().is_empty() {
        format!("Libburn{}", chrono::Local::now().format("%Y%m%d"))
    } else {
        volume_id.trim().to_string()
    };
    let vol_c = match std::ffi::CString::new(vol.as_str()) {
        Ok(c) => c,
        Err(_) => {
            let msg = lang.w_bad_volume_nul();
            notify.log(Level::Error, &msg);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };
    let image = unsafe {
        let mut image: *mut crate::isofs::IsoImage = std::ptr::null_mut();
        if (iso.image_new)(vol_c.as_ptr(), &mut image) != ISO_SUCCESS || image.is_null() {
            let msg = lang.w_image_create_failed();
            notify.log(Level::Error, &msg);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
        image
    };

    // Bestaande sessie importeren (multi-session op schrijf-eenmalige media).
    // De data-bron blijft bewust alive tot na de brand: oude bestandsdata
    // wordt tijdens het branden van de schijf gelezen.
    let mut data_src: *mut crate::isofs::IsoDataSource = std::ptr::null_mut();
    if let Some(start_block) = import_start_block {
        notify.log(Level::Info, lang.w_import_start(start_block));
        let dev_c = match std::ffi::CString::new(adr.as_str()) {
            Ok(c) => c,
            Err(_) => {
                let msg = lang.w_bad_dev_nul();
                notify.log(Level::Error, &msg);
                unsafe { (iso.image_unref)(image) };
                notify.send(Event::BurnFailed { index, error: msg });
                return;
            }
        };
        unsafe {
            let mut ds: *mut crate::isofs::IsoDataSource = std::ptr::null_mut();
            if (iso.data_source_new_from_file)(dev_c.as_ptr(), &mut ds) != ISO_SUCCESS
                || ds.is_null()
            {
                let msg = lang.w_import_open_failed();
                notify.log(Level::Error, &msg);
                (iso.image_unref)(image);
                notify.send(Event::BurnFailed { index, error: msg });
                return;
            }
            let mut ropts: *mut crate::isofs::IsoReadOpts = std::ptr::null_mut();
            if (iso.read_opts_new)(&mut ropts, 0) != ISO_SUCCESS || ropts.is_null() {
                let msg = lang.w_read_opts_failed();
                notify.log(Level::Error, &msg);
                (iso.data_source_unref)(ds);
                (iso.image_unref)(image);
                notify.send(Event::BurnFailed { index, error: msg });
                return;
            }
            (iso.read_opts_set_start_block)(ropts, start_block as c_uint);
            let mut features: *mut crate::isofs::IsoReadImageFeatures = std::ptr::null_mut();
            let r = (iso.image_import)(image, ds, ropts, &mut features);
            if !features.is_null() {
                let blocks = (iso.read_image_features_get_size)(features);
                notify.log(
                    Level::Info,
                    lang.w_import_done(blocks as i32, &format_blocks(blocks as i32)),
                );
                (iso.read_image_features_destroy)(features);
            }
            (iso.read_opts_free)(ropts);
            drain_iso_msgs(iso, notify);
            if r != ISO_SUCCESS {
                let msg = lang.w_import_failed();
                notify.log(Level::Error, &msg);
                (iso.data_source_unref)(ds);
                (iso.image_unref)(image);
                notify.send(Event::BurnFailed { index, error: msg });
                return;
            }
            data_src = ds; // alive tot na de brand
        }
    }

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
                    notify.log(Level::Info, lang.w_added_folder(p));
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
                    notify.log(Level::Info, lang.w_added_file(p));
                }
            }
        }
    }
    if !add_errors.is_empty() {
        for e in &add_errors {
            notify.log(Level::Error, lang.w_item_skipped(e));
        }
        if add_errors.len() == paths.len() {
            let msg = lang.w_nothing_added();
            notify.log(Level::Error, &msg);
            unsafe { (iso.image_unref)(image) };
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    }

    // Grab + media-check (gedeelde flow) — ná het importeren, want libisofs
    // mocht het device zelf openen. De libisofs-data bron houdt het device
    // wel open (oude bestandsdata wordt tijdens het branden gelezen); een
    // niet-exclusieve grab laat dat toe.
    if grab_and_check_media(st, &di, index, notify).is_err() {
        unsafe { (iso.image_unref)(image) };
        return;
    }

    // Snelheid: maximaal (0) of een door de user gekozen maximum (kB/s).
    let write_speed = if s.speed_max { 0 } else { s.speed_kbps };
    unsafe { (st.raw.drive_set_speed)(di.drive, 0, write_speed) };
    notify.log(Level::Info, lang.w_speed(s.speed_max, s.speed_kbps));

    // NWA: waar de nieuwe sessie begint (multi-session import).
    let (mut lba, mut nwa): (c_int, c_int) = (0, 0);
    let nwa_ok = unsafe {
        (st.raw.disc_track_lba_nwa)(di.drive, std::ptr::null_mut(), 0, &mut lba, &mut nwa)
    } == 1;
    if import_start_block.is_some() && !nwa_ok {
        let msg = lang.w_nwa_failed();
        notify.log(Level::Error, &msg);
        unsafe { (iso.image_unref)(image) };
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    if import_start_block.is_some() {
        notify.log(Level::Info, lang.w_nwa(nwa));
    }

    // libisofs write-opts: profiel 2 (DISTRIBUTION) = Rock Ridge + Joliet.
    let iso_opts = unsafe {
        let mut opts: *mut crate::isofs::IsoWriteOpts = std::ptr::null_mut();
        if (iso.write_opts_new)(&mut opts, 2) != ISO_SUCCESS || opts.is_null() {
            let msg = lang.w_isofs_opts_failed();
            notify.log(Level::Error, &msg);
            (iso.image_unref)(image);
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
        // Multi-session: appendable + ms_block zodat libisofs verwijzingen
        // naar de oude bestandsdata in de vorige sessie kan opnemen.
        if import_start_block.is_some() {
            (iso.write_opts_set_appendable)(opts, 1);
            (iso.write_opts_set_ms_block)(opts, nwa as c_uint);
        }
        opts
    };

    // Image-layout berekenen en burn_source opvragen (kan even duren).
    notify.log(Level::Info, lang.w_layout());
    let src_raw = unsafe {
        let mut src: *mut ffi::BurnSource = std::ptr::null_mut();
        let r = (iso.image_create_burn_source)(image, iso_opts, &mut src);
        drain_iso_msgs(iso, notify);
        if r != ISO_SUCCESS || src.is_null() {
            let msg = lang.w_compose_failed();
            notify.log(Level::Error, &msg);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
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
        let msg = lang.w_fifo_failed();
        notify.log(Level::Error, &msg);
        unsafe {
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
        }
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    let src = fifo;

    // Grootte via de get_size-callback van de burn_source.
    let size = unsafe { burn_source_get_size(src) };
    if size <= 0 {
        let msg = lang.w_size_unknown();
        notify.log(Level::Error, &msg);
        unsafe {
            (st.raw.source_free)(src);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
        }
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    let expected_sectors = ((size + 2047) / 2048) as i32;
    notify.log(
        Level::Info,
        lang.w_image_ready(
            expected_sectors,
            &format_blocks(expected_sectors),
            paths.len(),
        ),
    );

    // Ruimtecheck (indicatief).
    let avail = unsafe { (st.raw.disc_available_space)(di.drive, std::ptr::null_mut()) };
    if avail >= 0 && (avail as i64) < size {
        notify.log(
            Level::Warning,
            lang.w_image_size_warning(
                &format_blocks(expected_sectors),
                &format_blocks(avail as i32),
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
            let msg = lang.w_model_failed();
            notify.log(Level::Error, &msg);
            (st.raw.source_free)(src);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
        (disc, session, track)
    };
    let model_ok = unsafe {
        (st.raw.track_set_source)(track, src) == 0 && (st.raw.track_set_size)(track, size) == 1
    };
    if !model_ok {
        let msg = lang.w_attach_failed();
        notify.log(Level::Error, &msg);
        unsafe {
            (st.raw.track_free)(track);
            (st.raw.session_free)(session);
            (st.raw.disc_free)(disc);
            (st.raw.source_free)(src);
            (iso.write_opts_free)(iso_opts);
            (iso.image_unref)(image);
        }
        notify.send(Event::BurnFailed { index, error: msg });
        return;
    }
    unsafe { (st.raw.track_define_data)(track, 0, s.padding_kib * 1024, 1, ffi::BURN_MODE1) };

    let opts = match unsafe { make_write_opts(&st.raw, di.drive, disc, s, lang, notify) } {
        Ok(o) => o,
        Err(msg) => {
            unsafe {
                (st.raw.source_free)(src);
                (st.raw.track_free)(track);
                (st.raw.session_free)(session);
                (st.raw.disc_free)(disc);
                (iso.write_opts_free)(iso_opts);
                (iso.image_unref)(image);
            }
            notify.send(Event::BurnFailed { index, error: msg });
            return;
        }
    };

    // Branden starten (asynchroon); de libisofs-objecten (image, write-opts,
    // data-bron) blijven alive tot na de brand — oude bestandsdata wordt
    // tijdens het schrijven van de schijf gelezen.
    unsafe { (st.raw.disc_write)(opts, disc) };
    notify.send(Event::BurnStarted {
        index,
        total_sectors: expected_sectors,
        simulate: s.simulate,
    });
    notify.log(Level::Info, lang.w_burn_started(index, s.simulate));
    active.push(ActiveJob {
        index,
        adr,
        drive: di.drive,
        kind: JobKind::Write {
            simulate: s.simulate,
        },
        write: Some(WriteJob {
            s: s.clone(),
            res: WriteRes {
                opts,
                disc,
                session,
                track,
                source: src,
                data_src,
                iso_opts,
                image,
            },
        }),
        started: std::time::Instant::now(),
        last_progress: std::time::Instant::now(),
        last_pct: Some(i32::MIN),
        last_sector: 0,
        cancelled: false,
    });
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

/// Grab de drive en controleer de media-status. Bij fout: log + BurnFailed +
/// drive vrijgeven, Err(()).
fn grab_and_check_media(
    st: &WorkerState,
    di: &DriveInfo,
    index: usize,
    notify: &Notifier,
) -> Result<DiscStatus, ()> {
    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(di, &st.raw) };
        let msg = st.lang.w_grab_failed(index, &adr);
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

    if status != DiscStatus::Blank && status != DiscStatus::Appendable {
        let msg = st.lang.w_media_not_writable(status);
        notify.log(Level::Error, &msg);
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        notify.send(Event::BurnFailed { index, error: msg });
        return Err(());
    }
    if status == DiscStatus::Appendable {
        notify.log(Level::Warning, st.lang.w_media_appendable());
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
    lang: Lang,
    notify: &Notifier,
) -> Result<*mut ffi::BurnWriteOpts, String> {
    use crate::settings::{MultiSession, WriteMode};

    unsafe {
        let opts = (raw.write_opts_new)(drive);
        if opts.is_null() {
            let msg = lang.w_make_opts_failed();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        (raw.write_opts_set_perform_opc)(opts, 0);
        (raw.write_opts_set_simulate)(opts, s.simulate as c_int);
        let mut multi = match s.multi_session {
            MultiSession::Yes => 1,
            MultiSession::No => 0,
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
                notify.log(Level::Info, lang.w_overwritable_note(profile_no));
                multi = 0;
            }
        }
        (raw.write_opts_set_multi)(opts, multi);
        (raw.write_opts_set_underrun_proof)(opts, s.underrun_proof as c_int);
        if s.overburn {
            (raw.write_opts_set_force)(opts, 1);
            notify.log(Level::Info, lang.w_overburn_note());
        }

        match s.write_mode {
            WriteMode::Auto => {
                let mut reasons = [0 as c_char; ffi::BURN_REASONS_LEN];
                let wt = (raw.write_opts_auto_write_type)(opts, disc, reasons.as_mut_ptr(), 0);
                if wt == ffi::BURN_WRITE_NONE {
                    let reasons = cbuf_to_string(&reasons);
                    let mut msg = lang.w_write_mode_failed(&reasons);
                    notify.log(Level::Error, &msg);
                    if let Some(hint) = simulation_hint(s, &reasons, lang) {
                        notify.log(Level::Warning, hint.clone());
                        msg.push_str(" — ");
                        msg.push_str(&hint);
                    }
                    (raw.write_opts_free)(opts);
                    return Err(msg);
                }
                notify.log(Level::Info, lang.w_write_mode_auto(write_type_name(wt)));
            }
            WriteMode::Tao => {
                if (raw.write_opts_set_write_type)(opts, ffi::BURN_WRITE_TAO, ffi::BURN_BLOCK_MODE1)
                    != 1
                {
                    let msg = lang.w_tao_unsupported();
                    notify.log(Level::Error, &msg);
                    (raw.write_opts_free)(opts);
                    return Err(msg);
                }
                notify.log(Level::Info, lang.w_write_mode_tao());
                if let Some(err) = precheck_write_opts(raw, opts, disc, s, lang) {
                    notify.log(Level::Error, &err);
                    if let Some(hint) = simulation_hint(s, &err, lang) {
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
                    let msg = lang.w_sao_unsupported();
                    notify.log(Level::Error, &msg);
                    (raw.write_opts_free)(opts);
                    return Err(msg);
                }
                notify.log(Level::Info, lang.w_write_mode_sao());
                if let Some(err) = precheck_write_opts(raw, opts, disc, s, lang) {
                    notify.log(Level::Error, &err);
                    if let Some(hint) = simulation_hint(s, &err, lang) {
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
/// Pollt één actieve job (voortgang + annuleer-status). Geeft true als de
/// job klaar is (IDLE) en afgewerkt moet worden via `finalize_job`.
unsafe fn poll_job(st: &WorkerState, job: &mut ActiveJob, notify: &Notifier) -> bool {
    unsafe {
        let mut prog = Progress::default();
        let ds = DriveStatus::from_raw((st.raw.drive_get_status)(job.drive, &mut prog));
        if ds == DriveStatus::Idle {
            return true;
        }
        if let Some(wj) = &job.write {
            // Laatst bekende sector bijwerken (vóór de 200 ms-throttle),
            // zodat finalize_job de gemiddelde snelheid kan berekenen.
            if prog.sector > job.last_sector {
                job.last_sector = prog.sector;
            }
            // Write-job: voortgang tijdens de schrijf-fasen.
            if matches!(
                ds,
                DriveStatus::Writing
                    | DriveStatus::WritingLeadin
                    | DriveStatus::WritingLeadout
                    | DriveStatus::WritingPregap
                    | DriveStatus::ClosingTrack
                    | DriveStatus::ClosingSession
            ) && prog.sectors > 0
                && job.last_progress.elapsed() >= Duration::from_millis(200)
            {
                let elapsed = job.started.elapsed().as_secs_f64();
                let secs = elapsed.max(0.001);
                let kbps = (prog.sector as f64 * 2048.0) / secs / 1000.0;
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
                let fifo_pct = fifo_fill_pct(&st.raw, wj.res.source);
                notify.send(Event::BurnProgress {
                    index: job.index,
                    sector: prog.sector,
                    sectors: prog.sectors,
                    kbps,
                    buffer_pct,
                    fifo_pct,
                    phase: ds,
                    elapsed_secs: elapsed,
                    eta_secs: eta,
                });
                job.last_progress = std::time::Instant::now();
            }
        } else {
            // Onderhoudsjob: wissen/formatteren met percentage.
            let (busy, kind) = match job.kind {
                JobKind::Erase => (DriveStatus::Erasing, MaintKind::Erase),
                _ => (DriveStatus::Formatting, MaintKind::Format),
            };
            if ds == busy && prog.sectors > 0 {
                let pct = ((prog.sector as f32 / prog.sectors as f32) * 100.0).min(100.0) as i32;
                if Some(pct) != job.last_pct {
                    notify.log(Level::Info, st.lang.w_maint_progress(kind, pct));
                    notify.send(Event::MaintProgress {
                        kind,
                        index: job.index,
                        pct: pct as f32,
                    });
                    job.last_pct = Some(pct);
                }
            }
        }
        false
    }
}

/// Werk één afgeronde job af: resultaat, opruimen, eject-reeks en events.
unsafe fn finalize_job(st: &WorkerState, job: &mut ActiveJob, notify: &Notifier) {
    unsafe {
        let well = (st.raw.drive_wrote_well)(job.drive) == 1;
        // Brandduur nu vastleggen: na de eject-reeks (met pauzes) zou de
        // gemiddelde snelheid anders te laag uitvallen.
        let burn_elapsed = job.started.elapsed().as_secs_f64();
        drain_msgs(&st.raw, notify);

        match &mut job.kind {
            JobKind::Write { .. } => {
                if let Some(wj) = job.write.take() {
                    (st.raw.write_opts_free)(wj.res.opts);
                    (st.raw.source_free)(wj.res.source);
                    (st.raw.track_free)(wj.res.track);
                    (st.raw.session_free)(wj.res.session);
                    (st.raw.disc_free)(wj.res.disc);
                    if !wj.res.iso_opts.is_null() {
                        if let Some(iso) = st.isofs.as_ref() {
                            (iso.write_opts_free)(wj.res.iso_opts);
                        }
                    }
                    if !wj.res.image.is_null() {
                        if let Some(iso) = st.isofs.as_ref() {
                            (iso.image_unref)(wj.res.image);
                        }
                    }
                    if !wj.res.data_src.is_null() {
                        if let Some(iso) = st.isofs.as_ref() {
                            (iso.data_source_unref)(wj.res.data_src);
                        }
                    }

                    // Eject-reeks (stap 8): een eject van een AANGEKOPPELDE
                    // schijf wordt door de kernel geweigerd (EBUSY) — eerst
                    // unmounten, dan opnieuw grabben voor het eject-verzoek.
                    if wj.s.eject_after {
                        (st.raw.drive_release)(job.drive, 0);
                        thread::sleep(Duration::from_millis(300));
                        if is_dev_mounted(&job.adr) {
                            notify.log(Level::Info, st.lang.w_eject_mounted(&job.adr));
                            try_unmount(&job.adr, st.lang, notify);
                            thread::sleep(Duration::from_millis(300));
                        }
                        // De drive/udisks kan na het unmounten even bezig
                        // zijn; een paar pogingen met pauze lost dat meestal.
                        let mut grabbed = false;
                        for poging in 1..=5 {
                            if (st.raw.drive_grab)(job.drive, 0) == 1 {
                                grabbed = true;
                                break;
                            }
                            notify.log(Level::Info, st.lang.w_regrab_failed(poging));
                            thread::sleep(Duration::from_secs(1));
                        }
                        if grabbed {
                            (st.raw.drive_release)(job.drive, 1);
                            notify.log(Level::Info, st.lang.w_eject_sent());
                            notify.send(Event::MediaEjected { index: job.index });
                        } else {
                            notify.log(Level::Warning, st.lang.w_eject_manual());
                        }
                    } else {
                        (st.raw.drive_release)(job.drive, 0);
                        notify.log(Level::Info, st.lang.w_drive_released());
                    }

                    if job.cancelled {
                        notify.log(Level::Warning, st.lang.w_burn_cancelled());
                        notify.send(Event::BurnCancelled { index: job.index });
                    } else if well {
                        notify.log(
                            Level::Success,
                            st.lang.w_burn_done(
                                job.index,
                                wj.s.simulate,
                                wj.s.multi_session == crate::settings::MultiSession::Yes,
                            ),
                        );
                        // Samenvatting (komt automatisch ook in het
                        // sessielogbestand): payload, duur, gemiddelde snelheid.
                        if job.last_sector > 0 && burn_elapsed >= 1.0 {
                            let kbps = (job.last_sector as f64 * 2048.0) / burn_elapsed / 1000.0;
                            let secs = burn_elapsed as u64;
                            let duur = if secs >= 3600 {
                                format!(
                                    "{}:{:02}:{:02}",
                                    secs / 3600,
                                    (secs % 3600) / 60,
                                    secs % 60
                                )
                            } else {
                                format!("{}:{:02}", secs / 60, secs % 60)
                            };
                            notify.log(
                                Level::Info,
                                st.lang.w_burn_summary(
                                    job.index,
                                    &format_blocks(job.last_sector),
                                    &duur,
                                    kbps,
                                ),
                            );
                        }
                        notify.send(Event::BurnDone { index: job.index });
                    } else {
                        let msg = st.lang.w_burn_failed(job.index);
                        notify.log(Level::Error, &msg);
                        notify.send(Event::BurnFailed {
                            index: job.index,
                            error: msg,
                        });
                    }
                }
            }
            JobKind::Erase => {
                (st.raw.drive_re_assess)(job.drive, 0);
                (st.raw.drive_release)(job.drive, 0);
                if job.cancelled {
                    notify.log(Level::Warning, st.lang.w_erase_cancelled());
                    notify.send(Event::MaintCancelled {
                        kind: MaintKind::Erase,
                        index: job.index,
                    });
                } else if well {
                    notify.log(Level::Success, st.lang.w_erased());
                    notify.send(Event::MaintDone {
                        kind: MaintKind::Erase,
                        index: job.index,
                    });
                } else {
                    let msg = st.lang.w_erase_failed();
                    notify.log(Level::Error, &msg);
                    notify.send(Event::MaintFailed {
                        kind: MaintKind::Erase,
                        index: job.index,
                        error: msg,
                    });
                }
            }
            JobKind::Format => {
                (st.raw.drive_re_assess)(job.drive, 0);
                (st.raw.drive_release)(job.drive, 0);
                if job.cancelled {
                    notify.log(Level::Warning, st.lang.w_restore_cancelled());
                    notify.send(Event::MaintCancelled {
                        kind: MaintKind::Format,
                        index: job.index,
                    });
                } else if well {
                    notify.log(Level::Success, st.lang.w_restore_done());
                    notify.send(Event::MaintDone {
                        kind: MaintKind::Format,
                        index: job.index,
                    });
                } else {
                    let msg = st.lang.w_restore_failed();
                    notify.log(Level::Error, &msg);
                    notify.send(Event::MaintFailed {
                        kind: MaintKind::Format,
                        index: job.index,
                        error: msg,
                    });
                }
            }
        }

        // Onderhoudsdiagnostiek weer uit (staat alleen aan tijdens
        // wis/format-jobs; voor brandjobs is de log te omvangrijk).
        if !matches!(&job.kind, JobKind::Write { .. }) {
            (st.raw.set_scsi_logging)(0);
        }
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
    lang: Lang,
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
            let msg = lang.w_model_create_failed();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        if (raw.disc_add_session)(disc, session, ffi::BURN_POS_END) != 1
            || (raw.session_add_track)(session, track, ffi::BURN_POS_END) != 1
        {
            let msg = lang.w_model_add_failed();
            notify.log(Level::Error, &msg);
            (raw.track_free)(track);
            (raw.session_free)(session);
            (raw.disc_free)(disc);
            return Err(msg);
        }

        // Bron: bestand → FIFO (4 MiB) → track.
        let file_source = (raw.file_source_new)(path_c, std::ptr::null());
        if file_source.is_null() {
            let msg = lang.w_file_source_failed();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        let fifo = (raw.fifo_source_new)(file_source, 2048, BURN_FIFO_CHUNKS, 0);
        (raw.source_free)(file_source); // fifo heeft een eigen referentie
        if fifo.is_null() {
            let msg = lang.w_fifo_source_failed();
            notify.log(Level::Error, &msg);
            return Err(msg);
        }
        if (raw.track_set_source)(track, fifo) != 0 {
            let msg = lang.w_source_attach_failed();
            notify.log(Level::Error, &msg);
            (raw.source_free)(fifo);
            return Err(msg);
        }
        if (raw.track_set_size)(track, file_size as c_longlong) != 1 {
            let msg = lang.w_track_size_failed();
            notify.log(Level::Error, &msg);
            (raw.source_free)(fifo);
            return Err(msg);
        }
        // Datatrack: geen offset, padding als staart-nullen, laatste sector vullen.
        (raw.track_define_data)(track, 0, s.padding_kib * 1024, 1, ffi::BURN_MODE1);

        // Write-opts + schrijfmodus (gedeelde helper).
        let opts = make_write_opts(raw, drive, disc, s, lang, notify)?;

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
    lang: Lang,
) -> Option<String> {
    unsafe {
        let mut reasons = [0 as c_char; ffi::BURN_REASONS_LEN];
        let ret = (raw.precheck_write)(opts, disc, reasons.as_mut_ptr(), 1);
        if ret > 0 {
            return None;
        }
        let reasons = cbuf_to_string(&reasons);
        let mut msg = lang.w_precheck_failed(&reasons);
        if let Some(hint) = simulation_hint(s, &reasons, lang) {
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
fn simulation_hint(s: &BurnSettings, reasons: &str, lang: Lang) -> Option<String> {
    if !s.simulate {
        return None;
    }
    let lower = reasons.to_ascii_lowercase();
    if lower.contains("simulation") || lower.contains("simul") {
        Some(lang.w_simulation_hint())
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

/// Zoekt het apparaat in /proc/mounts (true = er is een bestandssysteem op
/// aangekoppeld; een eject van een aangekoppeld device wordt door de kernel
/// geweigerd met EBUSY).
fn is_dev_mounted(dev: &str) -> bool {
    if dev.is_empty() {
        return false;
    }
    std::fs::read_to_string("/proc/mounts")
        .map(|content| {
            content.lines().any(|l| {
                l.split_whitespace()
                    .next()
                    .map(|d| d == dev)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Probeert de schijf te unmounten via udisksctl (aanwezig op elk
/// desktopsysteem met automount).
fn try_unmount(dev: &str, lang: Lang, notify: &Notifier) {
    match std::process::Command::new("udisksctl")
        .args(["unmount", "-b", dev])
        .output()
    {
        Ok(out) if out.status.success() => {
            notify.log(Level::Info, lang.w_unmounted(dev));
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            notify.log(Level::Warning, lang.w_unmount_failed(dev, &err));
        }
        Err(e) => {
            notify.log(Level::Warning, lang.w_unmount_unavailable(&e.to_string()));
        }
    }
}

/// Wis de media los van een brandjob (stap 6).
fn erase_job(
    state: &Option<WorkerState>,
    lang: Lang,
    index: usize,
    fast: bool,
    active: &mut Vec<ActiveJob>,
    notify: &Notifier,
) {
    let Some(st) = state else {
        notify.log(Level::Warning, lang.w_erase_no_lib());
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(Level::Warning, lang.w_erase_unknown(index));
        return;
    }
    let di = unsafe { *st.infos.add(index) };
    notify.send(Event::MaintStarted {
        kind: MaintKind::Erase,
        index,
    });
    notify.log(Level::Info, lang.w_erase_starting(index, fast));

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = st.lang.w_grab_failed(index, &adr);
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
        let msg = st.lang.w_erase_overwritable(profile_no);
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
        let msg = st.lang.w_erase_not_rewritable();
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
    let adr = unsafe { adr_of(&di, &st.raw) };
    active.push(ActiveJob {
        index,
        adr,
        drive: di.drive,
        kind: JobKind::Erase,
        write: None,
        started: std::time::Instant::now(),
        last_progress: std::time::Instant::now(),
        last_pct: Some(i32::MIN),
        last_sector: 0,
        cancelled: false,
    });
}

/// Profielen waarop formatteren zinvol heeft (DVD-RW seq → RO, DVD-RW RO,
/// DVD+RW, DVD-RAM, BD-RE).
pub fn profile_is_formattable(pno: i32) -> bool {
    matches!(pno, 0x12 | 0x13 | 0x14 | 0x1A | 0x43)
}

/// Naam van een MMC-format-type (mmc5r03c 6.5.4.2).
fn format_type_name(ty: c_int) -> &'static str {
    match ty {
        0x00 => "volledig",
        0x01 => "spare-uitbreiding",
        0x10 => "DVD-RW volledig",
        0x13 => "DVD-RW groei",
        0x15 => "DVD-RW snel",
        0x26 => "DVD+RW",
        0x30 => "BD-RE met DM",
        0x31 => "BD-RE zonder DM",
        0x32 => "BD-R SRM",
        _ => "?",
    }
}

/// Logt de format-capaciteiten van de ingelegde media (diagnostiek) en geeft
/// de aangeboden format-types terug.
fn format_descriptor_types(st: &WorkerState, di: &ffi::DriveInfo, notify: &Notifier) -> Vec<c_int> {
    unsafe {
        let mut status: c_int = 0;
        let mut size: c_longlong = 0;
        let mut bl_sas: c_uint = 0;
        let mut num: c_int = 0;
        let ok =
            (st.raw.disc_get_formats)(di.drive, &mut status, &mut size, &mut bl_sas, &mut num) == 1;
        if !ok {
            notify.log(
                Level::Info,
                "Format capabilities: drive reports no readable format info",
            );
            return Vec::new();
        }
        let status_txt = match status {
            1 => "unformatted",
            2 => "formatted",
            3 => "unknown format status",
            _ => "?",
        };
        let mut types = Vec::new();
        let mut list = String::new();
        for i in 0..num {
            let mut ty: c_int = 0;
            let mut dsz: c_longlong = 0;
            let mut tdp: c_uint = 0;
            if (st.raw.disc_get_format_descr)(di.drive, i, &mut ty, &mut dsz, &mut tdp) == 1 {
                types.push(ty);
                if !list.is_empty() {
                    list.push_str(", ");
                }
                list.push_str(&format!(
                    "0x{:02X} {} ({})",
                    ty,
                    format_type_name(ty),
                    format_blocks((dsz / 2048) as i32)
                ));
            }
        }
        let cur = if size > 0 {
            format!(" ({})", format_blocks((size / 2048) as i32))
        } else {
            String::new()
        };
        notify.log(
            Level::Info,
            format!(
                "Format capabilities: {}{}, offered: {}",
                status_txt,
                cur,
                if list.is_empty() {
                    "none".to_string()
                } else {
                    list
                }
            ),
        );
        types
    }
}

/// Formatteer de media los van een brandjob (stap 6).
fn format_job(
    state: &Option<WorkerState>,
    lang: Lang,
    index: usize,
    s: &BurnSettings,
    active: &mut Vec<ActiveJob>,
    notify: &Notifier,
) {
    let Some(st) = state else {
        notify.log(Level::Warning, lang.w_restore_no_lib());
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(Level::Warning, lang.w_restore_unknown(index));
        return;
    }
    let di = unsafe { *st.infos.add(index) };
    notify.send(Event::MaintStarted {
        kind: MaintKind::Format,
        index,
    });
    notify.log(Level::Info, lang.w_restore_default_size());

    // Diagnostiek: SCSI-commandolog voor deze job — elk commando + sense
    // komt in /tmp/libburn_sg_command_log. Onmisbaar bij format-problemen,
    // want niet elk faalpad van libburn levert een zichtbare melding.
    // (Randgeval: bij parallelle onderhoudsjobs zet de eerst afgeronde job
    // de log al uit; dan herstart de volgende job hem bij de volgende grab
    // niet meer — acceptabel voor diagnostiek.)
    unsafe { (st.raw.set_scsi_logging)(1 | 4) };
    // Taalneutraal: pure diagnostiek.
    notify.log(
        Level::Info,
        "SCSI command log on: /tmp/libburn_sg_command_log",
    );

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = st.lang.w_grab_failed(index, &adr);
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
        let msg = lang.w_restore_not_applicable(profile_no);
        notify.log(Level::Error, &msg);
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        notify.send(Event::MaintFailed {
            kind: MaintKind::Format,
            index,
            error: msg,
        });
        return;
    }

    // Diagnostiek: welke format-types biedt de drive voor deze media aan?
    let fmt_types = format_descriptor_types(st, &di, notify);
    if s.disable_dm_on_format && profile_no == 0x43 && !fmt_types.contains(&0x31) {
        notify.log(Level::Warning, lang.w_restore_no_dm_type());
    }

    // Volledige format met enforce-re-format (bit4): bij DVD+RW/BD-RE/DVD-RAM
    // ziet libburn een al geformatteerde schijf anders als no-op
    // ("FORMAT UNIT ignored. Already completed."). Bit4 forceert de "de-ice"
    // — de bestaande data wordt gewist. size-mode 3 (bit1+2) = standaardgrootte.
    // Bit5 (optioneel, instelling): defect management proberen uit te
    // schakelen — sneller branden, maar geen hermapping van slechte blokken.
    let mut fmt_flag = (3 << 1) | (1 << 4);
    if s.disable_dm_on_format {
        fmt_flag |= 1 << 5;
        notify.log(Level::Info, lang.w_restore_dm_off());
    }
    if s.format_skip_certification {
        // Bit6: libburn kiest dan format-type 0x00 zonder certificatie —
        // de omweg waarmee dvd+rw-format vergelijkbare drives wél laat
        // formatteren.
        fmt_flag |= 1 << 6;
        notify.log(Level::Info, lang.w_restore_cert_skip());
    }
    notify.log(Level::Info, lang.w_restore_running());
    unsafe { (st.raw.disc_format)(di.drive, 0, fmt_flag) };
    let adr = unsafe { adr_of(&di, &st.raw) };
    active.push(ActiveJob {
        index,
        adr,
        drive: di.drive,
        kind: JobKind::Format,
        write: None,
        started: std::time::Instant::now(),
        last_progress: std::time::Instant::now(),
        last_pct: Some(i32::MIN),
        last_sector: 0,
        cancelled: false,
    });
}

/// Maakt een schijfkopie van de datamedia naar een bestand.
///
/// Gebruikt `burn_read_data` (random access, 2048-byte blokken): geschikt voor
/// CD/DVD/BD-datamedia, niet voor CD-audio. De drive wordt voor de duur van de
/// kopie gegrabbed en daarna weer vrijgegeven. Annuleren kan tussentijds.
fn read_disc(
    state: &Option<WorkerState>,
    lang: Lang,
    index: usize,
    path: &str,
    cmds: &Receiver<Command>,
    pending: &mut VecDeque<Command>,
    notify: &Notifier,
) {
    use std::io::Write;

    let Some(st) = state else {
        notify.log(Level::Warning, lang.w_read_no_lib());
        return;
    };
    if st.infos.is_null() || index >= st.n_drives {
        notify.log(Level::Warning, lang.w_read_unknown(index));
        return;
    }
    if path.is_empty() {
        notify.log(Level::Warning, lang.w_read_no_path());
        return;
    }

    let di = unsafe { *st.infos.add(index) };
    notify.log(Level::Info, lang.w_read_start(index, path));

    // Niet overschrijven: kies een andere naam.
    if std::path::Path::new(path).exists() {
        let msg = lang.w_read_exists(path);
        notify.log(Level::Error, format!("{}: {msg}", lang.w_station(index)));
        notify.send(Event::ReadFailed { index, error: msg });
        return;
    }

    let grabbed = unsafe { (st.raw.drive_grab)(di.drive, 0) } == 1;
    if !grabbed {
        let adr = unsafe { adr_of(&di, &st.raw) };
        let msg = st.lang.w_grab_failed(index, &adr);
        notify.log(Level::Error, msg.clone());
        notify.send(Event::ReadFailed { index, error: msg });
        return;
    }

    // Capaciteit bepalen (blokken van 2048 bytes).
    let mut capacity: c_int = 0;
    if unsafe { (st.raw.get_read_capacity)(di.drive, &mut capacity, 0) } != 1 || capacity <= 0 {
        unsafe { (st.raw.drive_release)(di.drive, 0) };
        let msg = lang.w_read_no_capacity();
        notify.log(Level::Error, format!("{}: {msg}", lang.w_station(index)));
        notify.send(Event::ReadFailed { index, error: msg });
        return;
    }
    let total_blocks = capacity as i64;
    notify.log(
        Level::Info,
        lang.w_read_blocks(index, total_blocks, &format_blocks(capacity)),
    );
    notify.send(Event::ReadStarted {
        index,
        total_blocks: capacity,
    });

    let mut file = match std::fs::File::create_new(path) {
        Ok(f) => f,
        Err(e) => {
            unsafe { (st.raw.drive_release)(di.drive, 0) };
            let msg = lang.w_read_create_failed(path, &e.to_string());
            notify.log(Level::Error, format!("{}: {msg}", lang.w_station(index)));
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
            lang.w_read_cancelled(index, blocks_done, path),
        );
        notify.send(Event::ReadCancelled);
    } else if let Some(err) = error {
        notify.log(Level::Error, lang.w_read_error(index, &err));
        notify.send(Event::ReadFailed { index, error: err });
    } else {
        notify.log(
            Level::Success,
            lang.w_read_done(index, blocks_done, &format_blocks(blocks_done as i32), path),
        );
        notify.send(Event::ReadDone);
    }
}

fn shutdown_lib(state: Option<WorkerState>, lang: Lang, notify: &Notifier) {
    let Some(mut st) = state else { return };
    free_drive_list(&mut st);
    unsafe { (st.raw.finish)() };
    notify.log(Level::Info, lang.w_lib_finished());
    if let Some(iso) = st.isofs {
        unsafe { (iso.finish)() };
        notify.log(Level::Info, lang.w_isofs_finished());
    }
}

/// Laadt libisofs van het systeem (non-fataal: zonder libisofs werkt alles
/// behalve “bestanden samenstellen”).
fn load_isofs(lang: Lang, notify: &Notifier) -> Option<RawLibisofs> {
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
                    last_err = lang.w_isofs_init_failed(cand);
                    continue;
                }
                (iso.set_msgs_severities)(
                    c"UPDATE".as_ptr(),
                    c"NEVER".as_ptr(),
                    c"libisofs_gui: ".as_ptr(),
                );
                let (mut maj, mut min, mut mic) = (0, 0, 0);
                (iso.version)(&mut maj, &mut min, &mut mic);
                let version = format!("{maj}.{min}.{mic}");
                notify.log(Level::Success, lang.w_isofs_loaded(&version, cand));
                return Some(iso);
            },
            Err(e) => last_err = format!("`{cand}`: {e}"),
        }
    }
    notify.log(Level::Warning, lang.w_isofs_unavailable_warn(&last_err));
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
        0x11 => "DVD-R sequential",
        0x12 => "DVD-RAM",
        0x13 => "DVD-RW restricted overwrite",
        0x14 => "DVD-RW sequential",
        0x15 => "DVD-R DL sequential",
        0x16 => "DVD-R DL layer jump",
        0x1A => "DVD+RW",
        0x1B => "DVD+R",
        0x2B => "DVD+R DL",
        0x40 => "BD-ROM",
        0x41 => "BD-R random recording",
        0x42 => "BD-R sequential",
        0x43 => "BD-RE",
        0xFFFF => "stdio file",
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

/// Profielen waarop simuleren niet mogelijk is (MMC: geen test write).
/// Bron van een snelheidsdescriptor (zie libburn.h) — UI-labels staan in de
/// i18n-catalogus (MediaTexts::speed_source_label).
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
                        eprintln!("  - {} @ {}", d.display_name("Station"), d.adr);
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
