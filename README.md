# Burnr — A lightweight libburn GUI for optical discs

A modern egui front-end for [libburn](https://libburnia-project.org/) — the
CD/DVD/BD burning library. The project is built **step by step**; every step
gives clear feedback in the log panel.

libburn is **not compiled in**: at runtime the GUI loads the system shared
object (`libburn.so.4`). As a result the GUI builds and starts even on systems
where libburn is not (yet) installed, and then explains what is needed.

## Requirements

- Rust 1.85+ (edition 2024; developed and tested with 1.96)
- libburn runtime on the system, e.g. on Debian/Ubuntu:

  ```sh
  sudo apt install libburn4        # runtime
  sudo apt install libburn-dev     # optional: headers, useful as reference
  ```

On startup the GUI searches in this order:

1. the path from the `LIBBURN_SO` environment variable
2. `libburn.so.4`
3. `libburn.so`

If that fails, the GUI shows a help panel where you can manually point to a
libburn shared object and reload.

## Building and running

```sh
cargo build --release
./target/release/burnr
```

## Features

- Drive scan and per-drive details (capabilities, block types, buffer)
- Media inspection: grab → status → speeds → release, profile, readable
  capacity, TOC (sessions/tracks), media code and manufacturer, BD defect
  management status
- Disc image reading (`burn_read_data`, 2048-byte blocks) with progress,
  speed and cancel — data media only
- Burning ISO files and self-composed data discs (libisofs, Rock Ridge +
  Joliet, no intermediate file)
- Multi-session import on write-once media (BD-R, DVD±R, CD-R): the existing
  session is imported and the new session is appended at the NWA
- Erasing (quick/full) for CD-RW and sequential DVD-RW
- **Restore attempt** (advanced, collapsed by default): re-formats the media
  (MMC FORMAT UNIT) as a rescue action for discs in a bad state. The outcome
  depends on drive and firmware; a failed attempt does not endanger the data
  but may leave the disc temporarily unreadable (power-cycling the drive
  usually fixes that). Options: defect management on/off, certification
  on/off.
- Multiple drives in parallel with per-drive progress, LED and cancel
- Session log file for troubleshooting (`~/.local/share/burnr/logs/`)
- Persistent settings (eframe persistence)
- Interface in **en-US** (default), **nl-NL** or **de-DE** — switch in the
  top bar; the log follows the language as well
- Color themes **Dark / Soft / Light / High contrast**; accent-coloured
  primary actions; about box with the runtime library versions and license

## Known limitations

- **Simulation**: only possible on write-once media (CD-R, DVD-R, DVD+R,
  BD-R). Overwritable media (BD-RE, DVD+RW, DVD-RAM, formatted DVD-RW) can
  **never** simulate — libburn rejects the burn job. The GUI warns
  proactively and the error message gives a concrete tip.
- **CD audio**: reading and burning audio tracks is not supported yet
  (`burn_read_data` only delivers 2048-byte data blocks).
- **RAW write mode**: not possible from an ISO file (requires 2352-byte
  source data); the GUI refuses this with an explanation.
- **Formatting is drive-dependent**: some drives refuse to re-format BD-RE
  entirely (observed: MATSHITA BD-MLT UJ265 fw 1.00 fails with MEDIUM ERROR
  3/31/01 "Format command failed" on all offered format types, while the
  same media formats fine in a TSSTcorp SE-506AB). A failed format attempt
  does not destroy the data, but can make the disc temporarily unreadable;
  a power-cycle of the drive (or of the whole machine) usually restores
  readability. This is why the format action is presented as a
  "restore attempt" and is hidden behind an advanced collapsible section.
- **Multi-session** applies to write-once media only; overwritable media
  (DVD+RW, BD-RE, DVD-RAM, formatted DVD-RW) is always writable, so the
  setting is disabled for it.

## Structure

| File | Contents |
| --- | --- |
| `src/main.rs` | eframe bootstrap (window icon, app id) |
| `src/app.rs` | application state, event handling, panel layout |
| `src/ffi.rs` | raw FFI: `#[repr(C)]` types from `libburn.h` + dynamic loading via `libloading` |
| `src/isofs.rs` | raw FFI for libisofs (`libisofs.so.6`): composing an ISO9660 image as a burn source |
| `src/raii.rs` | RAII ownership wrappers around the libburn/libisofs handles (Drop frees) |
| `src/worker.rs` | background worker that runs all libburn calls on one thread |
| `src/logger.rs` | feedback log with levels and timestamps |
| `src/settings.rs` | user settings (passed to libburn in the burn step) |
| `src/i18n.rs` | typed text catalogue: `Lang` enum + per-language `Texts` + worker log methods (en-US / nl-NL / de-DE) |
| `src/ui/` | panels: top bar, drives, details, settings, log, about |
| `assets/` | window icon: `icon.png` (embedded in the binary, loaded in `main.rs`) and `icon.svg` (source for the future .deb installation) |
| `QUICKSTART.md` | short getting-started guide (one page) |
| `docs/libburn.h` | reference header (libburn 1.5.6) from which the FFI layout was derived |
| `docs/i18n-glossary.md` | terminology and translation conventions for the catalogue |

## Roadmap

- [x] **Step 1 — Foundation**: egui shell, runtime loading of libburn, drive
  scan, drive details (capabilities, block types, buffer), media inspection
  (grab → status → speeds → release), feedback log with level filter,
  settings panel (values managed, wiring comes later).
- [x] **Step 2 — Media details**: media type via `burn_disc_get_profile`,
  readable capacity via `burn_get_read_capacity`, rewritability via
  `burn_disc_erasable`, TOC via `burn_drive_get_disc` → sessions → tracks
  (type, start LBA, size per track), incomplete sessions. Old inspection
  values are now cleared as soon as a new inspection starts.
- [x] **Step 3 — Disc image reading**: `burn_read_data` (random access,
  2048-byte blocks) to a single file, with progress bar, speed and cancel;
  data media only (no CD audio). Device open mode (`burn_preset_device_open`)
  configurable: turn off "Exclusive open" if the file manager mounts the disc
  (automount, e.g. Nemo/udisks2); grab failures now show a targeted hint with
  unmount advice.
- [x] **Step 4 — Burning**: burn an ISO file via `burn_disc_write` with a
  disc/session/track model, file source + 4 MiB FIFO (`burn_fifo_source_new`),
  write opts (speed via `burn_drive_set_speed`, TAO/SAO/auto via
  `burn_write_opts_auto_write_type`, simulation, multi-session, padding,
  overburn/force, underrun-proof), optional erase beforehand
  (`burn_disc_erase` + `burn_drive_re_assess`), progress with sector/speed/
  buffer/FIFO fill, cancel via `burn_drive_cancel`, result check via
  `burn_drive_wrote_well`, and libburn messages (`burn_msgs_*`) live in the
  log panel.
- [x] **Step 5 — Composing data discs**: choose files/folders in the GUI
  (file/folder picker, duplicate filter and size estimate) and burn them as
  an ISO9660 image via **libisofs** (`libisofs.so.6`, runtime loading like
  libburn; Rock Ridge + Joliet, ISO level 3).
  `iso_image_create_burn_source()` returns a burn_source that is attached
  directly to the libburn track — no intermediate file. The burn flow is
  shared with ISO burning (grab/media check/erase, write opts, poll loop).
  Also: phase display while burning (lead-out/track closing becomes visible
  after 100%), compact device info (technical details collapsible), file
  pickers via `rfd`, and the "islands" re-layout of the central panel
  (Drive & media / Burning / Disc image).
- [x] **Step 6 — Erase/format**: separate 🧽 actions in the media island:
  erase (quick/full) for CD-RW and sequential DVD-RW, formatting
  (`burn_disc_format`, default size) for DVD-RW/DVD+RW/DVD-RAM/BD-RE.
  Smart button activation per profile, progress in log and UI, cancel, and
  the erase flow and burn flow now share one wait loop.
- [x] **Step 7 — Multi-session with import (write-once media)**: add files
  to a written BD-R/DVD-R/DVD+R/CD-R while keeping the existing content.
  The existing session is imported (`iso_data_source_new_from_file` +
  `iso_image_import` with the start block from the TOC), new files go into
  the same tree, and the new session is written at the NWA with `appendable`
  + `ms_block` — the old file data stays physically intact. The data source
  stays alive until after the burn (old file data is read while writing the
  disc). Write-once media only; DVD+RW/BD-RE use overwriting.
- [x] **Step 8 — Persistent settings + extras**: all settings, exclusive
  open, source choice and last-used paths are stored via eframe persistence
  and loaded at startup. Also: **media code display**
  (`burn_disc_get_media_id` + manufacturer guess via
  `burn_guess_manufacturer`, e.g. "PHILIP R04 — Philips"), **defect
  management** status for BD media (`burn_disc_get_bd_spare_info`), a switch
  to disable DM when formatting (bit5, faster burning), a **speed combo
  box** filled with the speeds from the media inspection, and **eject
  diagnostics**: for overwritable media a settle pause before the eject
  request (background formatting) with log feedback.
- [x] **Step 9 — Finishing touches (partial, rest moved)**:
  - **Session log file** (troubleshooting): every session writes all log
    lines with full timestamp to `~/.local/share/burnr/logs/`;
    button "💾 Save…" in the log panel.
  - **Multiple drives at once**: the worker polls all active jobs
    non-blocking; burn/erase/format on different drives runs in parallel,
    each with its own progress/LED/cancel. Reading remains globally
    sequential.
  - Limitation documented in the UI: with multiple drives, the same source
    (files/ISO) and the same settings apply to simultaneous burns.
  - Presets/favourites: scrapped (settings are already persistent).
- [x] **Step 10a — UI refinements & format diagnostics** (this thread):
  - Multi-session choice simplified to a binary Yes/No radio pair
    (default: No — disc gets closed), with serde aliases so existing
    settings keep loading.
  - Multi-session setting greyed out for overwritable media (profile-based).
  - "Readable:" line on the media card is now status-aware (explains blank
    overwritable media, CD audio, no disc) instead of a cryptic dash.
  - Burn speed display emphasised (bold, bright orange) during burning.
  - Average speed logged after a successful burn (also in the session log
    file).
  - Window icon (`assets/icon.png` embedded via `include_bytes!`) and
    Wayland app id (`burnr`) for the future .desktop integration.
  - libburn message queue threshold lowered to DEBUG so SCSI error
    conditions on individual commands become visible in the log.
  - Format diagnostics: format capabilities are logged before every format
    (status + offered format types via `burn_disc_get_formats`/
    `burn_disc_get_format_descr`), a proactive warning when "disable defect
    management" is requested but the drive does not offer type 0x31, and a
    SCSI command log (`burn_set_scsi_logging` →
    `/tmp/libburn_sg_command_log`) active during erase/format jobs.
  - "Skip certification" option (libburn flag bit6 → format type 0x00
    without certification) — the workaround for drives that fail on full
    certification ("Format command failed", MMC sense 3/31/01).
  - Read-capacity quirk worked around: after a failed READ CAPACITY libburn
    still reports "1 block" as success; the inspection now ignores that.
  - Format action re-branded as **"Restore attempt"** and moved behind an
    advanced collapsible section with positive toggle switches (defect
    management / certification), because formatting optical media is
    drive-dependent and is only needed as a rescue action.
- [x] **Step 10b — i18n, themes & polish**: typed text catalogue
  (`src/i18n.rs`: `Lang` enum, per-language `Texts` struct for all panels,
  per-message methods for the worker log — en-US source, nl-NL carried over
  1:1, de-DE translated; terminology in `docs/i18n-glossary.md`), language
  switch in the top bar (en-US default), the worker receives the language
  so all log lines follow, `LogFilter` newtype replacing the magic indices,
  color themes Dark / Soft / Light / High contrast, accent-coloured primary
  actions, about box with the runtime library versions, media-aware eject
  settle pause (2000 ms for overwritable media — drives silently ignore an
  eject during background formatting), a "General" settings group (padding,
  timestamps, eject), quickstart guide (`QUICKSTART.md`), README fully in
  en-US, `rust-version` pinned, clippy-clean, and the license
  (GPL-3.0-or-later, see `LICENSE`).
- [ ] **Step 10c — Next thread**: the RAII hardening pass is done (this
  thread); the .deb package is next (Depends: libburn4, libisofs6 —
  libisoburn1 not needed; check the t64 suffix on Debian trixie; the
  .desktop file must be named exactly `burnr.desktop` to match the Wayland
  app id).
  - **RAII hardening (done)**: new `src/raii.rs` with ownership wrappers
    around every libburn/libisofs handle — `GrabbedDrive` (grab/release/
    regrab/eject), `OwnedDisc/Session/Track/Source/WriteOpts`, `SpeedList`,
    composed `BurnModel`, and for libisofs `IsoImageRef`/`IsoWriteOptsRef`/
    `IsoDataSourceRef`/`IsoParts`. Every wrapper frees its handle in `Drop`,
    so early returns and panics can no longer leak (the old error ladders had
    real leaks, e.g. in the model-build error paths and the multi-session
    import). `WorkerState` got a `Drop` (drive list → burn_finish →
    iso_finish), the two burn flows share one tail (`start_write_job`:
    space check, model, write opts, start), and `grab_and_check_media`/
    `make_write_opts` hand out RAII types. i18n messages that became unused
    were removed.
  - **Shutdown-during-job tests (done)**: `worker_shutdown_during_scan`
    (Shutdown mid-scan → WorkerStopped + clean thread exit) and
    `shutdown_during_stdio_burn` (a real async burn job on a `stdio:`
    pseudo-drive via the new `burn_drive_scan_and_grab` FFI, cancelled
    mid-flight like `Command::Shutdown` does, then polled to IDLE and torn
    down via RAII). The scan test exposed a real pre-existing crash:
    aborting a scan left libburn's detached scan thread running, and
    `burn_finish()`/`dlclose` under it segfaulted. The scan abort path now
    drains the scan protocol until the thread reports done, frees the
    result array and settles briefly before shutdown.

## Tests

```sh
cargo test
```

The smoke tests load the real system library and check
`burn_initialize`/`burn_version` and the full scan flow (including the
struct layout of `burn_drive_info`). Without libburn on the system they
quietly skip.

## License

GNU General Public License version 3 **or later** — see [LICENSE](LICENSE).
(`QUICKSTART.md` has a one-page getting-started guide.)
