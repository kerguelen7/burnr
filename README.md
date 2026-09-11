# LibBurn GUI

Een moderne egui-interface rond [libburn](https://libburnia-project.org/) — de
CD/DVD/BD-brandbibliotheek. Het project wordt **stap voor stap** opgebouwd;
bij elke stap geeft de GUI duidelijke feedback in het logpaneel.

libburn wordt **niet meegecompileerd**: tijdens runtime wordt het shared object
van het systeem geladen (`libburn.so.4`). De GUI bouwt en start daardoor ook op
systemen waar libburn (nog) niet geïnstalleerd is, en legt dan duidelijk uit wat
er nodig is.

## Vereisten

- Rust 1.95+ (edition 2024)
- libburn-runtime op het systeem, bijv. op Debian/Ubuntu:

  ```sh
  sudo apt install libburn4        # runtime
  sudo apt install libburn-dev     # optioneel: headers, handig als referentie
  ```

De GUI zoekt bij het starten in deze volgorde:

1. het pad uit de omgevingsvariabele `LIBBURN_SO`
2. `libburn.so.4`
3. `libburn.so`

Lukt dat niet, dan toont de GUI een hulppaneel waar je handmatig een pad naar
een libburn-shared object kunt opgeven en opnieuw kunt laden.

## Bouwen en starten

```sh
cargo build --release
./target/release/libburn_gui
```

## Structuur

| Bestand | Inhoud |
| --- | --- |
| `src/main.rs` | eframe-bootstrap |
| `src/app.rs` | applicatiestatus, event-verwerking, paneelindeling |
| `src/ffi.rs` | ruwe FFI: `#[repr(C)]`-types uit `libburn.h` + dynamisch laden via `libloading` |
| `src/worker.rs` | achtergrondworker die álle libburn-aanroepen op één thread uitvoert |
| `src/logger.rs` | feedback-log met niveaus en tijdstempels |
| `src/settings.rs` | gebruikersinstellingen (worden in de brand-stap aan libburn gekoppeld) |
| `src/ui/` | panelen: bovenbalk, stations, details, instellingen, log |
| `docs/libburn.h` | referentie-header (libburn 1.5.6) waar de FFI-layout van is afgeleid |

## Stappenplan

- [x] **Stap 1 — Fundament**: egui-shell, runtime-loading van libburn, drive-scan,
  drive-details (capabilities, bloktypen, buffer), media-inspectie
  (grab → status → snelheden → release), feedback-log met niveaufilter,
  instellingenpaneel (waarden worden beheerd, koppeling volgt later).
- [x] **Stap 2 — Media-details**: mediatype via `burn_disc_get_profile`,
  leesbare capaciteit via `burn_get_read_capacity`, herbeschrijfbaarheid via
  `burn_disc_erasable`, TOC via `burn_drive_get_disc` → sessies → tracks
  (type, start-LBA, grootte per track), onvolledige sessies. Oude
  inspectiewaarden worden nu gewist zodra een nieuwe inspectie start.
- [x] **Stap 3 — Schijfkopie lezen**: `burn_read_data` (random access,
  2048-byte blokken) naar één bestand, met voortgangsbalk, snelheid en
  annuleren; alleen datamedia (geen CD-audio). Apparaat-openingsmodus
  (`burn_preset_device_open`) instelbaar: “Exclusief openen” uitzetten als de
  bestandsbeheerder de schijf aankoppelt (automount, bijv. Nemo/udisks2);
  grab-fouten tonen nu een gerichte hint met unmount-advies.
- [ ] **Stap 2 — Media-details**: profiel/media-type via `burn_disc_get_profile`,
  TOC lezen (`burn_disc_read_toc`), capaciteit (`burn_disc_get_media_capacity`).
- [ ] **Stap 3 — Schijfkopie lezen**: `burn_disc_read` met voortgangsbalk
  (`burn_drive_get_status` + `burn_progress`), FIFO (`burn_fifo_new`).
- [ ] **Stap 4 — Branden**: `burn_source` uit bestand, `burn_write_opts`
  (snelheid, TAO/SAO/RAW, simulatie, multi-session, padding, overburn),
  voortgang en bufferstatus live in de UI.
- [ ] **Stap 5 — Wissen/formatteren**: `burn_disc_erase`, `burn_disc_format`.
- [ ] **Stap 6 — Instellingen volledig koppelen**: `burn_preset_device_open`,
  snelheidslimieten, multi-session-gedrag; instellingen persistently opslaan.
- [ ] **Stap 7 — Afwerking**: favorieten/presets, meerdere stations tegelijk,
  foutopsporing (libburn-message-callback `burn_set_message_handler`).

## Tests

```sh
cargo test
```

De rooktests laden de echte systeem-library en controleren
`burn_initialize`/`burn_version` en de volledige scan-flow (incl. de
struct-layout van `burn_drive_info`). Zonder libburn op het systeem slaan ze
stilletjes over.