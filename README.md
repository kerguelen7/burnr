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

## Bekende beperkingen

- **Simulatie**: alleen mogelijk op schrijf-eenmalige media (CD-R, DVD-R,
  DVD+R, BD-R). Overbeschrijfbare media (BD-RE, DVD+RW, DVD-RAM,
  geformatteerde DVD-RW) kan **nooit** simuleren — libburn wijst het brandjob
  dan af. De GUI waarschuwt proactief en de foutmelding geeft een concrete tip.
- **CD-audio**: lezen en branden van audiotracks wordt nog niet ondersteund
  (`burn_read_data` levert alleen 2048-byte datablokken).
- **RAW-schrijfmodus**: niet mogelijk vanuit een ISO-bestand (vereist
  2352-byte brondata); de GUI weigert dit met uitleg.

## Structuur

| Bestand | Inhoud |
| --- | --- |
| `src/main.rs` | eframe-bootstrap |
| `src/app.rs` | applicatiestatus, event-verwerking, paneelindeling |
| `src/ffi.rs` | ruwe FFI: `#[repr(C)]`-types uit `libburn.h` + dynamisch laden via `libloading` |
| `src/isofs.rs` | ruwe FFI voor libisofs (`libisofs.so.6`): ISO9660-image samenstellen als burn_source |
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
- [x] **Stap 4 — Branden**: ISO-bestand branden via `burn_disc_write` met
  disc/session/track-model, file-bron + 4 MiB FIFO (`burn_fifo_source_new`),
  write-opts (snelheid via `burn_drive_set_speed`, TAO/SAO/auto via
  `burn_write_opts_auto_write_type`, simulatie, multi-session, padding,
  overburn/force, underrun-proof), optioneel wissen vooraf
  (`burn_disc_erase` + `burn_drive_re_assess`), voortgang met sector/snelheid/
  buffer/FIFO-vulling, annuleren via `burn_drive_cancel`, resultaatcheck via
  `burn_drive_wrote_well`, en libburn-meldingen (`burn_msgs_*`) live in het
  logpaneel.
- [x] **Stap 5 — Data-disc samenstellen**: bestanden/mappen kiezen in de GUI
  (met bestands/map-picker, duplicaatfilter en grootte-schatting) en als
  ISO9660-image op schijf branden via **libisofs** (`libisofs.so.6`,
  runtime-loading zoals libburn; Rock Ridge + Joliet, ISO-niveau 3).
  `iso_image_create_burn_source()` levert een burn_source die direct aan de
  libburn-track wordt gekoppeld — geen tussenbestand. De brand-flow is
  gedeeld met ISO-branden (grab/media-check/wissen, write-opts, poll-lus).
  Daarnaast: fase-weergave bij het branden (lead-out/track-afsluiting wordt
  zichtbaar na 100%), compacte apparaatinfo (technische details inklapbaar),
  file pickers via `rfd`, en de “eilanden”-herindeling van het centrale
  paneel (Station & media / Branden / Schijfkopie).
- [x] **Stap 6 — Wissen/formatteren**: losse 🧽-acties in het media-eiland:
  wissen (snel/volledig) voor CD-RW en DVD-RW sequentieel, formatteren
  (`burn_disc_format`, standaardgrootte) voor DVD-RW/DVD+RW/DVD-RAM/BD-RE.
  Slimme knop-activering per profiel (overschrijfbare media → formatteren
  i.p.v. wissen), voortgang in log én UI, annuleren, en de wis-flow van de
  brand-flow deelt nu één gedeelde wachtlus.
- [x] **Stap 7 — Multi-session met import (schrijf-eenmalige media)**:
  bestanden toevoegen aan een beschreven BD-R/DVD-R/DVD+R/CD-R met behoud van
  de bestaande inhoud. De bestaande sessie wordt geïmporteerd
  (`iso_data_source_new_from_file` + `iso_image_import` met start-blok uit de
  TOC), nieuwe bestanden komen in dezelfde boom, en de nieuwe sessie wordt
  geschreven op het NWA met `appendable` + `ms_block` — de oude bestandsdata
  blijft fysiek intact. De data-bron blijft alive tot na de brand (oude
  bestandsdata wordt tijdens het schrijven van de schijf gelezen). Alleen
  voor schrijf-eenmalige media; DVD+RW/BD-RE gebruiken overschrijven.
- [x] **Stap 8 — Instellingen persistent opslaan + uitbreidingen**:
  alle instellingen, exclusief-openen, bronkeuze en laatst-gebruikte paden
  worden via eframe-persistence bewaard en bij de start geladen. Daarnaast:
  **mediacode-weergave** (`burn_disc_get_media_id` + fabrikant-schatting via
  `burn_guess_manufacturer`, bijv. “PHILIP R04 — Philips”), **defect
  management**-status bij BD-media (`burn_disc_get_bd_spare_info`) en een
  vinkje om DM bij het formatteren uit te schakelen (bit5, sneller branden),
  een **snelheden-combobox** gevuld met de snelheden uit de media-inspectie,
  en **eject-diagnostiek**: bij overschrijfbare media een settle-pauze vóór
  het eject-verzoek (achtergrondformattering) met logfeedback.
- [x] **Stap 9 — Afwerking (deels, rest verplaatst)**:
  - **Sessielogbestand** (foutopsporing): elke sessie schrijft alle logregels
    met volledige tijdstempel naar `~/.local/share/libburn_gui/logs/`;
    knop “💾 Opslaan…” in het logpaneel.
  - **Meerdere stations tegelijk**: de worker polt alle actieve jobs
    niet-blokkerend; brand/wis/format op verschillende stations draait
    parallel, elk met eigen voortgang/LED/annuleren. Lezen blijft globaal
    sequentieel.
  - Beperking gedocumenteerd in de UI: bij meerdere stations geldt dezelfde
    bron (bestanden/ISO) en dezelfde instellingen voor gelijktijdige brands.
  - Presets/favorieten: geschrapt (instellingen zijn al persistent).
- [ ] **Stap 10 — Volgende thread**: i18n (typed tekstcatalogus + taalkeuze,
  NL brontaal, EN/DE als vertaling), .deb-pakket (Depends: libburn4,
  libisofs6 — libisoburn1 niet nodig), en kleine GUI-aanpassingen.

## Tests

```sh
cargo test
```

De rooktests laden de echte systeem-library en controleren
`burn_initialize`/`burn_version` en de volledige scan-flow (incl. de
struct-layout van `burn_drive_info`). Zonder libburn op het systeem slaan ze
stilletjes over.