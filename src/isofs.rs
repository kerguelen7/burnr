//! Ruwe FFI-laag voor libisofs — de ISO9660-generator van het libburnia-project.
//!
//! libburn brandt alleen datablokken; het samenstellen van een bestandssysteem
//! uit losse bestanden is de taak van libisofs. Deze module laadt het
//! systeem-`libisofs.so.6` tijdens runtime (zelfde patroon als `ffi.rs`).
//!
//! De types hieronder zijn afgeleid uit `docs/libisofs.h` (libisofs 1.5.6)
//! en moeten overeenkomen met de geïnstalleerde library.

use std::ffi::{c_char, c_int};

use libloading::Library;

use crate::ffi::BurnSource;

/// `ISO_SUCCESS`
pub const ISO_SUCCESS: c_int = 1;
/// `ISO_MSGS_MESSAGE_LEN`
pub const ISO_MSGS_MESSAGE_LEN: usize = 4096;

/// Opaque handle naar `struct IsoImage`.
#[repr(C)]
pub struct IsoImage {
    _private: [u8; 0],
}

/// Opaque handle naar `struct IsoDir` (map in de image-boom).
#[repr(C)]
pub struct IsoDir {
    _private: [u8; 0],
}

/// Opaque handle naar `struct IsoNode` (bestand/map/symlink in de boom).
#[repr(C)]
pub struct IsoNode {
    _private: [u8; 0],
}

/// Opaque handle naar `struct IsoWriteOpts`.
#[repr(C)]
pub struct IsoWriteOpts {
    _private: [u8; 0],
}

/// Alle benodigde functiewijzers, geresolved uit het geladen shared object.
pub struct RawLibisofs {
    _lib: Library,
    pub init: unsafe extern "C" fn() -> c_int,
    pub finish: unsafe extern "C" fn(),
    pub version: unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int),
    pub set_msgs_severities:
        unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> c_int,
    pub obtain_msgs: unsafe extern "C" fn(
        *mut c_char,
        *mut c_int,
        *mut c_int,
        *mut c_char,
        *mut c_char,
    ) -> c_int,
    pub image_new: unsafe extern "C" fn(*const c_char, *mut *mut IsoImage) -> c_int,
    pub image_unref: unsafe extern "C" fn(*mut IsoImage),
    pub image_get_root: unsafe extern "C" fn(*const IsoImage) -> *mut IsoDir,
    pub tree_add_new_node: unsafe extern "C" fn(
        *mut IsoImage,
        *mut IsoDir,
        *const c_char,
        *const c_char,
        *mut *mut IsoNode,
    ) -> c_int,
    pub tree_add_new_dir:
        unsafe extern "C" fn(*mut IsoDir, *const c_char, *mut *mut IsoDir) -> c_int,
    pub tree_add_dir_rec: unsafe extern "C" fn(*mut IsoImage, *mut IsoDir, *const c_char) -> c_int,
    pub write_opts_new: unsafe extern "C" fn(*mut *mut IsoWriteOpts, c_int) -> c_int,
    pub write_opts_free: unsafe extern "C" fn(*mut IsoWriteOpts),
    pub write_opts_set_iso_level: unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int,
    pub write_opts_set_rockridge: unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int,
    pub write_opts_set_joliet: unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int,
    pub write_opts_set_replace_timestamps: unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int,
    pub write_opts_set_dir_rec_mtime: unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int,
    pub image_create_burn_source:
        unsafe extern "C" fn(*mut IsoImage, *mut IsoWriteOpts, *mut *mut BurnSource) -> c_int,
}

macro_rules! resolve_iso {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let sym: libloading::Symbol<$ty> = $lib
            .get::<$ty>(concat!($name, "\0").as_bytes())
            .map_err(|e| format!("symbool `{}` ontbreekt: {e}", $name))?;
        *sym
    }};
}

impl RawLibisofs {
    /// Laadt een libisofs-shared object en resolveert alle benodigde symbolen.
    pub fn load(path: &str) -> Result<Self, String> {
        let lib =
            unsafe { Library::new(path) }.map_err(|e| format!("kan `{path}` niet openen: {e}"))?;
        unsafe {
            // Eerst alle symbolen resolve (met `lib` nog in leven), daarna pas
            // de library in de struct plaatsen.
            let init = resolve_iso!(lib, "iso_init", unsafe extern "C" fn() -> c_int);
            let finish = resolve_iso!(lib, "iso_finish", unsafe extern "C" fn());
            let version = resolve_iso!(
                lib,
                "iso_lib_version",
                unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int)
            );
            let set_msgs_severities = resolve_iso!(
                lib,
                "iso_set_msgs_severities",
                unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> c_int
            );
            let obtain_msgs = resolve_iso!(
                lib,
                "iso_obtain_msgs",
                unsafe extern "C" fn(
                    *mut c_char,
                    *mut c_int,
                    *mut c_int,
                    *mut c_char,
                    *mut c_char,
                ) -> c_int
            );
            let image_new = resolve_iso!(
                lib,
                "iso_image_new",
                unsafe extern "C" fn(*const c_char, *mut *mut IsoImage) -> c_int
            );
            let image_unref =
                resolve_iso!(lib, "iso_image_unref", unsafe extern "C" fn(*mut IsoImage));
            let image_get_root = resolve_iso!(
                lib,
                "iso_image_get_root",
                unsafe extern "C" fn(*const IsoImage) -> *mut IsoDir
            );
            let tree_add_new_node = resolve_iso!(
                lib,
                "iso_tree_add_new_node",
                unsafe extern "C" fn(
                    *mut IsoImage,
                    *mut IsoDir,
                    *const c_char,
                    *const c_char,
                    *mut *mut IsoNode,
                ) -> c_int
            );
            let tree_add_new_dir = resolve_iso!(
                lib,
                "iso_tree_add_new_dir",
                unsafe extern "C" fn(*mut IsoDir, *const c_char, *mut *mut IsoDir) -> c_int
            );
            let tree_add_dir_rec = resolve_iso!(
                lib,
                "iso_tree_add_dir_rec",
                unsafe extern "C" fn(*mut IsoImage, *mut IsoDir, *const c_char) -> c_int
            );
            let write_opts_new = resolve_iso!(
                lib,
                "iso_write_opts_new",
                unsafe extern "C" fn(*mut *mut IsoWriteOpts, c_int) -> c_int
            );
            let write_opts_free = resolve_iso!(
                lib,
                "iso_write_opts_free",
                unsafe extern "C" fn(*mut IsoWriteOpts)
            );
            let write_opts_set_iso_level = resolve_iso!(
                lib,
                "iso_write_opts_set_iso_level",
                unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int
            );
            let write_opts_set_rockridge = resolve_iso!(
                lib,
                "iso_write_opts_set_rockridge",
                unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int
            );
            let write_opts_set_joliet = resolve_iso!(
                lib,
                "iso_write_opts_set_joliet",
                unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int
            );
            let write_opts_set_replace_timestamps = resolve_iso!(
                lib,
                "iso_write_opts_set_replace_timestamps",
                unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int
            );
            let write_opts_set_dir_rec_mtime = resolve_iso!(
                lib,
                "iso_write_opts_set_dir_rec_mtime",
                unsafe extern "C" fn(*mut IsoWriteOpts, c_int) -> c_int
            );
            let image_create_burn_source = resolve_iso!(
                lib,
                "iso_image_create_burn_source",
                unsafe extern "C" fn(
                    *mut IsoImage,
                    *mut IsoWriteOpts,
                    *mut *mut BurnSource,
                ) -> c_int
            );

            Ok(Self {
                _lib: lib,
                init,
                finish,
                version,
                set_msgs_severities,
                obtain_msgs,
                image_new,
                image_unref,
                image_get_root,
                tree_add_new_node,
                tree_add_new_dir,
                tree_add_dir_rec,
                write_opts_new,
                write_opts_free,
                write_opts_set_iso_level,
                write_opts_set_rockridge,
                write_opts_set_joliet,
                write_opts_set_replace_timestamps,
                write_opts_set_dir_rec_mtime,
                image_create_burn_source,
            })
        }
    }
}

/// Zet een NUL-beëindigd `c_char`-buffer om naar een `String`.
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
    use crate::ffi::tests::LIBBURN_LOCK;

    /// Rooktest tegen de echte systeem-library; slaat over als libisofs
    /// niet geïnstalleerd is.
    #[test]
    fn load_init_version() {
        let _guard = LIBBURN_LOCK.lock().unwrap();
        let iso = match RawLibisofs::load("libisofs.so.6") {
            Ok(iso) => iso,
            Err(e) => {
                eprintln!("overgeslagen: libisofs niet beschikbaar ({e})");
                return;
            }
        };
        unsafe {
            assert_eq!((iso.init)(), ISO_SUCCESS, "iso_init moet ISO_SUCCESS geven");
            let (mut maj, mut min, mut mic) = (0, 0, 0);
            (iso.version)(&mut maj, &mut min, &mut mic);
            assert!(maj > 0, "iso_lib_version moet een geldige versie geven");
            (iso.finish)();
        }
    }

    /// Debugtest voor de lege-map-bug: bouwt met dezelfde aanroepen als de
    /// worker een image uit een temp-map en controleert of de inhoud meetelt
    /// in de imagegrootte.
    #[test]
    fn dir_rec_adds_contents() {
        let _guard = LIBBURN_LOCK.lock().unwrap();
        let iso = match RawLibisofs::load("libisofs.so.6") {
            Ok(iso) => iso,
            Err(e) => {
                eprintln!("overgeslagen: libisofs niet beschikbaar ({e})");
                return;
            }
        };
        unsafe {
            assert_eq!((iso.init)(), ISO_SUCCESS);

            // Temp-map met: 1 bestand van 1 MiB + submap met een bestand.
            let tmp = std::env::temp_dir().join("libburn_gui_dirrec_test");
            let _ = std::fs::remove_dir_all(&tmp);
            let sub = tmp.join("submap");
            std::fs::create_dir_all(&sub).unwrap();
            std::fs::write(tmp.join("groot.bin"), vec![0u8; 1024 * 1024]).unwrap();
            std::fs::write(sub.join("klein.txt"), "hallo").unwrap();

            let vol = c"TESTVOL";
            let mut image: *mut IsoImage = std::ptr::null_mut();
            assert_eq!((iso.image_new)(vol.as_ptr(), &mut image), ISO_SUCCESS);
            let root = (iso.image_get_root)(image);

            let dirpath = tmp.to_str().unwrap();
            let name_c = std::ffi::CString::new("testmap").unwrap();

            let mut dir: *mut IsoDir = std::ptr::null_mut();
            let r_new = (iso.tree_add_new_dir)(root, name_c.as_ptr(), &mut dir);
            eprintln!("tree_add_new_dir -> {r_new}");
            // Retourwaarde = aantal nodes in parent bij succes; < 0 = fout.
            assert!(r_new >= 0, "tree_add_new_dir mislukt (code {r_new})");
            assert!(!dir.is_null());

            let path_c = std::ffi::CString::new(dirpath).unwrap();
            let r_rec = (iso.tree_add_dir_rec)(image, dir, path_c.as_ptr());
            eprintln!("tree_add_dir_rec -> {r_rec}");
            assert!(
                r_rec >= 0,
                "iso_tree_add_dir_rec moet slagen (code {r_rec})"
            );

            let mut opts: *mut IsoWriteOpts = std::ptr::null_mut();
            assert_eq!((iso.write_opts_new)(&mut opts, 2), ISO_SUCCESS);
            (iso.write_opts_set_iso_level)(opts, 3);
            (iso.write_opts_set_rockridge)(opts, 1);
            (iso.write_opts_set_joliet)(opts, 1);

            let mut src: *mut crate::ffi::BurnSource = std::ptr::null_mut();
            let r_src = (iso.image_create_burn_source)(image, opts, &mut src);
            eprintln!("create_burn_source -> {r_src}");
            assert_eq!(r_src, ISO_SUCCESS);
            assert!(!src.is_null());

            // Grootte via de get_size-callback van de burn_source.
            #[repr(C)]
            struct Layout {
                refcount: c_int,
                read: Option<
                    unsafe extern "C" fn(*mut crate::ffi::BurnSource, *mut u8, c_int) -> c_int,
                >,
                read_sub: Option<
                    unsafe extern "C" fn(*mut crate::ffi::BurnSource, *mut u8, c_int) -> c_int,
                >,
                get_size: Option<
                    unsafe extern "C" fn(*mut crate::ffi::BurnSource) -> std::ffi::c_longlong,
                >,
                set_size: Option<
                    unsafe extern "C" fn(
                        *mut crate::ffi::BurnSource,
                        std::ffi::c_longlong,
                    ) -> c_int,
                >,
                free_data: Option<unsafe extern "C" fn(*mut crate::ffi::BurnSource)>,
                next: *mut crate::ffi::BurnSource,
                data: *mut std::ffi::c_void,
                version: c_int,
                read_xt: Option<
                    unsafe extern "C" fn(*mut crate::ffi::BurnSource, *mut u8, c_int) -> c_int,
                >,
                cancel: Option<unsafe extern "C" fn(*mut crate::ffi::BurnSource) -> c_int>,
            }
            let layout = src as *const Layout;
            let size = ((*layout).get_size.unwrap())(src);
            eprintln!("imagegrootte: {size} bytes ({}) blokken", size / 2048);
            // Lege image ≈ enkele tientallen blokken; met 1 MiB aan inhoud
            // moet dit ruim boven ~500 blokken uitkomen.
            assert!(
                size > 500 * 2048,
                "mapinhoud ontbreekt in de image (grootte {size})"
            );

            // Opruimen: burn_source_free komt uit libburn.
            if let Ok(burn) = crate::ffi::RawLibburn::load("libburn.so.4") {
                (burn.source_free)(src);
            }
            (iso.write_opts_free)(opts);
            (iso.image_unref)(image);
            (iso.finish)();
            let _ = std::fs::remove_dir_all(&tmp);
        }
    }
}
