//! RAII-wrappers rond de libburn/libisofs-handles (stap 10c).
//!
//! Elke wrapper bezit één C-handle en een kopie van de bijbehorende
//! vrijgeef-functie; `Drop` geeft de handle netjes vrij. Daardoor ruimen ook
//! vroege returns en panics in de job-flows alle tussentijds aangemaakte
//! objecten op — de oude code moest dat handmatig op élk faalpad doen (met
//! echte lekken tot gevolg op enkele paden).
//!
//! De vrijgeef-functies worden als functiewijzer gekopieerd in plaats van een
//! referentie naar `RawLibburn`/`RawLibisofs` vast te houden: zo blijven de
//! wrappers los van de levensduur van de library-structs en zijn ze zonder
//! aliasing-zorgen te verplaatsen (alles blijft op de worker-thread).

use std::ffi::c_int;

use crate::ffi::{self, RawLibburn};
use crate::isofs::{self, RawLibisofs};

// ---------------------------------------------------------------------------
// libburn: gegrabde drive
// ---------------------------------------------------------------------------

/// Gegrabde drive (`burn_drive_grab`). `Drop` geeft de drive weer vrij
/// (`burn_drive_release`, eject = 0), tenzij dat al expliciet gebeurde via
/// `release_now()`.
///
/// De drive-pointer blijft na `release_now` bruikbaar voor een hergrab
/// (`regrab`) — nodig voor de eject-reeks na een brand: release → settle →
/// unmount → regrab → release(eject).
pub(crate) struct GrabbedDrive {
    drive: *mut ffi::BurnDrive,
    /// `false` na een expliciete release; Drop doet dan niets meer.
    armed: bool,
    grab: unsafe extern "C" fn(*mut ffi::BurnDrive, c_int) -> c_int,
    release: unsafe extern "C" fn(*mut ffi::BurnDrive, c_int),
}

impl GrabbedDrive {
    /// Grabt de drive (load = 0); `None` als `burn_drive_grab` faalt.
    pub(crate) fn grab(raw: &RawLibburn, drive: *mut ffi::BurnDrive) -> Option<Self> {
        (unsafe { (raw.drive_grab)(drive, 0) } == 1).then(|| Self {
            drive,
            armed: true,
            grab: raw.drive_grab,
            release: raw.drive_release,
        })
    }

    /// Neemt een al gegrabde drive over — voor `burn_drive_scan_and_grab`,
    /// dat zelf grabt (alleen gebruikt door de shutdown-tijdens-job test).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn assume_grabbed(raw: &RawLibburn, drive: *mut ffi::BurnDrive) -> Self {
        Self {
            drive,
            armed: true,
            grab: raw.drive_grab,
            release: raw.drive_release,
        }
    }

    pub(crate) fn handle(&self) -> *mut ffi::BurnDrive {
        self.drive
    }

    /// Geeft de drive expliciet vrij (eject != 0 vraagt uitwerpen). Daarna
    /// geeft `Drop` niet nogmaals vrij; `handle()` blijft bruikbaar voor een
    /// hergrab.
    pub(crate) fn release_now(&mut self, eject: c_int) {
        if self.armed {
            unsafe { (self.release)(self.drive, eject) };
            self.armed = false;
        }
    }

    /// Grabt opnieuw (na `release_now`); zet `Drop` weer aan bij succes.
    /// Geeft true als de drive gegrabbed is.
    pub(crate) fn regrab(&mut self) -> bool {
        if self.armed {
            return true;
        }
        self.armed = unsafe { (self.grab)(self.drive, 0) } == 1;
        self.armed
    }
}

impl Drop for GrabbedDrive {
    fn drop(&mut self) {
        self.release_now(0);
    }
}

// ---------------------------------------------------------------------------
// libburn: disc-model, bronnen en write-opts
// ---------------------------------------------------------------------------

macro_rules! owned_handle {
    ($name:ident, $ty:ty, $doc:expr, $sel_free:ident) => {
        #[doc = $doc]
        pub(crate) struct $name {
            ptr: *mut $ty,
            free: unsafe extern "C" fn(*mut $ty),
        }

        impl $name {
            /// Neemt eigendom van een zojuist aangemaakte handle; `None` bij NULL.
            pub(crate) fn new(raw: &RawLibburn, ptr: *mut $ty) -> Option<Self> {
                (!ptr.is_null()).then(|| Self {
                    ptr,
                    free: raw.$sel_free,
                })
            }

            pub(crate) fn handle(&self) -> *mut $ty {
                self.ptr
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                if !self.ptr.is_null() {
                    unsafe { (self.free)(self.ptr) };
                }
            }
        }
    };
}

owned_handle!(
    OwnedDisc,
    ffi::BurnDisc,
    "Eigenaar van `struct burn_disc`; Drop roept `burn_disc_free` aan. De bij\n/// de disc geregistreerde sessies/tracks worden door libburn (refcounting)\n/// meegeruimd.",
    disc_free
);
owned_handle!(
    OwnedSession,
    ffi::BurnSession,
    "Eigenaar van `struct burn_session`; Drop roept `burn_session_free` aan.",
    session_free
);
owned_handle!(
    OwnedTrack,
    ffi::BurnTrack,
    "Eigenaar van `struct burn_track`; Drop roept `burn_track_free` aan.",
    track_free
);
owned_handle!(
    OwnedSource,
    ffi::BurnSource,
    "Eigenaar van een `burn_source`; Drop roept `burn_source_free` aan.",
    source_free
);
owned_handle!(
    OwnedWriteOpts,
    ffi::BurnWriteOpts,
    "Eigenaar van `struct burn_write_opts`; Drop roept `burn_write_opts_free`\n/// aan.",
    write_opts_free
);

/// Compleet disc-model van een brandjob: disc → session → track.
///
/// Veldvolgorde = drop-volgorde: eerst de track, daarna de session, daarna de
/// disc — dezelfde volgorde als de oude handmatige opruimronde. De velden
/// bestaan primair voor hun RAII-Drop; alleen `disc` wordt ook uitgelezen.
#[allow(dead_code)]
pub(crate) struct BurnModel {
    pub(crate) track: OwnedTrack,
    pub(crate) session: OwnedSession,
    pub(crate) disc: OwnedDisc,
}

// ---------------------------------------------------------------------------
// libburn: snelhedenlijst
// ---------------------------------------------------------------------------

/// Kopie van de snelhedenlijst van een drive
/// (`burn_drive_get_speedlist`); `Drop` roept `burn_drive_free_speedlist` aan.
pub(crate) struct SpeedList {
    head: *mut ffi::SpeedDescriptor,
    free: unsafe extern "C" fn(*mut *mut ffi::SpeedDescriptor) -> c_int,
}

impl SpeedList {
    /// Vraagt de lijst op; `None` bij fout of een lege lijst.
    pub(crate) fn get(raw: &RawLibburn, drive: *mut ffi::BurnDrive) -> Option<Self> {
        let mut head: *mut ffi::SpeedDescriptor = std::ptr::null_mut();
        let r = unsafe { (raw.drive_get_speedlist)(drive, &mut head) };
        (r > 0 && !head.is_null()).then(|| Self {
            head,
            free: raw.drive_free_speedlist,
        })
    }

    /// Begin van de geketende lijst (via `.next` doorlopen).
    pub(crate) fn head(&self) -> *mut ffi::SpeedDescriptor {
        self.head
    }
}

impl Drop for SpeedList {
    fn drop(&mut self) {
        if !self.head.is_null() {
            unsafe { (self.free)(&mut self.head) };
        }
    }
}

// ---------------------------------------------------------------------------
// libisofs: image, write-opts en data-bron
// ---------------------------------------------------------------------------

macro_rules! owned_iso_handle {
    ($name:ident, $ty:ty, $doc:expr, $sel_free:ident) => {
        #[doc = $doc]
        pub(crate) struct $name {
            ptr: *mut $ty,
            free: unsafe extern "C" fn(*mut $ty),
        }

        impl $name {
            /// Neemt eigendom van een zojuist aangemaakte handle; `None` bij NULL.
            pub(crate) fn new(_iso: &RawLibisofs, ptr: *mut $ty) -> Option<Self> {
                (!ptr.is_null()).then(|| Self {
                    ptr,
                    free: _iso.$sel_free,
                })
            }

            pub(crate) fn handle(&self) -> *mut $ty {
                self.ptr
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                if !self.ptr.is_null() {
                    unsafe { (self.free)(self.ptr) };
                }
            }
        }
    };
}

owned_iso_handle!(
    IsoImageRef,
    isofs::IsoImage,
    "Referentie op een libisofs-image; Drop roept `iso_image_unref` aan.",
    image_unref
);
owned_iso_handle!(
    IsoWriteOptsRef,
    isofs::IsoWriteOpts,
    "Referentie op libisofs write-opts; Drop roept `iso_write_opts_free` aan.",
    write_opts_free
);
owned_iso_handle!(
    IsoDataSourceRef,
    isofs::IsoDataSource,
    "Referentie op een libisofs-data bron; Drop roept `iso_data_source_unref`\n/// aan.",
    data_source_unref
);

/// De libisofs-delen van een "bestanden branden"-job die pas ná de brand
/// vrijgegeven mogen worden (oude bestandsdata wordt tijdens het schrijven
/// gelezen).
///
/// Veldvolgorde = drop-volgorde: eerst de write-opts, daarna het image,
/// daarna de data-bron — dezelfde volgorde als de oude handmatige opruimronde.
/// De velden bestaan primair voor hun RAII-Drop; de handles worden via de
/// wrappers zelf uitgelezen vóór de brand.
#[allow(dead_code)]
pub(crate) struct IsoParts {
    pub(crate) opts: IsoWriteOptsRef,
    pub(crate) image: IsoImageRef,
    /// Alleen bij multi-session import; blijft bewust alive tot na de brand.
    pub(crate) data_src: Option<IsoDataSourceRef>,
}
