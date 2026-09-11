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

use std::ffi::{c_char, c_int, c_longlong, c_uint};

use libloading::Library;

/// Maximale lengte van een drive-adres (`BURN_DRIVE_ADR_LEN`).
pub const BURN_DRIVE_ADR_LEN: usize = 1024;

/// Opaque handle naar `struct burn_drive`.
#[repr(C)]
pub struct BurnDrive {
    _private: [u8; 0],
}

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
    pub drive_grab: unsafe extern "C" fn(*mut BurnDrive, c_int) -> c_int,
    pub drive_release: unsafe extern "C" fn(*mut BurnDrive, c_int),
    pub disc_get_status: unsafe extern "C" fn(*mut BurnDrive) -> c_int,
    pub drive_get_status: unsafe extern "C" fn(*mut BurnDrive, *mut Progress) -> c_int,
    pub drive_get_speedlist:
        unsafe extern "C" fn(*mut BurnDrive, *mut *mut SpeedDescriptor) -> c_int,
    pub drive_free_speedlist: unsafe extern "C" fn(*mut *mut SpeedDescriptor) -> c_int,
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

            Ok(Self {
                _lib: lib,
                initialize,
                finish,
                version,
                drive_scan,
                drive_info_free,
                drive_get_adr,
                drive_grab,
                drive_release,
                disc_get_status,
                drive_get_status,
                drive_get_speedlist,
                drive_free_speedlist,
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
}
