//! Ruwe FFI-laag voor libburn.
//!
//! libburn wordt **niet meegecompileerd**: tijdens runtime wordt het
//! shared object van het systeem geladen (`libburn.so.4`, of het pad uit de
//! omgevingsvariabele `LIBBURN_SO`, of een pad dat de gebruiker opgeeft).
//! Zo bouwt en start deze GUI ook op systemen waar libburn (nog) niet
//! geïnstalleerd is, met duidelijke feedback in de interface.
//!
//! De types hieronder zijn 1-op-1 overgenomen uit `docs/libburn.h`
//! (libburn 1.5.6) en moeten exact overeenkomen met de geïnstalleerde
//! library (SONAME `libburn.so.4`).

use std::ffi::{c_char, c_int, c_longlong, c_uint, c_void};

use libloading::Library;

// libc free() — libburn geeft strings terug (burn_disc_get_media_id,
// burn_guess_manufacturer) die met free() moeten worden vrijgegeven.
unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

/// Neemt eigendom van een door libburn toegewezen C-string en zet die om
/// naar een Rust-String (de C-string wordt netjes vrijgegeven).
pub(crate) unsafe fn take_cstring(p: *mut c_char) -> Option<String> {
    unsafe {
        if p.is_null() {
            return None;
        }
        let s = std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned();
        free(p as *mut c_void);
        Some(s)
    }
}

/// Geeft een door libburn toegewezen C-string weer vrij.
pub(crate) unsafe fn free_c(p: *mut c_char) {
    unsafe { free(p as *mut c_void) };
}

/// Maximale lengte van een drive-adres (`BURN_DRIVE_ADR_LEN`).
pub const BURN_DRIVE_ADR_LEN: usize = 1024;

/// Opaque handle naar `struct burn_drive`.
#[repr(C)]
pub struct BurnDrive {
    _private: [u8; 0],
}

/// Opaque handle naar `struct burn_disc` (TOC-model van de ingelegde schijf).
#[repr(C)]
pub struct BurnDisc {
    _private: [u8; 0],
}

/// Opaque handle naar `struct burn_session`.
#[repr(C)]
pub struct BurnSession {
    _private: [u8; 0],
}

/// Opaque handle naar `struct burn_track`.
#[repr(C)]
pub struct BurnTrack {
    _private: [u8; 0],
}

/// Opaque handle naar `struct burn_source` (databron voor een track).
#[repr(C)]
pub struct BurnSource {
    _private: [u8; 0],
}

/// Opaque handle naar `struct burn_write_opts`.
#[repr(C)]
pub struct BurnWriteOpts {
    _private: [u8; 0],
}

/// `BURN_POS_END` — positiecode voor "achteraan toevoegen".
pub const BURN_POS_END: c_uint = 100;
/// `BURN_MODE1` — datatrack met 2048 bytes gebruikersdata per sector.
pub const BURN_MODE1: c_int = 1 << 2;
/// `BURN_BLOCK_MODE1` — bloktype bij schrijfmodus TAO voor datatracks.
pub const BURN_BLOCK_MODE1: c_int = 256;
/// `BURN_BLOCK_SAO` — bloktype bij schrijfmodus SAO.
pub const BURN_BLOCK_SAO: c_int = 16384;
/// `BURN_WRITE_TAO` / `BURN_WRITE_SAO` / `BURN_WRITE_NONE` (enum burn_write_types).
pub const BURN_WRITE_TAO: c_int = 1;
pub const BURN_WRITE_SAO: c_int = 2;
pub const BURN_WRITE_RAW: c_int = 3;
pub const BURN_WRITE_NONE: c_int = 4;
/// `BURN_MSGS_MESSAGE_LEN` — minimale buffer voor `burn_msgs_obtain`.
pub const BURN_MSGS_MESSAGE_LEN: usize = 4096;
/// `BURN_REASONS_LEN` — buffer voor afwijzingsredenen van de precheck.
pub const BURN_REASONS_LEN: usize = 4096;

/// `struct burn_drive_info` uit libburn.h.
///
/// De 11 capability-bitvelden uit de C-header worden door GCC (little-endian,
/// x86_64/aarch64) in één `unsigned int`-eenheid gepakt. Hier staat die eenheid
/// als `flags`; bit 0 is het eerst gedeclareerde veld (`read_dvdram`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DriveInfo {
    pub vendor: [c_char; 9],
    pub product: [c_char; 17],
    pub revision: [c_char; 5],
    /// Vervangen door `burn_drive_get_adr()`; bevat geen bruikbare data meer.
    pub location: [c_char; 17],
    pub flags: c_uint,
    pub buffer_size: c_int,
    pub tao_block_types: c_int,
    pub sao_block_types: c_int,
    pub raw_block_types: c_int,
    pub packet_block_types: c_int,
    pub drive: *mut BurnDrive,
}

/// `struct burn_toc_entry` uit libburn.h (1.5.6).
///
/// 15 `unsigned char`-velden gevolgd door vier `int`-velden; de compiler
/// plakt 1 byte padding vóór `start_lba` — `repr(C)` doet in Rust hetzelfde.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TocEntry {
    pub session: u8,
    pub adr: u8,
    /// bit2 = datatrack, bit1 = kopie toegestaan
    pub control: u8,
    pub tno: u8,
    /// Tracknummer (0xA2 = lead-out)
    pub point: u8,
    pub min: u8,
    pub sec: u8,
    pub frame: u8,
    pub zero: u8,
    pub pmin: u8,
    pub psec: u8,
    pub pframe: u8,
    /// bit0 = DVD-extensie geldig, bit1 = LRA geldig, bit2 = statusbits geldig
    pub extensions_valid: u8,
    pub session_msb: u8,
    pub point_msb: u8,
    pub start_lba: c_int,
    pub track_blocks: c_int,
    pub last_recorded_address: c_int,
    pub track_status_bits: c_int,
}

/// `struct burn_progress` uit libburn.h.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Progress {
    pub sessions: c_int,
    pub session: c_int,
    pub tracks: c_int,
    pub track: c_int,
    pub indices: c_int,
    pub index: c_int,
    pub start_sector: c_int,
    pub sectors: c_int,
    pub sector: c_int,
    pub buffer_capacity: c_uint,
    pub buffer_available: c_uint,
    pub buffered_bytes: c_longlong,
    pub buffer_min_fill: c_uint,
}

/// `struct burn_speed_descriptor` uit libburn.h (geketende lijst).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpeedDescriptor {
    pub source: c_int,
    pub profile_loaded: c_int,
    pub profile_name: [c_char; 80],
    pub end_lba: c_int,
    pub write_speed: c_int,
    pub read_speed: c_int,
    pub wrc: c_int,
    pub exact: c_int,
    pub mrw: c_int,
    pub prev: *mut SpeedDescriptor,
    pub next: *mut SpeedDescriptor,
}

/// `enum burn_disc_status`
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiscStatus {
    Unready,
    Blank,
    Empty,
    Appendable,
    Full,
    Ungrabbed,
    Unsuitable,
    Unknown(c_int),
}

impl DiscStatus {
    pub fn from_raw(v: c_int) -> Self {
        match v {
            0 => Self::Unready,
            1 => Self::Blank,
            2 => Self::Empty,
            3 => Self::Appendable,
            4 => Self::Full,
            5 => Self::Ungrabbed,
            6 => Self::Unsuitable,
            other => Self::Unknown(other),
        }
    }
}

/// `enum burn_drive_status`
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriveStatus {
    Idle,
    Spawning,
    Reading,
    Writing,
    WritingLeadin,
    WritingLeadout,
    Erasing,
    Grabbing,
    WritingPregap,
    ClosingTrack,
    ClosingSession,
    Formatting,
    ReadingSync,
    WritingSync,
    Other(c_int),
}

impl DriveStatus {
    pub fn from_raw(v: c_int) -> Self {
        match v {
            0 => Self::Idle,
            1 => Self::Spawning,
            2 => Self::Reading,
            3 => Self::Writing,
            4 => Self::WritingLeadin,
            5 => Self::WritingLeadout,
            6 => Self::Erasing,
            7 => Self::Grabbing,
            8 => Self::WritingPregap,
            9 => Self::ClosingTrack,
            10 => Self::ClosingSession,
            11 => Self::Formatting,
            12 => Self::ReadingSync,
            13 => Self::WritingSync,
            other => Self::Other(other),
        }
    }
}

/// Alle benodigde functiewijzers, geresolved uit het geladen shared object.
pub struct RawLibburn {
    _lib: Library,
    pub initialize: unsafe extern "C" fn() -> c_int,
    pub finish: unsafe extern "C" fn(),
    pub version: unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int),
    pub drive_scan: unsafe extern "C" fn(*mut *mut DriveInfo, *mut c_uint) -> c_int,
    pub drive_info_free: unsafe extern "C" fn(*mut DriveInfo),
    pub drive_get_adr: unsafe extern "C" fn(*mut DriveInfo, *mut c_char) -> c_int,
    pub drive_get_all_profiles:
        unsafe extern "C" fn(*mut BurnDrive, *mut c_int, *mut c_int, *mut c_char) -> c_int,
    pub drive_grab: unsafe extern "C" fn(*mut BurnDrive, c_int) -> c_int,
    pub drive_release: unsafe extern "C" fn(*mut BurnDrive, c_int),
    pub disc_get_status: unsafe extern "C" fn(*mut BurnDrive) -> c_int,
    pub drive_get_status: unsafe extern "C" fn(*mut BurnDrive, *mut Progress) -> c_int,
    pub drive_get_speedlist:
        unsafe extern "C" fn(*mut BurnDrive, *mut *mut SpeedDescriptor) -> c_int,
    pub drive_free_speedlist: unsafe extern "C" fn(*mut *mut SpeedDescriptor) -> c_int,
    // Stap 2: profiel, capaciteit, TOC.
    pub disc_get_profile: unsafe extern "C" fn(*mut BurnDrive, *mut c_int, *mut c_char) -> c_int,
    pub get_read_capacity: unsafe extern "C" fn(*mut BurnDrive, *mut c_int, c_int) -> c_int,
    pub disc_erasable: unsafe extern "C" fn(*mut BurnDrive) -> c_int,
    pub drive_get_disc: unsafe extern "C" fn(*mut BurnDrive) -> *mut BurnDisc,
    pub disc_get_sessions: unsafe extern "C" fn(*mut BurnDisc, *mut c_int) -> *mut *mut BurnSession,
    pub session_get_tracks:
        unsafe extern "C" fn(*mut BurnSession, *mut c_int) -> *mut *mut BurnTrack,
    pub track_get_entry: unsafe extern "C" fn(*mut BurnTrack, *mut TocEntry),
    pub session_get_leadout_entry: unsafe extern "C" fn(*mut BurnSession, *mut TocEntry),
    pub disc_get_incomplete_sessions: unsafe extern "C" fn(*mut BurnDisc) -> c_int,
    pub disc_free: unsafe extern "C" fn(*mut BurnDisc),
    // Stap 3: apparaat-openingsbeleid en random-access lezen.
    pub preset_device_open: unsafe extern "C" fn(c_int, c_int, c_int),
    pub read_data: unsafe extern "C" fn(
        *mut BurnDrive,
        c_longlong,
        *mut c_char,
        c_longlong,
        *mut c_longlong,
        c_int,
    ) -> c_int,
    // Stap 4: disc-model, bronnen, write-opts, branden/wissen en meldingen.
    pub disc_create: unsafe extern "C" fn() -> *mut BurnDisc,
    pub session_create: unsafe extern "C" fn() -> *mut BurnSession,
    pub session_free: unsafe extern "C" fn(*mut BurnSession),
    pub disc_add_session: unsafe extern "C" fn(*mut BurnDisc, *mut BurnSession, c_uint) -> c_int,
    pub track_create: unsafe extern "C" fn() -> *mut BurnTrack,
    pub track_free: unsafe extern "C" fn(*mut BurnTrack),
    pub session_add_track: unsafe extern "C" fn(*mut BurnSession, *mut BurnTrack, c_uint) -> c_int,
    pub track_set_source: unsafe extern "C" fn(*mut BurnTrack, *mut BurnSource) -> c_int,
    pub source_free: unsafe extern "C" fn(*mut BurnSource),
    pub file_source_new: unsafe extern "C" fn(*const c_char, *const c_char) -> *mut BurnSource,
    pub fifo_source_new:
        unsafe extern "C" fn(*mut BurnSource, c_int, c_int, c_int) -> *mut BurnSource,
    pub fifo_inquire_status:
        unsafe extern "C" fn(*mut BurnSource, *mut c_int, *mut c_int, *mut *const c_char) -> c_int,
    pub track_set_size: unsafe extern "C" fn(*mut BurnTrack, c_longlong) -> c_int,
    pub track_define_data: unsafe extern "C" fn(*mut BurnTrack, c_int, c_int, c_int, c_int),
    pub write_opts_new: unsafe extern "C" fn(*mut BurnDrive) -> *mut BurnWriteOpts,
    pub write_opts_free: unsafe extern "C" fn(*mut BurnWriteOpts),
    pub write_opts_set_perform_opc: unsafe extern "C" fn(*mut BurnWriteOpts, c_int),
    pub write_opts_set_simulate: unsafe extern "C" fn(*mut BurnWriteOpts, c_int) -> c_int,
    pub write_opts_set_multi: unsafe extern "C" fn(*mut BurnWriteOpts, c_int),
    pub write_opts_set_underrun_proof: unsafe extern "C" fn(*mut BurnWriteOpts, c_int),
    pub write_opts_set_force: unsafe extern "C" fn(*mut BurnWriteOpts, c_int),
    pub write_opts_set_write_type: unsafe extern "C" fn(*mut BurnWriteOpts, c_int, c_int) -> c_int,
    pub write_opts_auto_write_type:
        unsafe extern "C" fn(*mut BurnWriteOpts, *mut BurnDisc, *mut c_char, c_int) -> c_int,
    pub precheck_write:
        unsafe extern "C" fn(*mut BurnWriteOpts, *mut BurnDisc, *mut c_char, c_int) -> c_int,
    pub disc_write: unsafe extern "C" fn(*mut BurnWriteOpts, *mut BurnDisc),
    pub disc_erase: unsafe extern "C" fn(*mut BurnDrive, c_int),
    pub disc_format: unsafe extern "C" fn(*mut BurnDrive, c_longlong, c_int),
    pub drive_cancel: unsafe extern "C" fn(*mut BurnDrive),
    pub drive_wrote_well: unsafe extern "C" fn(*mut BurnDrive) -> c_int,
    pub drive_re_assess: unsafe extern "C" fn(*mut BurnDrive, c_int) -> c_int,
    pub disc_get_bd_spare_info:
        unsafe extern "C" fn(*mut BurnDrive, *mut c_int, *mut c_int, c_int) -> c_int,
    pub disc_get_media_id: unsafe extern "C" fn(
        *mut BurnDrive,
        *mut *mut c_char,
        *mut *mut c_char,
        *mut *mut c_char,
        *mut *mut c_char,
        c_int,
    ) -> c_int,
    pub guess_manufacturer:
        unsafe extern "C" fn(c_int, *mut c_char, *mut c_char, c_int) -> *mut c_char,
    pub disc_available_space:
        unsafe extern "C" fn(*mut BurnDrive, *mut BurnWriteOpts) -> c_longlong,
    pub msgs_set_severities:
        unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> c_int,
    pub msgs_obtain: unsafe extern "C" fn(
        *mut c_char,
        *mut c_int,
        *mut c_char,
        *mut c_int,
        *mut c_char,
    ) -> c_int,
}

macro_rules! resolve {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let sym: libloading::Symbol<$ty> = $lib
            .get::<$ty>(concat!($name, "\0").as_bytes())
            .map_err(|e| format!("symbool `{}` ontbreekt: {e}", $name))?;
        *sym
    }};
}

impl RawLibburn {
    /// Laadt een libburn-shared object en resolveert alle benodigde symbolen.
    pub fn load(path: &str) -> Result<Self, String> {
        let lib =
            unsafe { Library::new(path) }.map_err(|e| format!("kan `{path}` niet openen: {e}"))?;
        unsafe {
            // Eerst alle symbolen resolve (met `lib` nog in leven), daarna pas
            // de library in de struct plaatsen.
            let initialize = resolve!(lib, "burn_initialize", unsafe extern "C" fn() -> c_int);
            let finish = resolve!(lib, "burn_finish", unsafe extern "C" fn());
            let version = resolve!(
                lib,
                "burn_version",
                unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int)
            );
            let drive_scan = resolve!(
                lib,
                "burn_drive_scan",
                unsafe extern "C" fn(*mut *mut DriveInfo, *mut c_uint) -> c_int
            );
            let drive_info_free = resolve!(
                lib,
                "burn_drive_info_free",
                unsafe extern "C" fn(*mut DriveInfo)
            );
            let drive_get_adr = resolve!(
                lib,
                "burn_drive_get_adr",
                unsafe extern "C" fn(*mut DriveInfo, *mut c_char) -> c_int
            );
            let drive_get_all_profiles = resolve!(
                lib,
                "burn_drive_get_all_profiles",
                unsafe extern "C" fn(*mut BurnDrive, *mut c_int, *mut c_int, *mut c_char) -> c_int
            );
            let drive_grab = resolve!(
                lib,
                "burn_drive_grab",
                unsafe extern "C" fn(*mut BurnDrive, c_int) -> c_int
            );
            let drive_release = resolve!(
                lib,
                "burn_drive_release",
                unsafe extern "C" fn(*mut BurnDrive, c_int)
            );
            let disc_get_status = resolve!(
                lib,
                "burn_disc_get_status",
                unsafe extern "C" fn(*mut BurnDrive) -> c_int
            );
            let drive_get_status = resolve!(
                lib,
                "burn_drive_get_status",
                unsafe extern "C" fn(*mut BurnDrive, *mut Progress) -> c_int
            );
            let drive_get_speedlist = resolve!(
                lib,
                "burn_drive_get_speedlist",
                unsafe extern "C" fn(*mut BurnDrive, *mut *mut SpeedDescriptor) -> c_int
            );
            let drive_free_speedlist = resolve!(
                lib,
                "burn_drive_free_speedlist",
                unsafe extern "C" fn(*mut *mut SpeedDescriptor) -> c_int
            );
            let disc_get_profile = resolve!(
                lib,
                "burn_disc_get_profile",
                unsafe extern "C" fn(*mut BurnDrive, *mut c_int, *mut c_char) -> c_int
            );
            let get_read_capacity = resolve!(
                lib,
                "burn_get_read_capacity",
                unsafe extern "C" fn(*mut BurnDrive, *mut c_int, c_int) -> c_int
            );
            let disc_erasable = resolve!(
                lib,
                "burn_disc_erasable",
                unsafe extern "C" fn(*mut BurnDrive) -> c_int
            );
            let drive_get_disc = resolve!(
                lib,
                "burn_drive_get_disc",
                unsafe extern "C" fn(*mut BurnDrive) -> *mut BurnDisc
            );
            let disc_get_sessions = resolve!(
                lib,
                "burn_disc_get_sessions",
                unsafe extern "C" fn(*mut BurnDisc, *mut c_int) -> *mut *mut BurnSession
            );
            let session_get_tracks = resolve!(
                lib,
                "burn_session_get_tracks",
                unsafe extern "C" fn(*mut BurnSession, *mut c_int) -> *mut *mut BurnTrack
            );
            let track_get_entry = resolve!(
                lib,
                "burn_track_get_entry",
                unsafe extern "C" fn(*mut BurnTrack, *mut TocEntry)
            );
            let session_get_leadout_entry = resolve!(
                lib,
                "burn_session_get_leadout_entry",
                unsafe extern "C" fn(*mut BurnSession, *mut TocEntry)
            );
            let disc_get_incomplete_sessions = resolve!(
                lib,
                "burn_disc_get_incomplete_sessions",
                unsafe extern "C" fn(*mut BurnDisc) -> c_int
            );
            let disc_free = resolve!(lib, "burn_disc_free", unsafe extern "C" fn(*mut BurnDisc));
            let preset_device_open = resolve!(
                lib,
                "burn_preset_device_open",
                unsafe extern "C" fn(c_int, c_int, c_int)
            );
            let read_data = resolve!(
                lib,
                "burn_read_data",
                unsafe extern "C" fn(
                    *mut BurnDrive,
                    c_longlong,
                    *mut c_char,
                    c_longlong,
                    *mut c_longlong,
                    c_int,
                ) -> c_int
            );
            let disc_create = resolve!(
                lib,
                "burn_disc_create",
                unsafe extern "C" fn() -> *mut BurnDisc
            );
            let session_create = resolve!(
                lib,
                "burn_session_create",
                unsafe extern "C" fn() -> *mut BurnSession
            );
            let session_free = resolve!(
                lib,
                "burn_session_free",
                unsafe extern "C" fn(*mut BurnSession)
            );
            let disc_add_session = resolve!(
                lib,
                "burn_disc_add_session",
                unsafe extern "C" fn(*mut BurnDisc, *mut BurnSession, c_uint) -> c_int
            );
            let track_create = resolve!(
                lib,
                "burn_track_create",
                unsafe extern "C" fn() -> *mut BurnTrack
            );
            let track_free = resolve!(lib, "burn_track_free", unsafe extern "C" fn(*mut BurnTrack));
            let session_add_track = resolve!(
                lib,
                "burn_session_add_track",
                unsafe extern "C" fn(*mut BurnSession, *mut BurnTrack, c_uint) -> c_int
            );
            let track_set_source = resolve!(
                lib,
                "burn_track_set_source",
                unsafe extern "C" fn(*mut BurnTrack, *mut BurnSource) -> c_int
            );
            let source_free = resolve!(
                lib,
                "burn_source_free",
                unsafe extern "C" fn(*mut BurnSource)
            );
            let file_source_new = resolve!(
                lib,
                "burn_file_source_new",
                unsafe extern "C" fn(*const c_char, *const c_char) -> *mut BurnSource
            );
            let fifo_source_new = resolve!(
                lib,
                "burn_fifo_source_new",
                unsafe extern "C" fn(*mut BurnSource, c_int, c_int, c_int) -> *mut BurnSource
            );
            let fifo_inquire_status = resolve!(
                lib,
                "burn_fifo_inquire_status",
                unsafe extern "C" fn(
                    *mut BurnSource,
                    *mut c_int,
                    *mut c_int,
                    *mut *const c_char,
                ) -> c_int
            );
            let track_set_size = resolve!(
                lib,
                "burn_track_set_size",
                unsafe extern "C" fn(*mut BurnTrack, c_longlong) -> c_int
            );
            let track_define_data = resolve!(
                lib,
                "burn_track_define_data",
                unsafe extern "C" fn(*mut BurnTrack, c_int, c_int, c_int, c_int)
            );
            let write_opts_new = resolve!(
                lib,
                "burn_write_opts_new",
                unsafe extern "C" fn(*mut BurnDrive) -> *mut BurnWriteOpts
            );
            let write_opts_free = resolve!(
                lib,
                "burn_write_opts_free",
                unsafe extern "C" fn(*mut BurnWriteOpts)
            );
            let write_opts_set_perform_opc = resolve!(
                lib,
                "burn_write_opts_set_perform_opc",
                unsafe extern "C" fn(*mut BurnWriteOpts, c_int)
            );
            let write_opts_set_simulate = resolve!(
                lib,
                "burn_write_opts_set_simulate",
                unsafe extern "C" fn(*mut BurnWriteOpts, c_int) -> c_int
            );
            let write_opts_set_multi = resolve!(
                lib,
                "burn_write_opts_set_multi",
                unsafe extern "C" fn(*mut BurnWriteOpts, c_int)
            );
            let write_opts_set_underrun_proof = resolve!(
                lib,
                "burn_write_opts_set_underrun_proof",
                unsafe extern "C" fn(*mut BurnWriteOpts, c_int)
            );
            let write_opts_set_force = resolve!(
                lib,
                "burn_write_opts_set_force",
                unsafe extern "C" fn(*mut BurnWriteOpts, c_int)
            );
            let write_opts_set_write_type = resolve!(
                lib,
                "burn_write_opts_set_write_type",
                unsafe extern "C" fn(*mut BurnWriteOpts, c_int, c_int) -> c_int
            );
            let write_opts_auto_write_type = resolve!(
                lib,
                "burn_write_opts_auto_write_type",
                unsafe extern "C" fn(
                    *mut BurnWriteOpts,
                    *mut BurnDisc,
                    *mut c_char,
                    c_int,
                ) -> c_int
            );
            let precheck_write = resolve!(
                lib,
                "burn_precheck_write",
                unsafe extern "C" fn(
                    *mut BurnWriteOpts,
                    *mut BurnDisc,
                    *mut c_char,
                    c_int,
                ) -> c_int
            );
            let disc_write = resolve!(
                lib,
                "burn_disc_write",
                unsafe extern "C" fn(*mut BurnWriteOpts, *mut BurnDisc)
            );
            let disc_erase = resolve!(
                lib,
                "burn_disc_erase",
                unsafe extern "C" fn(*mut BurnDrive, c_int)
            );
            let disc_format = resolve!(
                lib,
                "burn_disc_format",
                unsafe extern "C" fn(*mut BurnDrive, c_longlong, c_int)
            );
            let drive_cancel = resolve!(
                lib,
                "burn_drive_cancel",
                unsafe extern "C" fn(*mut BurnDrive)
            );
            let drive_wrote_well = resolve!(
                lib,
                "burn_drive_wrote_well",
                unsafe extern "C" fn(*mut BurnDrive) -> c_int
            );
            let drive_re_assess = resolve!(
                lib,
                "burn_drive_re_assess",
                unsafe extern "C" fn(*mut BurnDrive, c_int) -> c_int
            );
            let disc_get_bd_spare_info = resolve!(
                lib,
                "burn_disc_get_bd_spare_info",
                unsafe extern "C" fn(*mut BurnDrive, *mut c_int, *mut c_int, c_int) -> c_int
            );
            let disc_get_media_id = resolve!(
                lib,
                "burn_disc_get_media_id",
                unsafe extern "C" fn(
                    *mut BurnDrive,
                    *mut *mut c_char,
                    *mut *mut c_char,
                    *mut *mut c_char,
                    *mut *mut c_char,
                    c_int,
                ) -> c_int
            );
            let guess_manufacturer = resolve!(
                lib,
                "burn_guess_manufacturer",
                unsafe extern "C" fn(c_int, *mut c_char, *mut c_char, c_int) -> *mut c_char
            );
            let disc_available_space = resolve!(
                lib,
                "burn_disc_available_space",
                unsafe extern "C" fn(*mut BurnDrive, *mut BurnWriteOpts) -> c_longlong
            );
            let msgs_set_severities = resolve!(
                lib,
                "burn_msgs_set_severities",
                unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> c_int
            );
            let msgs_obtain = resolve!(
                lib,
                "burn_msgs_obtain",
                unsafe extern "C" fn(
                    *mut c_char,
                    *mut c_int,
                    *mut c_char,
                    *mut c_int,
                    *mut c_char,
                ) -> c_int
            );

            Ok(Self {
                _lib: lib,
                initialize,
                finish,
                version,
                drive_scan,
                drive_info_free,
                drive_get_adr,
                drive_get_all_profiles,
                drive_grab,
                drive_release,
                disc_get_status,
                drive_get_status,
                drive_get_speedlist,
                drive_free_speedlist,
                disc_get_profile,
                get_read_capacity,
                disc_erasable,
                drive_get_disc,
                disc_get_sessions,
                session_get_tracks,
                track_get_entry,
                session_get_leadout_entry,
                disc_get_incomplete_sessions,
                disc_free,
                preset_device_open,
                read_data,
                disc_create,
                session_create,
                session_free,
                disc_add_session,
                track_create,
                track_free,
                session_add_track,
                track_set_source,
                source_free,
                file_source_new,
                fifo_source_new,
                fifo_inquire_status,
                track_set_size,
                track_define_data,
                write_opts_new,
                write_opts_free,
                write_opts_set_perform_opc,
                write_opts_set_simulate,
                write_opts_set_multi,
                write_opts_set_underrun_proof,
                write_opts_set_force,
                write_opts_set_write_type,
                write_opts_auto_write_type,
                precheck_write,
                disc_write,
                disc_erase,
                disc_format,
                drive_cancel,
                drive_wrote_well,
                drive_re_assess,
                disc_get_bd_spare_info,
                disc_get_media_id,
                guess_manufacturer,
                disc_available_space,
                msgs_set_severities,
                msgs_obtain,
            })
        }
    }
}

/// Zet een NUL-beëindigd `c_char`-buffer om naar een `String`
/// (SCSI-inquiry-strings zijn vaak met spaties gevuld; die worden getrimd).
pub fn cbuf_to_string(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).trim().to_string()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    /// libburn houdt globale status bij; tests die de library gebruiken mogen
    /// niet parallel draaien.
    pub(crate) static LIBBURN_LOCK: Mutex<()> = Mutex::new(());

    /// Rooktest tegen de echte systeem-library. Slaat stilletjes over als
    /// libburn niet geïnstalleerd is.
    #[test]
    fn load_initialize_version() {
        let _guard = LIBBURN_LOCK.lock().unwrap();
        let raw = match RawLibburn::load("libburn.so.4") {
            Ok(raw) => raw,
            Err(e) => {
                eprintln!("overgeslagen: libburn niet beschikbaar ({e})");
                return;
            }
        };
        unsafe {
            assert_eq!((raw.initialize)(), 1, "burn_initialize moet 1 teruggeven");
            let (mut maj, mut min, mut mic) = (0, 0, 0);
            (raw.version)(&mut maj, &mut min, &mut mic);
            assert!(maj > 0, "burn_version moet een geldige versie geven");
            (raw.finish)();
        }
    }

    /// Test de toggle-keten van “Exclusief openen”: herladen (burn_finish →
    /// initialize) moet opnieuw LibLoaded geven en daarna moet een scan
    /// gewoon werken.
    #[test]
    fn worker_toggle_exclusive_reload() {
        use crate::worker::{Command, Event, spawn};
        use std::sync::mpsc::channel;
        use std::time::Duration;

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

        let (mut loaded1, mut loaded2, mut scan_done) = (false, false, false);
        let mut sent_reload = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while std::time::Instant::now() < deadline {
            match event_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Event::LibLoaded { .. }) => {
                    if !sent_reload {
                        loaded1 = true;
                        sent_reload = true;
                        cmd_tx
                            .send(Command::LoadLibrary {
                                path: None,
                                exclusive: false,
                            })
                            .unwrap();
                    } else {
                        loaded2 = true;
                        cmd_tx.send(Command::Scan).unwrap();
                    }
                }
                Ok(Event::ScanDone { .. }) => {
                    scan_done = true;
                    break;
                }
                Ok(Event::LibLoadFailed { error }) => {
                    panic!("herladen mislukt: {error}");
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        drop(cmd_tx);
        assert!(
            loaded1 && loaded2 && scan_done,
            "toggle-keten faalde: eerste load={loaded1}, herload={loaded2}, scan={scan_done}"
        );
    }
}
