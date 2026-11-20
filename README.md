# Burnr: a lightweight libburn GUI for optical discs

A modern egui front-end for [libburn](https://libburnia-project.org/), the CD/DVD/BD burning library.

libburn is not compiled in. Burnr loads the system shared object at runtime, so the GUI builds and starts even where libburn is missing, and then explains what to install.

<img width="1919" height="790" alt="screenshot-burnr-gui-detail-v0 9" src="https://github.com/user-attachments/assets/c8768af1-d4a8-4c63-a2a1-460103989db3" />

## Requirements

- Rust 1.85+ (edition 2024; tested with 1.96)
- libburn runtime, e.g. on Debian/Ubuntu:

  ```sh
  sudo apt install libburn4        # runtime
  sudo apt install libburn-dev     # optional: headers, useful as reference
  ```

At startup Burnr searches for libburn in this order:

1. the path in the `LIBBURN_SO` environment variable
2. `libburn.so.4`
3. `libburn.so`

If none is found, a help panel lets you point to a shared object manually and reload.

## Build and run

```sh
cargo build --release
./target/release/burnr
```

## Debian package

```sh
cargo install cargo-deb --locked    # once
cargo deb
sudo apt install ./target/debian/burnr_0.9.0-1_amd64.deb
```

Uninstall with `sudo apt remove burnr`.

The package installs the binary, a menu entry (`burnr.desktop`, matching the Wayland app id) and hicolor icons. It registers `application/x-iso9660-image`, so `.iso` files can be opened with Burnr from the file manager. No maintainer scripts are needed; the icon and MIME caches update through dpkg triggers.

Dependencies use the t64 package names of Debian trixie, with the plain names as alternatives for older releases:
`libc6, libburn4t64 | libburn4, libisofs6t64 | libisofs6`

## Features

**Drives and media**
- Drive scan with details (capabilities, block types, buffer)
- Media inspection: profile, readable capacity, TOC (sessions and tracks), media code and manufacturer, BD defect management status
- Multiple drives in parallel, each with its own progress, LED and cancel

**Reading**
- Disc image reading in 2048-byte blocks, with progress, speed and cancel (data media only)

**Writing**
- Burn ISO files, or compose a data disc from files and folders (libisofs, Rock Ridge + Joliet, no intermediate file)
- Multi-session on write-once media (BD-R, DVD±R, CD-R): the existing session is imported and a new one appended
- Erase (quick or full) for CD-RW and sequential DVD-RW
- **Restore attempt** (advanced, collapsed by default): re-formats media with MMC FORMAT UNIT as a rescue action. See the limitations below.

**Integration and interface**
- Open `.iso` files from the file manager, or run `burnr image.iso`. The path is prefilled as burn source; burning never starts automatically.
- Log panel with level filter and a session log file in `~/.local/share/burnr/logs/`
- Persistent settings
- Interface in en-US (default), nl-NL and de-DE, switchable in the top bar; the log follows the language
- Themes: Dark, Soft, Light, High contrast
- Zoom 80–160% via the top bar or Ctrl+ / Ctrl− / Ctrl+0

## Known limitations

- **Simulation** works only on write-once media (CD-R, DVD-R, DVD+R, BD-R). Overwritable media (BD-RE, DVD+RW, DVD-RAM, formatted DVD-RW) cannot simulate; libburn rejects the job. The GUI warns in advance.
- **CD audio** is not supported, for reading or burning.
- **RAW write mode** is not possible from an ISO file, since it requires 2352-byte source data. The GUI refuses it and explains why.
- **Formatting depends on the drive.** Some drives refuse to format BD-RE. For example, a MATSHITA BD-MLT UJ265 (fw 1.00) fails with MEDIUM ERROR 3/31/01 on every format type, while a TSSTcorp SE-506AB formats the same media without problems. A failed attempt does not destroy data, but can leave the disc temporarily unreadable; power-cycling the drive usually fixes this. The "skip certification" option works around drives that fail on full certification.
- **Multi-session** is only available for write-once media; the setting is disabled for overwritable media.
- **Disc image reading copies the full readable capacity**, not just the files. Rewritable media report their full formatted capacity, so a 6 GB data set on a 22.56 GiB BD-RE yields a 22.56 GiB image with a zero-filled tail (which compresses well). Write-once media report only up to the last written sector. The GUI shows a hint for overwritable media.
- **Book type on BD** is not meaningful. libburn decodes the BD disc information with the DVD table, so the book type is shown for DVD media only. On BD the media kind comes from the Disc Type Identifier (BDO/BDR/BDW). The media code is unaffected.
- **Multiple drives** receive the same source and settings for simultaneous burns.

## Structure

| File | Contents |
| --- | --- |
| `src/main.rs` | eframe bootstrap (window icon, app id) |
| `src/app.rs` | application state, event handling, panel layout |
| `src/ffi.rs` | raw FFI: `#[repr(C)]` types from `libburn.h`, loaded dynamically via `libloading` |
| `src/isofs.rs` | raw FFI for libisofs (`libisofs.so.6`) to compose ISO9660 images |
| `src/raii.rs` | RAII wrappers around libburn/libisofs handles (freed in `Drop`) |
| `src/worker.rs` | background worker; all libburn calls run on one thread |
| `src/logger.rs` | feedback log with levels and timestamps |
| `src/settings.rs` | user settings |
| `src/i18n.rs` | text catalogue: `Lang` enum and per-language texts (en-US, nl-NL, de-DE) |
| `src/ui/` | panels: top bar, drives, details, settings, log, about |
| `assets/` | icons, `burnr.desktop` and the DEP-5 `copyright` file for the .deb |
| `QUICKSTART.md` | one-page getting-started guide |
| `docs/libburn.h` | reference header (libburn 1.5.6) the FFI layout is derived from |
| `docs/i18n-glossary.md` | terminology and translation conventions |

## Tests

```sh
cargo test
```

The smoke tests load the real system library and check `burn_initialize`, `burn_version` and the full scan flow, including the struct layout of `burn_drive_info`. Without libburn installed, they skip quietly.

## License

GNU General Public License version 3 or later. See [LICENSE](LICENSE).

## Acknowledgements

- [libburn](https://libburnia-project.org/) and libisofs, the libraries that do the actual optical work (loaded at runtime, not compiled in)
- Developed by hand, with GLM (Z.ai) as an AI coding assistant in the Zed editor. Design, testing and all decisions by the author.
```
