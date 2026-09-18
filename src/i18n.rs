//! Typed tekstcatalogus voor i18n (stap 10b).
//!
//! Ontwerp: één `Lang`-enum voor de taalkeuze en per taal een `const`-exemplaar
//! van `Texts` — een struct met vaste velden, onderverdeeld in substructs per
//! UI-gebied. De compiler garandeert dat elke taal élke tekst invult: een
//! nieuw veld geeft compilefouten in alle talen die het nog missen. In de UI
//! werken we met `let t = app.lang.texts();` en dan `t.<gebied>.<veld>`.
//!
//! De catalogus groeit per deel-stap mee met de migratie van de panels: nu
//! `top_bar` en `log`; daarna settings, details, drives en de logmeldingen
//! uit app/worker. Terminologie: `docs/i18n-glossary.md` — en-US is de
//! brontaal, de bestaande Nederlandse teksten zijn 1-op-1 de nl-NL-inhoud.

use crate::ffi::{DiscStatus, DriveStatus};
use crate::logger::Level;
use crate::settings::MultiSession;
use crate::worker::MaintKind;
use serde::{Deserialize, Serialize};

/// Interfacetaal. De serde-namen volgen de locale-conventie zodat het
/// opslagbestand leesbaar blijft.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Lang {
    #[serde(rename = "en-US")]
    EnUs,
    /// Default zolang de migratie loopt: de bestaande UI-teksten zijn Nederlands.
    #[default]
    NlNl,
    #[serde(rename = "de-DE")]
    DeDe,
}

impl Lang {
    /// Alle talen, in de volgorde van de keuzelijst.
    pub const ALL: [Lang; 3] = [Lang::EnUs, Lang::NlNl, Lang::DeDe];

    /// Eigen naam van de taal — taalneutraal, dus bruikbaar in de keuzelijst
    /// ongeacht de actieve taal.
    pub fn endonym(self) -> &'static str {
        match self {
            Lang::EnUs => "English",
            Lang::NlNl => "Nederlands",
            Lang::DeDe => "Deutsch",
        }
    }

    /// De tekstcatalogus van deze taal (statisch, geen runtime-kosten).
    pub fn texts(self) -> &'static Texts {
        match self {
            Lang::EnUs => &EN_US,
            Lang::NlNl => &NL_NL,
            Lang::DeDe => &DE_DE,
        }
    }
}

/// Tekstcatalogus: per UI-gebied een substruct. Velden zijn `&'static str`;
/// regels met variabelen worden in de UI met `format!` samengevoegd.
pub struct Texts {
    pub common: CommonTexts,
    pub top_bar: TopBarTexts,
    pub log: LogTexts,
    pub settings: SettingsTexts,
    pub drives: DrivesTexts,
    pub details: DetailsTexts,
    pub media: MediaTexts,
    pub burn: BurnTexts,
    pub image: DiscImageTexts,
    pub maint: MaintTexts,
    pub app_log: AppLogTexts,
    // Groeit mee met de migratie: logmeldingen (worker), help.
}

/// Gedeelde teksten over meerdere panels heen.
pub struct CommonTexts {
    /// Annuleerknop (lezen, branden, onderhoud).
    pub cancel: &'static str,
    /// Bestandskiezer-knop.
    pub choose: &'static str,
    /// Prefix "Status" vóór een LED-tip.
    pub status_prefix: &'static str,
    pub led_idle: &'static str,
    pub led_busy: &'static str,
    pub led_ok: &'static str,
    pub led_error: &'static str,
}

/// Centraal paneel: libburn-status, stationdetails en mogelijkheden.
pub struct DetailsTexts {
    pub loading: &'static str,
    pub select_hint: &'static str,
    pub lib_failed_title: &'static str,
    pub lib_failed_body: &'static str,
    pub lib_failed_install: &'static str,
    pub lib_failed_manual: &'static str,
    pub lib_failed_env: &'static str,
    pub path_label: &'static str,
    pub reload: &'static str,
    pub tech_details: &'static str,
    /// Prefix "Firmware-revisie" — de UI plakt ": {}" erachter.
    pub firmware_prefix: &'static str,
    pub buffer: &'static str,
    pub tao_blocks: &'static str,
    pub sao_blocks: &'static str,
    pub raw_blocks: &'static str,
    pub packet_blocks: &'static str,
    pub capabilities: &'static str,
    pub caps_read: &'static str,
    pub caps_write: &'static str,
    /// Chip "C2-fouten" in de lees-mogelijkheden.
    pub caps_c2: &'static str,
    /// Chip "simulatie" in de schrijf-mogelijkheden.
    pub caps_simulate: &'static str,
}

/// Media-kaart: status, info-grid, snelheden en TOC.
pub struct MediaTexts {
    pub title: &'static str,
    pub inspecting: &'static str,
    pub inspect_btn: &'static str,
    pub waiting_for_scan: &'static str,
    pub not_inspected: &'static str,
    /// Prefix "Media" vóór de disc-status.
    pub media_prefix: &'static str,
    /// Prefix "Station" vóór de drive-status.
    pub drive_prefix: &'static str,
    pub unknown: &'static str,
    // Capaciteitsweergave.
    pub cap_blank_formatted: &'static str,
    pub cap_blocks_suffix: &'static str,
    pub cap_no_disc: &'static str,
    pub cap_nothing_written: &'static str,
    pub cap_unsuitable: &'static str,
    pub cap_no_data: &'static str,
    // Defect management (BD).
    dm_active: &'static str,
    dm_free: &'static str,
    dm_of: &'static str,
    dm_off: &'static str,
    // Info-grid.
    pub type_label: &'static str,
    pub media_code: &'static str,
    pub rewritable: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
    pub book_type: &'static str,
    pub readable: &'static str,
    pub readable_hover: &'static str,
    pub defect_mgmt: &'static str,
    // Snelhedentabel.
    speeds_title: &'static str,
    speeds_entries: &'static str,
    pub col_source: &'static str,
    pub col_profile: &'static str,
    pub col_write: &'static str,
    pub col_read: &'static str,
    pub col_end_lba: &'static str,
    // TOC.
    pub toc_blank: &'static str,
    pub toc_none: &'static str,
    toc_title: &'static str,
    toc_sessions: &'static str,
    toc_tracks: &'static str,
    pub col_session: &'static str,
    pub col_track: &'static str,
    pub col_type: &'static str,
    pub col_start_lba: &'static str,
    pub col_blocks: &'static str,
    pub col_size: &'static str,
    pub toc_data_copy: &'static str,
    pub toc_data: &'static str,
    pub toc_audio_copy: &'static str,
    pub toc_audio: &'static str,
    /// Woord "sessie" in het sessiebereik ("sessie 2 — LBA …").
    pub toc_session_word: &'static str,
    /// Tekst achter "⚠ {n} …" voor onvolledige sessies.
    pub toc_incomplete: &'static str,
    /// Disc-statuslabels (kaart-stijl, met hoofdletter).
    disc_unready: &'static str,
    disc_blank: &'static str,
    disc_empty: &'static str,
    disc_appendable: &'static str,
    disc_full: &'static str,
    disc_ungrabbed: &'static str,
    disc_unsuitable: &'static str,
    /// Drive-statuslabels achter "Station: ".
    ds_idle: &'static str,
    ds_spawning: &'static str,
    ds_reading: &'static str,
    ds_writing: &'static str,
    ds_writing_leadin: &'static str,
    ds_writing_leadout: &'static str,
    ds_erasing: &'static str,
    ds_grabbing: &'static str,
    ds_writing_pregap: &'static str,
    ds_closing_track: &'static str,
    ds_closing_session: &'static str,
    ds_formatting: &'static str,
    ds_reading_sync: &'static str,
    ds_writing_sync: &'static str,
    /// Bronlabels van de snelhedentabel.
    src_mode_page: &'static str,
    src_get_performance: &'static str,
    src_get_performance_read: &'static str,
    src_other: &'static str,
}

impl MediaTexts {
    /// Disc-status als gekleurde kaarttekst (hoofdletter, card-stijl).
    pub fn disc_status_label(&self, s: DiscStatus) -> &'static str {
        match s {
            DiscStatus::Unready => self.disc_unready,
            DiscStatus::Blank => self.disc_blank,
            DiscStatus::Empty => self.disc_empty,
            DiscStatus::Appendable => self.disc_appendable,
            DiscStatus::Full => self.disc_full,
            DiscStatus::Ungrabbed => self.disc_ungrabbed,
            DiscStatus::Unsuitable => self.disc_unsuitable,
            DiscStatus::Unknown(_) => self.unknown,
        }
    }

    /// Drive-status als tekst achter "Station: ".
    pub fn drive_status_label(&self, s: DriveStatus) -> &'static str {
        match s {
            DriveStatus::Idle => self.ds_idle,
            DriveStatus::Spawning => self.ds_spawning,
            DriveStatus::Reading => self.ds_reading,
            DriveStatus::Writing => self.ds_writing,
            DriveStatus::WritingLeadin => self.ds_writing_leadin,
            DriveStatus::WritingLeadout => self.ds_writing_leadout,
            DriveStatus::Erasing => self.ds_erasing,
            DriveStatus::Grabbing => self.ds_grabbing,
            DriveStatus::WritingPregap => self.ds_writing_pregap,
            DriveStatus::ClosingTrack => self.ds_closing_track,
            DriveStatus::ClosingSession => self.ds_closing_session,
            DriveStatus::Formatting => self.ds_formatting,
            DriveStatus::ReadingSync => self.ds_reading_sync,
            DriveStatus::WritingSync => self.ds_writing_sync,
            DriveStatus::Other(_) => self.unknown,
        }
    }

    /// Bron van een snelheidsdescriptor.
    pub fn speed_source_label(&self, source: i32) -> &'static str {
        match source {
            1 => self.src_mode_page,
            2 => self.src_get_performance,
            3 => self.src_get_performance_read,
            _ => self.src_other,
        }
    }

    /// Kop van de snelhedentabel, bijv. "Snelheden (3 entrees)".
    pub fn speeds_header(&self, entries: usize) -> String {
        format!(
            "{} ({} {})",
            self.speeds_title, entries, self.speeds_entries
        )
    }

    /// Kop van de TOC, bijv. "Inhoud (TOC) — 2 sessie(s), 4 track(s)".
    pub fn toc_header(&self, sessions: usize, tracks: usize) -> String {
        format!(
            "{} — {} {}, {} {}",
            self.toc_title, sessions, self.toc_sessions, tracks, self.toc_tracks
        )
    }

    /// Defect-management-waarde voor de kaart.
    pub fn dm_label(&self, spare: Option<(i32, i32)>) -> String {
        match spare {
            Some((alloc, free)) => format!(
                "{} ({} {free} {} {alloc})",
                self.dm_active, self.dm_free, self.dm_of
            ),
            None => self.dm_off.to_string(),
        }
    }
}

/// Bovenbalk.
pub struct TopBarTexts {
    /// libburn wordt geladen.
    pub loading: &'static str,
    /// libburn kon niet geladen worden.
    pub not_loaded: &'static str,
    /// Scan loopt (knop uit).
    pub scanning: &'static str,
    /// Scan-knop.
    pub scan: &'static str,
}

/// Logpaneel.
pub struct LogTexts {
    pub title: &'static str,
    /// "toon:" vóór de filter-schakelaars.
    pub show: &'static str,
    pub filter_info: &'static str,
    pub filter_ok: &'static str,
    pub filter_warning: &'static str,
    pub filter_error: &'static str,
    pub clear: &'static str,
    pub save: &'static str,
    pub save_hover: &'static str,
    pub auto_scroll: &'static str,
    /// Prefix vóór het pad, bijv. "Sessielog: /pad/naar/bestand".
    pub session_log: &'static str,
    /// Prefix van de logregel na een geslaagde save.
    pub saved_prefix: &'static str,
    /// Prefix van de logregel bij een mislukte save.
    pub save_failed_prefix: &'static str,
    /// Kolomtag per niveau in de schermlog. Het sessielogbestand blijft
    /// taalonafhankelijk en gebruikt `Level::tag`.
    tag_info: &'static str,
    tag_ok: &'static str,
    tag_warning: &'static str,
    tag_error: &'static str,
}

impl LogTexts {
    /// Label van de filter-schakelaar bij een logniveau.
    pub fn filter_label(&self, level: Level) -> &'static str {
        match level {
            Level::Info => self.filter_info,
            Level::Success => self.filter_ok,
            Level::Warning => self.filter_warning,
            Level::Error => self.filter_error,
        }
    }

    /// Kolomtag van een logniveau in de schermlog.
    pub fn level_tag(&self, level: Level) -> &'static str {
        match level {
            Level::Info => self.tag_info,
            Level::Success => self.tag_ok,
            Level::Warning => self.tag_warning,
            Level::Error => self.tag_error,
        }
    }
}

/// Rechterpaneel met de brand- en apparaatinstellingen.
pub struct SettingsTexts {
    pub title: &'static str,
    pub burn_header: &'static str,
    pub speed: &'static str,
    pub speed_desc: &'static str,
    /// Kop "Schrijfmodus"; de keuzelabels (Auto/TAO/SAO/RAW) zijn
    /// taalneutrale technische termen en blijven in `settings.rs`.
    pub write_mode: &'static str,
    pub simulate: &'static str,
    pub overburn: &'static str,
    pub underrun: &'static str,
    pub multi_session: &'static str,
    pub multi_session_hover: &'static str,
    ms_no: &'static str,
    ms_yes: &'static str,
    pub ms_not_applicable: &'static str,
    pub general: &'static str,
    pub padding: &'static str,
    pub padding_hover: &'static str,
    pub keep_timestamps: &'static str,
    pub keep_timestamps_hover: &'static str,
    pub eject_after: &'static str,
    pub eject_after_hover: &'static str,
    pub device_header: &'static str,
    pub exclusive_open: &'static str,
    /// Prefix "Huidige modus" — de UI plakt ": {}" erachter.
    pub current_mode: &'static str,
    mode_exclusive: &'static str,
    mode_non_exclusive: &'static str,
    pub exclusive_hint: &'static str,
    pub exclusive_reload: &'static str,
    /// Voetnoot onder de instellingen.
    pub apply_hint: &'static str,
}

impl SettingsTexts {
    /// Label van de Multi-session-keuze.
    pub fn ms_label(&self, ms: MultiSession) -> &'static str {
        match ms {
            MultiSession::No => self.ms_no,
            MultiSession::Yes => self.ms_yes,
        }
    }

    /// Label van de huidige apparaat-openingsmodus.
    pub fn mode_label(&self, exclusive: bool) -> &'static str {
        if exclusive {
            self.mode_exclusive
        } else {
            self.mode_non_exclusive
        }
    }
}

/// Linkerpaneel met de stationslijst.
pub struct DrivesTexts {
    pub title: &'static str,
    /// Voor stations zonder naam: "Station 1" / "Drive 1" / "Laufwerk 1".
    pub unnamed_prefix: &'static str,
    pub scanning: &'static str,
    /// Prefix "Scan mislukt" — de UI plakt ": {error}" erachter.
    pub scan_failed_prefix: &'static str,
    /// Hint bij geen stations + exclusief openen + eerdere scan vond wél stations.
    pub hint_automount: &'static str,
    /// Hint bij geen stations + exclusief openen (geen eerdere vondst).
    pub hint_none_exclusive: &'static str,
    /// Hint bij geen stations zonder exclusief openen.
    pub hint_none: &'static str,
    /// libburn is nog niet geladen.
    pub not_loaded: &'static str,
    /// Tussen haakjes onder de naam als het adres onbekend is.
    pub address_unknown: &'static str,
}

/// Eiland 2: branden (bronkeuze, snelheid, voortgang).
pub struct BurnTexts {
    pub island_header: &'static str,
    /// Startknop bij ISO-bron.
    pub burn_btn: &'static str,
    /// Startknop bij bestandsbron.
    pub compose_burn_btn: &'static str,
    pub speed_label: &'static str,
    pub speed_max: &'static str,
    /// Prefix "max." vóór "{} kB/s".
    pub speed_max_prefix: &'static str,
    pub source_label: &'static str,
    pub source_iso: &'static str,
    pub source_files: &'static str,
    pub multi_drive_hint: &'static str,
    /// Filternaam in de bestandskiezer.
    pub iso_filter: &'static str,
    /// Filternaam in de bestandskiezer.
    pub all_files_filter: &'static str,
    pub add_files: &'static str,
    pub add_folder: &'static str,
    pub no_files: &'static str,
    /// Woord "schatting" in de grootte-indicatie.
    pub estimate_word: &'static str,
    /// Woord "item(s)" in de grootte-indicatie.
    pub items_word: &'static str,
    pub volume_label: &'static str,
    pub compose_hint: &'static str,
    pub ms_import_hint: &'static str,
    pub inspect_first: &'static str,
    pub simulate_warn: &'static str,
    pub uses_settings_hint: &'static str,
    // Voortgang.
    /// "100% geschreven — drive is nog bezig:" — de UI plakt " {phase}…" aan.
    pub done_phase_prefix: &'static str,
    /// "Fase" — de UI plakt ": {}" aan.
    pub phase_prefix: &'static str,
    pub sector_word: &'static str,
    pub buffer_word: &'static str,
    pub elapsed_word: &'static str,
    pub simulate_banner: &'static str,
    pub other_drive_burn: &'static str,
}

/// Eiland 3: schijfkopie lezen.
pub struct DiscImageTexts {
    pub island_header: &'static str,
    pub make_copy_btn: &'static str,
    pub copy_hint: &'static str,
    pub other_drive_read: &'static str,
    /// Suffix in "123 / 456 blokken".
    pub blocks_suffix: &'static str,
    /// Standaardbestandsnaam in de opsladialoog.
    pub default_file: &'static str,
}

/// Onderhoud: wissen en de herstelpoging.
pub struct MaintTexts {
    pub title: &'static str,
    pub other_drive_maint: &'static str,
    pub erase_quick: &'static str,
    pub erase_full: &'static str,
    pub restore_header: &'static str,
    pub restore_explain: &'static str,
    pub restore_start_btn: &'static str,
    pub options_label: &'static str,
    pub dm_enable: &'static str,
    pub dm_enable_hover: &'static str,
    pub cert_enable: &'static str,
    pub cert_enable_hover: &'static str,
    pub overwritable_hint: &'static str,
    pub not_rewritable_hint: &'static str,
    /// In "Media …" tijdens de job: "wissen" / "erasing" / "wird gelöscht".
    kind_erase: &'static str,
    kind_restore: &'static str,
}

impl MaintTexts {
    /// Label van een lopende onderhoudsjob.
    pub fn kind_label(&self, kind: MaintKind) -> &'static str {
        match kind {
            MaintKind::Erase => self.kind_erase,
            MaintKind::Format => self.kind_restore,
        }
    }
}

/// Logmeldingen die de App zelf pusht (levenscyclus, verzoeken, events).
/// Meldingen met variabelen bestaan uit een prefix-veld; de aanroepplek
/// plakt de argumenten er met `format!` achter — zo blijft de woordvolgorde
/// per taal vrij (Duits zet de werkwoordsvorm achteraan).
pub struct AppLogTexts {
    pub started: &'static str,
    pub settings_loaded: &'static str,
    pub worker_stopped: &'static str,
    pub scan_requested: &'static str,
    /// + " {index}"
    pub inspect_requested: &'static str,
    pub reloading: &'static str,
    pub rescan_after_reload: &'static str,
    pub auto_scan_started: &'static str,
    pub no_output_file: &'static str,
    /// + " {index} → `{path}`"
    pub read_requested: &'static str,
    pub cancel_read_requested: &'static str,
    pub no_iso_file: &'static str,
    /// + ": `{path}`"
    pub iso_missing: &'static str,
    /// + " {index} {with_word} `{path}`"
    pub burn_requested: &'static str,
    pub with_word: &'static str,
    pub cancel_burn_requested: &'static str,
    pub cancel_maint_requested: &'static str,
    /// + " {index} ({quick|full})"
    pub erase_requested: &'static str,
    pub quick_word: &'static str,
    pub full_word: &'static str,
    pub restore_requested: &'static str,
    pub no_burn_files: &'static str,
    /// + " {index} — {n} {items_word}, ≈ {size}"
    pub burn_data_requested: &'static str,
    pub files_cleared: &'static str,
    pub reinspect_after_maint: &'static str,
    pub media_cleared_after_eject: &'static str,
}

/// en-US: brontaal (terminologie volgens `docs/i18n-glossary.md`).
const EN_US: Texts = Texts {
    top_bar: TopBarTexts {
        loading: "loading libburn…",
        not_loaded: "libburn not loaded",
        scanning: "⏳ Scanning…",
        scan: "🔄 Scan",
    },
    log: LogTexts {
        title: "Log",
        show: "show:",
        filter_info: "info",
        filter_ok: "ok",
        filter_warning: "warning",
        filter_error: "error",
        clear: "Clear",
        save: "💾 Save…",
        save_hover: "Save the visible log lines to a file",
        auto_scroll: "auto-scroll",
        session_log: "Session log",
        saved_prefix: "Log saved",
        save_failed_prefix: "Saving log failed",
        tag_info: "INFO",
        tag_ok: "OK",
        tag_warning: "WARN",
        tag_error: "ERR",
    },
    settings: SettingsTexts {
        title: "Settings",
        burn_header: "🔥 Burning",
        speed: "Speed",
        speed_desc: "The drive burns at the highest speed allowed by media and drive.",
        write_mode: "Write mode",
        simulate: "Simulation (laser off)",
        overburn: "Allow overburn",
        underrun: "Buffer underrun protection",
        multi_session: "Multi-session",
        multi_session_hover: "Yes: add extra sessions after the burn — only useful on write-once media (CD-R, DVD±R, BD-R). Overwritable media always stays writable.",
        ms_no: "No — disc gets closed",
        ms_yes: "Yes — disc stays open (multi-session)",
        ms_not_applicable: "Not applicable: the media in the selected drive is overwritable and always stays writable.",
        general: "General",
        padding: "Padding (KiB):",
        padding_hover: "Writes this amount of zeros after the track data (tail). For data discs 0 is normal; a few KiB can help with certain old CD players or audio burns. Has no effect on multi-session.",
        keep_timestamps: "Preserve original file timestamps",
        keep_timestamps_hover: "On: files get their own date/time on the disc (Rock Ridge + directory records). Off: everything gets the recording time.",
        eject_after: "Eject disc after completion",
        eject_after_hover: "After a successful burn the disc is taken out of the drive. A mounted disc is unmounted first.",
        device_header: "💽 Device",
        exclusive_open: "Exclusive open (O_EXCL)",
        current_mode: "Current mode",
        mode_exclusive: "exclusive (O_EXCL)",
        mode_non_exclusive: "non-exclusive",
        exclusive_hint: "Turn off if the file manager mounts the disc (automount, e.g. Nemo/udisks2).",
        exclusive_reload: "Changing this reloads libburn and rescans.",
        apply_hint: "ℹ These values are applied when burning (🔥 Burning on the media card).",
    },
    drives: DrivesTexts {
        title: "Drives",
        unnamed_prefix: "Drive",
        scanning: "Scanning…",
        scan_failed_prefix: "Scan failed",
        hint_automount: "No drives found while “Exclusive open” is on. A mounted disc (automount) can block the drive — turn the mode off (Settings → Device) and scan again.",
        hint_none_exclusive: "No drives found. Possible causes: no burner connected; or a mounted disc blocks the drive while “Exclusive open” is on (turn the mode off in Settings → Device and scan again).",
        hint_none: "No drives found. Connect a burner and scan again.",
        not_loaded: "Drives will appear here once libburn is loaded.",
        address_unknown: "(address unknown)",
    },
    common: CommonTexts {
        cancel: "⏹ Cancel",
        choose: "📄 Choose…",
        status_prefix: "Status",
        led_idle: "idle",
        led_busy: "busy…",
        led_ok: "done — succeeded",
        led_error: "failed",
    },
    details: DetailsTexts {
        loading: "loading libburn…",
        select_hint: "Select a drive on the left to see its details.",
        lib_failed_title: "⚠ libburn could not be loaded",
        lib_failed_body: "This GUI uses the libburn installed on your system (runtime linking, not compiled in).",
        lib_failed_install: "• Install the runtime:  sudo apt install libburn4   (Debian/Ubuntu)",
        lib_failed_manual: "• Or point to a libburn shared object below (e.g. /usr/lib/x86_64-linux-gnu/libburn.so.4)",
        lib_failed_env: "• Or start with the environment variable:  LIBBURN_SO=/path/to/libburn.so.4",
        path_label: "Path:",
        reload: "Reload",
        tech_details: "⚙ Technical details",
        firmware_prefix: "Firmware revision",
        buffer: "Buffer",
        tao_blocks: "TAO block types",
        sao_blocks: "SAO block types",
        raw_blocks: "RAW block types",
        packet_blocks: "Packet block types",
        capabilities: "Capabilities",
        caps_read: "read:",
        caps_write: "write:",
        caps_c2: "C2 errors",
        caps_simulate: "simulation",
    },
    media: MediaTexts {
        title: "Media",
        inspecting: "Inspecting drive (grab → status → speeds → release)…",
        inspect_btn: "🔍 Inspect media",
        waiting_for_scan: "(waiting for the drive scan)",
        not_inspected: "Not inspected yet — click “Inspect media”.",
        media_prefix: "Media",
        drive_prefix: "Drive",
        unknown: "unknown",
        cap_blank_formatted: "formatted — still empty",
        cap_blocks_suffix: "blocks",
        cap_no_disc: "no disc",
        cap_nothing_written: "nothing written yet",
        cap_unsuitable: "unusable media",
        cap_no_data: "no readable data (e.g. CD audio)",
        dm_active: "active",
        dm_free: "free",
        dm_of: "of",
        dm_off: "off",
        type_label: "Type:",
        media_code: "Media code:",
        rewritable: "Rewritable:",
        yes: "yes",
        no: "no",
        book_type: "Book type:",
        readable: "Readable:",
        readable_hover: "The maximum readable from this disc via burn_read_data. On DVD/BD the drive reports the full formatted capacity even when the disc is still empty; on CD only the written area. Non-data media (e.g. CD audio) is not readable this way.",
        defect_mgmt: "Defect mgmt:",
        speeds_title: "Speeds",
        speeds_entries: "entries",
        col_source: "Source",
        col_profile: "Profile",
        col_write: "Write",
        col_read: "Read",
        col_end_lba: "End-LBA",
        toc_blank: "Blank media — no TOC yet.",
        toc_none: "No TOC available for this media.",
        toc_title: "Contents (TOC)",
        toc_sessions: "session(s)",
        toc_tracks: "track(s)",
        col_session: "Session",
        col_track: "Track",
        col_type: "Type",
        col_start_lba: "Start-LBA",
        col_blocks: "Blocks",
        col_size: "≈ Size",
        toc_data_copy: "data · copy ok",
        toc_data: "data",
        toc_audio_copy: "audio · copy ok",
        toc_audio: "audio",
        toc_session_word: "session",
        toc_incomplete: "incomplete session(s) present",
        disc_unready: "not yet known",
        disc_blank: "Blank — ready to be written",
        disc_empty: "No disc",
        disc_appendable: "Appendable — extra session possible",
        disc_full: "Full / closed (read-only)",
        disc_ungrabbed: "not grabbed (internal error)",
        disc_unsuitable: "Unusable media",
        ds_idle: "idle",
        ds_spawning: "starting up",
        ds_reading: "reading",
        ds_writing: "writing",
        ds_writing_leadin: "writing lead-in",
        ds_writing_leadout: "writing lead-out",
        ds_erasing: "erasing",
        ds_grabbing: "grabbing",
        ds_writing_pregap: "writing pregap",
        ds_closing_track: "closing track",
        ds_closing_session: "closing session",
        ds_formatting: "restore attempt",
        ds_reading_sync: "reading (sync)",
        ds_writing_sync: "writing (sync)",
        src_mode_page: "mode page 2Ah",
        src_get_performance: "GET PERFORMANCE",
        src_get_performance_read: "GET PERFORMANCE (read)",
        src_other: "other",
    },
    burn: BurnTexts {
        island_header: "🔥 Burning",
        burn_btn: "🔥 Burn",
        compose_burn_btn: "🔥 Compose & burn",
        speed_label: "Speed:",
        speed_max: "Maximum",
        speed_max_prefix: "max.",
        source_label: "Source:",
        source_iso: "ISO file",
        source_files: "Files",
        multi_drive_hint: "With multiple drives: the chosen source and settings go to every drive you start a burn on at the same time — different content per drive is not supported.",
        iso_filter: "ISO images",
        all_files_filter: "All files",
        add_files: "➕ Add files…",
        add_folder: "📁 Add folder…",
        no_files: "No files selected yet — add files or folders.",
        estimate_word: "estimate",
        items_word: "item(s)",
        volume_label: "Volume name:",
        compose_hint: "Builds an ISO9660 image (Rock Ridge + Joliet, ISO level 3) with libisofs and burns it directly — no intermediate file.",
        ms_import_hint: "ℹ Existing session will be imported — new files are added to the existing content.",
        inspect_first: "(inspect first; burning requires blank or appendable media)",
        simulate_warn: "⚠ Simulation is on, but this drive/media cannot simulate (all BD media, DVD-R DL and overwritable media can never simulate). Turn “Simulation” off in Settings → Burning — note: without simulation everything is written for real.",
        uses_settings_hint: "Uses the settings on the right: speed, write mode, simulation, multi-session, padding. Erase and the restore attempt are under Maintenance on the media island.",
        done_phase_prefix: "100% written — drive still busy:",
        phase_prefix: "Phase",
        sector_word: "sector",
        buffer_word: "buffer",
        elapsed_word: "⏱ elapsed",
        simulate_banner: "SIMULATION — nothing is written permanently",
        other_drive_burn: "A burn job is currently running on another drive.",
    },
    image: DiscImageTexts {
        island_header: "💾 Disc image (data)",
        make_copy_btn: "💾 Make copy",
        copy_hint: "Reads all data blocks (2048 B) into one file — suitable for CD/DVD/BD data, not for CD audio.",
        other_drive_read: "A copy is currently running on another drive.",
        blocks_suffix: "blocks",
        default_file: "copy.iso",
    },
    maint: MaintTexts {
        title: "Maintenance",
        other_drive_maint: "A maintenance job is running on another drive.",
        erase_quick: "🧽 Erase (quick)",
        erase_full: "🧽 Erase (full)",
        restore_header: "🛠 Restore attempt for this media (advanced)",
        restore_explain: "Attempts to re-format the media (MMC FORMAT UNIT) — meant as a rescue for discs in a bad state. The outcome depends on drive and firmware; a failed attempt is risk-free for the data, but can leave the disc temporarily unreadable (a power-cycle of the drive usually helps).",
        restore_start_btn: "🛠 Start restore attempt",
        options_label: "Options:",
        dm_enable: "Enable defect management",
        dm_enable_hover: "On: during the restore attempt the drive reserves spare areas and remaps bad blocks (recommended). Off: faster burning, no remapping — only on reliable media.",
        cert_enable: "Enable certification",
        cert_enable_hover: "On: during the restore attempt the drive verifies the whole surface — thorough and slow, and on some drives the reason the attempt fails. Off: quick format; the verification happens afterwards during the burn.",
        overwritable_hint: "Directly overwritable media does not need erasing — writing directly is enough. A restore attempt (above) is only needed in special cases.",
        not_rewritable_hint: "This media is not rewritable — erasing or a restore attempt is not possible.",
        kind_erase: "erasing",
        kind_restore: "restoring",
    },
    app_log: AppLogTexts {
        started: "Burnr started — loading libburn from the system…",
        settings_loaded: "Settings loaded from previous session",
        worker_stopped: "Worker thread has stopped",
        scan_requested: "Scan requested",
        inspect_requested: "Inspect requested for drive",
        reloading: "loading libburn (again)…",
        rescan_after_reload: "Automatic scan follows after the reload.",
        auto_scan_started: "Automatic scan after reload started",
        no_output_file: "No output file given",
        read_requested: "Disc copy requested for drive",
        cancel_read_requested: "Copy cancellation requested",
        no_iso_file: "No ISO file given",
        iso_missing: "ISO file does not exist",
        burn_requested: "Burn requested for drive",
        with_word: "with",
        cancel_burn_requested: "Burn cancellation requested",
        cancel_maint_requested: "Maintenance cancellation requested",
        erase_requested: "Erase requested for drive",
        quick_word: "quick",
        full_word: "full",
        restore_requested: "Restore attempt requested",
        no_burn_files: "No files selected to burn",
        burn_data_requested: "Data image burn requested for drive",
        files_cleared: "File list cleared after successful burn",
        reinspect_after_maint: "Automatic re-inspection after maintenance…",
        media_cleared_after_eject: "Media info cleared after eject",
    },
};

/// nl-NL: de oorspronkelijke UI-teksten 1-op-1 overgenomen (glossary:
/// "carried over into the catalogue, not re-translated").
const NL_NL: Texts = Texts {
    top_bar: TopBarTexts {
        loading: "libburn laden…",
        not_loaded: "libburn niet geladen",
        scanning: "⏳ Scannen…",
        scan: "🔄 Scannen",
    },
    log: LogTexts {
        title: "Log",
        show: "toon:",
        filter_info: "info",
        filter_ok: "ok",
        filter_warning: "waarschuwing",
        filter_error: "fout",
        clear: "Wissen",
        save: "💾 Opslaan…",
        save_hover: "Sla de zichtbare logregels op in een bestand",
        auto_scroll: "auto-scroll",
        session_log: "Sessielog",
        saved_prefix: "Log opgeslagen",
        save_failed_prefix: "Log opslaan mislukt",
        tag_info: "INFO",
        tag_ok: "OK",
        tag_warning: "WARN",
        tag_error: "FOUT",
    },
    settings: SettingsTexts {
        title: "Instellingen",
        burn_header: "🔥 Branden",
        speed: "Snelheid",
        speed_desc: "De drive brandt op de hoogste door media en drive toegelaten snelheid.",
        write_mode: "Schrijfmodus",
        simulate: "Simulatie (laser uit)",
        overburn: "Overburn toestaan",
        underrun: "Buffer-underrun-beveiliging",
        multi_session: "Multi-session",
        multi_session_hover: "Ja: na de brand extra sessies toevoegen — alleen zinvol op schrijf-eenmalige media (CD-R, DVD±R, BD-R). Overschrijfbare media blijft altijd beschrijfbaar.",
        ms_no: "Nee — schijf wordt afgesloten",
        ms_yes: "Ja — schijf blijft open (multi-session)",
        ms_not_applicable: "Niet van toepassing: de media in het geselecteerde station is overschrijfbaar en blijft altijd beschrijfbaar.",
        general: "Algemeen",
        padding: "Padding (KiB):",
        padding_hover: "Schrijft deze hoeveelheid nullen achter de trackdata (tail). Voor data-discs is 0 normaal; enkele KiB kan helpen bij bepaalde oude CD-spelers of audio-brandingen. Heeft geen invloed op multi-session.",
        keep_timestamps: "Originele bestandsdatums behouden",
        keep_timestamps_hover: "Aan: bestanden krijgen hun eigen datum/tijd op de schijf (Rock Ridge + directoryrecords). Uit: alles krijgt de opnametijd.",
        eject_after: "Schijf uitwerpen na afloop",
        eject_after_hover: "Na een geslaagde brand wordt de schijf uit de drive genomen. Een aangekoppelde schijf wordt eerst ge-unmount.",
        device_header: "💽 Apparaat",
        exclusive_open: "Exclusief openen (O_EXCL)",
        current_mode: "Huidige modus",
        mode_exclusive: "exclusief (O_EXCL)",
        mode_non_exclusive: "niet-exclusief",
        exclusive_hint: "Uitzetten als de bestandsbeheerder de schijf aankoppelt (automount, bijv. Nemo/udisks2).",
        exclusive_reload: "Wijzigen herlaadt libburn en scant opnieuw.",
        apply_hint: "ℹ Deze waarden worden toegepast bij het branden (🔥 Branden in het mediakaartje).",
    },
    drives: DrivesTexts {
        title: "Stations",
        unnamed_prefix: "Station",
        scanning: "Bezig met scannen…",
        scan_failed_prefix: "Scan mislukt",
        hint_automount: "Geen stations gevonden terwijl “Exclusief openen” aan staat. Een aangekoppelde schijf (automount) kan de drive blokkeren — zet de modus uit (Instellingen → Apparaat) en scan opnieuw.",
        hint_none_exclusive: "Geen stations gevonden. Mogelijke oorzaken: geen brander aangesloten; of een aangekoppelde schijf blokkeert de drive terwijl “Exclusief openen” aan staat (zet de modus uit in Instellingen → Apparaat en scan opnieuw).",
        hint_none: "Geen stations gevonden. Sluit een brander aan en scan opnieuw.",
        not_loaded: "Stations verschijnen hier zodra libburn geladen is.",
        address_unknown: "(adres onbekend)",
    },
    common: CommonTexts {
        cancel: "⏹ Annuleren",
        choose: "📄 Kies…",
        status_prefix: "Status",
        led_idle: "inactief",
        led_busy: "bezig…",
        led_ok: "klaar — geslaagd",
        led_error: "mislukt",
    },
    details: DetailsTexts {
        loading: "libburn wordt geladen…",
        select_hint: "Selecteer links een station om de details te zien.",
        lib_failed_title: "⚠ libburn kon niet geladen worden",
        lib_failed_body: "Deze GUI gebruikt de libburn die op je systeem staat (runtime-linking, niet meegecompileerd).",
        lib_failed_install: "• Installeer de runtime:  sudo apt install libburn4   (Debian/Ubuntu)",
        lib_failed_manual: "• Of wijs hieronder een libburn-shared object aan (bijv. /usr/lib/x86_64-linux-gnu/libburn.so.4)",
        lib_failed_env: "• Of start met de omgevingsvariabele:  LIBBURN_SO=/pad/naar/libburn.so.4",
        path_label: "Pad:",
        reload: "Opnieuw laden",
        tech_details: "⚙ Technische details",
        firmware_prefix: "Firmware-revisie",
        buffer: "Buffer",
        tao_blocks: "TAO-bloktypen",
        sao_blocks: "SAO-bloktypen",
        raw_blocks: "RAW-bloktypen",
        packet_blocks: "Packet-bloktypen",
        capabilities: "Mogelijkheden",
        caps_read: "lezen:",
        caps_write: "schrijven:",
        caps_c2: "C2-fouten",
        caps_simulate: "simulatie",
    },
    media: MediaTexts {
        title: "Media",
        inspecting: "Station inspecteren (grab → status → snelheden → release)…",
        inspect_btn: "🔍 Media inspecteren",
        waiting_for_scan: "(wacht op de stationscan)",
        not_inspected: "Nog niet geïnspecteerd — klik op “Media inspecteren”.",
        media_prefix: "Media",
        drive_prefix: "Station",
        unknown: "onbekend",
        cap_blank_formatted: "geformatteerd — nog leeg",
        cap_blocks_suffix: "blokken",
        cap_no_disc: "geen schijf",
        cap_nothing_written: "nog niets beschreven",
        cap_unsuitable: "onbruikbare media",
        cap_no_data: "geen leesbare data (bijv. CD-audio)",
        dm_active: "actief",
        dm_free: "vrij",
        dm_of: "van",
        dm_off: "uit",
        type_label: "Type:",
        media_code: "Mediacode:",
        rewritable: "Herbeschrijfbaar:",
        yes: "ja",
        no: "nee",
        book_type: "Book type:",
        readable: "Leesbaar:",
        readable_hover: "Wat via burn_read_data maximaal leesbaar is van deze schijf. Op DVD/BD meldt de drive de volledige geformatteerde capaciteit, ook als de schijf nog leeg is; op CD alleen het beschreven gebied. Niet-datamedia (bijv. CD-audio) is zo niet leesbaar.",
        defect_mgmt: "Defect mgmt:",
        speeds_title: "Snelheden",
        speeds_entries: "entrees",
        col_source: "Bron",
        col_profile: "Profiel",
        col_write: "Schrijven",
        col_read: "Lezen",
        col_end_lba: "End-LBA",
        toc_blank: "Lege media — nog geen TOC.",
        toc_none: "Geen TOC beschikbaar voor deze media.",
        toc_title: "Inhoud (TOC)",
        toc_sessions: "sessie(s)",
        toc_tracks: "track(s)",
        col_session: "Sessie",
        col_track: "Track",
        col_type: "Type",
        col_start_lba: "Start-LBA",
        col_blocks: "Blokken",
        col_size: "≈ Grootte",
        toc_data_copy: "data · kopie ok",
        toc_data: "data",
        toc_audio_copy: "audio · kopie ok",
        toc_audio: "audio",
        toc_session_word: "sessie",
        toc_incomplete: "onvolledige sessie(s) aanwezig",
        disc_unready: "nog niet bekend",
        disc_blank: "Leeg — klaar om te beschrijven",
        disc_empty: "Geen schijf",
        disc_appendable: "Onvolledig — extra sessie mogelijk",
        disc_full: "Vol / afgesloten (alleen lezen)",
        disc_ungrabbed: "niet gegrabbed (interne fout)",
        disc_unsuitable: "Onbruikbare media",
        ds_idle: "inactief",
        ds_spawning: "bezig met starten",
        ds_reading: "lezen",
        ds_writing: "schrijven",
        ds_writing_leadin: "lead-in schrijven",
        ds_writing_leadout: "lead-out schrijven",
        ds_erasing: "wissen",
        ds_grabbing: "grabben",
        ds_writing_pregap: "pregap schrijven",
        ds_closing_track: "track afsluiten",
        ds_closing_session: "sessie afsluiten",
        ds_formatting: "herstelpoging",
        ds_reading_sync: "synchroon lezen",
        ds_writing_sync: "synchroon schrijven",
        src_mode_page: "mode page 2Ah",
        src_get_performance: "GET PERFORMANCE",
        src_get_performance_read: "GET PERFORMANCE (lees)",
        src_other: "overig",
    },
    burn: BurnTexts {
        island_header: "🔥 Branden",
        burn_btn: "🔥 Branden",
        compose_burn_btn: "🔥 Samenstellen & branden",
        speed_label: "Snelheid:",
        speed_max: "Maximaal",
        speed_max_prefix: "max.",
        source_label: "Bron:",
        source_iso: "ISO-bestand",
        source_files: "Bestanden",
        multi_drive_hint: "Bij meerdere stations: de gekozen bron en instellingen gaan naar élle stations waar je tegelijk een brand start — per station verschillende inhoud wordt niet ondersteund.",
        iso_filter: "ISO-images",
        all_files_filter: "Alle bestanden",
        add_files: "➕ Bestanden…",
        add_folder: "📁 Map…",
        no_files: "Nog geen bestanden gekozen — voeg bestanden of mappen toe.",
        estimate_word: "schatting",
        items_word: "item(s)",
        volume_label: "Volume-naam:",
        compose_hint: "Maakt een ISO9660-image (Rock Ridge + Joliet, ISO-niveau 3) met libisofs en brandt die direct — geen tussenbestand.",
        ms_import_hint: "ℹ Bestaande sessie wordt geïmporteerd — nieuwe bestanden komen bij de bestaande inhoud.",
        inspect_first: "(inspecteer eerst; branden vereist lege of onvolledige media)",
        simulate_warn: "⚠ Simulatie staat aan, maar deze drive/media kan niet simuleren (alle BD-media, DVD-R DL en overbeschrijfbare media kunnen nooit simuleren). Zet “Simulatie” uit in Instellingen → Branden — let op: zonder simulatie wordt er écht geschreven.",
        uses_settings_hint: "Gebruikt de instellingen rechts: snelheid, schrijfmodus, simulatie, multi-session, padding. Wissen en de herstelpoging gaan via Onderhoud in het Media-eiland.",
        done_phase_prefix: "100% geschreven — drive is nog bezig:",
        phase_prefix: "Fase",
        sector_word: "sector",
        buffer_word: "buffer",
        elapsed_word: "⏱ verstreken",
        simulate_banner: "SIMULATIE — er wordt niets definitief geschreven",
        other_drive_burn: "Er draait momenteel een brandjob op een ander station.",
    },
    image: DiscImageTexts {
        island_header: "💾 Schijfkopie (data)",
        make_copy_btn: "💾 Kopie maken",
        copy_hint: "Leest alle datablokken (2048 B) naar één bestand — geschikt voor CD/DVD/BD-data, niet voor CD-audio.",
        other_drive_read: "Er draait momenteel een kopie op een ander station.",
        blocks_suffix: "blokken",
        default_file: "kopie.iso",
    },
    maint: MaintTexts {
        title: "Onderhoud",
        other_drive_maint: "Er draait een onderhoudsjob op een ander station.",
        erase_quick: "🧽 Wissen (snel)",
        erase_full: "🧽 Wissen (volledig)",
        restore_header: "🛠 Herstelpoging voor deze media (gevorderd)",
        restore_explain: "Probeert de media opnieuw te formatteren (MMC FORMAT UNIT) — bedoeld als redding voor schijven in een verkeerde toestand. De uitkomst hangt af van drive en firmware; een mislukte poging is risicoloos voor de data, maar kan de schijf tijdelijk onleesbaar maken (power-cycle van de drive helpt meestal).",
        restore_start_btn: "🛠 Herstelpoging starten",
        options_label: "Opties:",
        dm_enable: "Defect management activeren",
        dm_enable_hover: "Aan: de drive reserveert bij de herstelpoging spare-gebieden en hermapt slechte blokken (aanbevolen). Uit: sneller branden, geen hermapping — alleen op betrouwbare media.",
        cert_enable: "Certificering activeren",
        cert_enable_hover: "Aan: de drive controleert bij de herstelpoging het hele oppervlak — grondig en traag, en bij sommige drives de reden dat de poging faalt. Uit: snelformat; de controle gebeurt daarna alsnog tijdens het branden.",
        overwritable_hint: "Direct overschrijfbare media hoeft niet gewist te worden — direct beschrijven volstaat. Een herstelpoging (hierboven) is alleen nodig in bijzondere gevallen.",
        not_rewritable_hint: "Deze media is niet herbeschrijfbaar — wissen of een herstelpoging is niet mogelijk.",
        kind_erase: "wissen",
        kind_restore: "herstellen",
    },
    app_log: AppLogTexts {
        started: "Burnr gestart — libburn wordt van het systeem geladen…",
        settings_loaded: "Instellingen geladen uit vorige sessie",
        worker_stopped: "Worker-thread is gestopt",
        scan_requested: "Scan aangevraagd",
        inspect_requested: "Inspectie aangevraagd voor station",
        reloading: "libburn (opnieuw) laden…",
        rescan_after_reload: "Automatische scan volgt na het herladen.",
        auto_scan_started: "Automatische scan na herladen gestart",
        no_output_file: "Geen uitvoerbestand opgegeven",
        read_requested: "Schijfkopie aangevraagd voor station",
        cancel_read_requested: "Kopie annuleren aangevraagd",
        no_iso_file: "Geen ISO-bestand opgegeven",
        iso_missing: "ISO-bestand bestaat niet",
        burn_requested: "Brandjob aangevraagd voor station",
        with_word: "met",
        cancel_burn_requested: "Brandjob annuleren aangevraagd",
        cancel_maint_requested: "Onderhoudsjob annuleren aangevraagd",
        erase_requested: "Wissen aangevraagd voor station",
        quick_word: "snel",
        full_word: "volledig",
        restore_requested: "Herstelpoging aangevraagd",
        no_burn_files: "Geen bestanden gekozen om te branden",
        burn_data_requested: "Data-image branden aangevraagd voor station",
        files_cleared: "Bestandenlijst gewist na geslaagde brand",
        reinspect_after_maint: "Automatische herinspectie na onderhoud…",
        media_cleared_after_eject: "Mediagegevens gewist na eject",
    },
};

/// de-DE: eerste vertaling; technische termen (grab, NWA, FIFO, TOC, MMC)
/// blijven onvertaald, conform de glossary-stijlnotities.
const DE_DE: Texts = Texts {
    top_bar: TopBarTexts {
        loading: "libburn wird geladen…",
        not_loaded: "libburn nicht geladen",
        scanning: "⏳ Scannen…",
        scan: "🔄 Scannen",
    },
    log: LogTexts {
        title: "Log",
        show: "anzeigen:",
        filter_info: "Info",
        filter_ok: "OK",
        filter_warning: "Warnung",
        filter_error: "Fehler",
        clear: "Leeren",
        save: "💾 Speichern…",
        save_hover: "Sichtbare Logzeilen in einer Datei speichern",
        auto_scroll: "Auto-Scroll",
        session_log: "Sitzungslog",
        saved_prefix: "Log gespeichert",
        save_failed_prefix: "Log speichern fehlgeschlagen",
        tag_info: "INFO",
        tag_ok: "OK",
        tag_warning: "WARN",
        tag_error: "FEHLER",
    },
    settings: SettingsTexts {
        title: "Einstellungen",
        burn_header: "🔥 Brennen",
        speed: "Geschwindigkeit",
        speed_desc: "Das Laufwerk brennt mit der höchsten von Medium und Laufwerk erlaubten Geschwindigkeit.",
        write_mode: "Schreibmodus",
        simulate: "Simulation (Laser aus)",
        overburn: "Overburn erlauben",
        underrun: "Puffer-Underrun-Schutz",
        multi_session: "Multi-Session",
        multi_session_hover: "Ja: nach dem Brennen weitere Sessions hinzufügen — nur bei einmal beschreibbaren Medien sinnvoll (CD-R, DVD±R, BD-R). Wiederbeschreibbare Medien bleiben immer beschreibbar.",
        ms_no: "Nein — Disc wird abgeschlossen",
        ms_yes: "Ja — Disc bleibt offen (Multi-Session)",
        ms_not_applicable: "Nicht anwendbar: das Medium im ausgewählten Laufwerk ist wiederbeschreibbar und bleibt immer beschreibbar.",
        general: "Allgemein",
        padding: "Padding (KiB):",
        padding_hover: "Schreibt diese Menge Nullen hinter die Trackdaten (Tail). Bei Daten-Discs ist 0 normal; einige KiB können bei bestimmten alten CD-Spielern oder Audio-Brennvorgängen helfen. Hat keinen Einfluss auf Multi-Session.",
        keep_timestamps: "Originale Datei-Zeitstempel erhalten",
        keep_timestamps_hover: "Ein: Dateien behalten ihr eigenes Datum und ihre Uhrzeit auf der Disc (Rock Ridge + Verzeichniseinträge). Aus: alles bekommt die Aufnahmezeit.",
        eject_after: "Disc nach Abschluss auswerfen",
        eject_after_hover: "Nach erfolgreichem Brennen wird die Disc aus dem Laufwerk genommen. Eine eingehängte Disc wird zuerst ausgehängt.",
        device_header: "💽 Gerät",
        exclusive_open: "Exklusiv öffnen (O_EXCL)",
        current_mode: "Aktueller Modus",
        mode_exclusive: "exklusiv (O_EXCL)",
        mode_non_exclusive: "nicht-exklusiv",
        exclusive_hint: "Ausschalten, wenn der Dateimanager die Disc einhängt (Automount, z. B. Nemo/udisks2).",
        exclusive_reload: "Änderungen laden libburn neu und starten einen neuen Scan.",
        apply_hint: "ℹ Diese Werte werden beim Brennen angewendet (🔥 Brennen auf der Medienkarte).",
    },
    drives: DrivesTexts {
        title: "Laufwerke",
        unnamed_prefix: "Laufwerk",
        scanning: "Wird gescannt…",
        scan_failed_prefix: "Scan fehlgeschlagen",
        hint_automount: "Keine Laufwerke gefunden, während „Exklusiv öffnen“ aktiv ist. Eine eingehängte Disc (Automount) kann das Laufwerk blockieren — Modus ausschalten (Einstellungen → Gerät) und erneut scannen.",
        hint_none_exclusive: "Keine Laufwerke gefunden. Mögliche Ursachen: kein Brenner angeschlossen; oder eine eingehängte Disc blockiert das Laufwerk, während „Exklusiv öffnen“ aktiv ist (Modus in Einstellungen → Gerät ausschalten und erneut scannen).",
        hint_none: "Keine Laufwerke gefunden. Brenner anschließen und erneut scannen.",
        not_loaded: "Laufwerke erscheinen hier, sobald libburn geladen ist.",
        address_unknown: "(Adresse unbekannt)",
    },
    common: CommonTexts {
        cancel: "⏹ Abbrechen",
        choose: "📄 Auswählen…",
        status_prefix: "Status",
        led_idle: "inaktiv",
        led_busy: "beschäftigt…",
        led_ok: "fertig — erfolgreich",
        led_error: "fehlgeschlagen",
    },
    details: DetailsTexts {
        loading: "libburn wird geladen…",
        select_hint: "Wähle links ein Laufwerk, um die Details zu sehen.",
        lib_failed_title: "⚠ libburn konnte nicht geladen werden",
        lib_failed_body: "Diese GUI verwendet die auf deinem System installierte libburn (Runtime-Linking, nicht einkompiliert).",
        lib_failed_install: "• Runtime installieren:  sudo apt install libburn4   (Debian/Ubuntu)",
        lib_failed_manual: "• Oder unten ein libburn-Shared-Object angeben (z. B. /usr/lib/x86_64-linux-gnu/libburn.so.4)",
        lib_failed_env: "• Oder mit der Umgebungsvariablen starten:  LIBBURN_SO=/pfad/zu/libburn.so.4",
        path_label: "Pfad:",
        reload: "Neu laden",
        tech_details: "⚙ Technische Details",
        firmware_prefix: "Firmware-Revision",
        buffer: "Puffer",
        tao_blocks: "TAO-Blocktypen",
        sao_blocks: "SAO-Blocktypen",
        raw_blocks: "RAW-Blocktypen",
        packet_blocks: "Packet-Blocktypen",
        capabilities: "Fähigkeiten",
        caps_read: "lesen:",
        caps_write: "schreiben:",
        caps_c2: "C2-Fehler",
        caps_simulate: "Simulation",
    },
    media: MediaTexts {
        title: "Medium",
        inspecting: "Laufwerk wird inspiziert (grab → status → Geschwindigkeiten → release)…",
        inspect_btn: "🔍 Medium inspizieren",
        waiting_for_scan: "(wartet auf den Laufwerk-Scan)",
        not_inspected: "Noch nicht inspiziert — auf „Medium inspizieren“ klicken.",
        media_prefix: "Medium",
        drive_prefix: "Laufwerk",
        unknown: "unbekannt",
        cap_blank_formatted: "formatiert — noch leer",
        cap_blocks_suffix: "Blöcke",
        cap_no_disc: "keine Disc",
        cap_nothing_written: "noch nichts beschrieben",
        cap_unsuitable: "unbrauchbares Medium",
        cap_no_data: "keine lesbaren Daten (z. B. CD-Audio)",
        dm_active: "aktiv",
        dm_free: "frei",
        dm_of: "von",
        dm_off: "aus",
        type_label: "Typ:",
        media_code: "Mediacode:",
        rewritable: "Wiederbeschreibbar:",
        yes: "ja",
        no: "nein",
        book_type: "Book type:",
        readable: "Lesbar:",
        readable_hover: "Was über burn_read_data maximal von dieser Disc lesbar ist. Bei DVD/BD meldet das Laufwerk die volle formatierte Kapazität, auch wenn die Disc noch leer ist; bei CD nur den beschriebenen Bereich. Nicht-Datenmedien (z. B. CD-Audio) sind so nicht lesbar.",
        defect_mgmt: "Defekt-Mgmt:",
        speeds_title: "Geschwindigkeiten",
        speeds_entries: "Einträge",
        col_source: "Quelle",
        col_profile: "Profil",
        col_write: "Schreiben",
        col_read: "Lesen",
        col_end_lba: "End-LBA",
        toc_blank: "Leeres Medium — noch keine TOC.",
        toc_none: "Keine TOC für dieses Medium verfügbar.",
        toc_title: "Inhalt (TOC)",
        toc_sessions: "Session(s)",
        toc_tracks: "Track(s)",
        col_session: "Session",
        col_track: "Track",
        col_type: "Typ",
        col_start_lba: "Start-LBA",
        col_blocks: "Blöcke",
        col_size: "≈ Größe",
        toc_data_copy: "Daten · Kopie ok",
        toc_data: "Daten",
        toc_audio_copy: "Audio · Kopie ok",
        toc_audio: "Audio",
        toc_session_word: "Session",
        toc_incomplete: "unvollständige Session(s) vorhanden",
        disc_unready: "noch unbekannt",
        disc_blank: "Leer — bereit zum Beschreiben",
        disc_empty: "Keine Disc",
        disc_appendable: "Unvollständig — zusätzliche Session möglich",
        disc_full: "Voll / abgeschlossen (nur lesen)",
        disc_ungrabbed: "nicht gegrabbed (interner Fehler)",
        disc_unsuitable: "Unbrauchbares Medium",
        ds_idle: "inaktiv",
        ds_spawning: "startet gerade",
        ds_reading: "liest",
        ds_writing: "schreibt",
        ds_writing_leadin: "Lead-in wird geschrieben",
        ds_writing_leadout: "Lead-out wird geschrieben",
        ds_erasing: "wird gelöscht",
        ds_grabbing: "wird gegrabbed",
        ds_writing_pregap: "Pregap wird geschrieben",
        ds_closing_track: "Track wird abgeschlossen",
        ds_closing_session: "Session wird abgeschlossen",
        ds_formatting: "Herstellungsversuch",
        ds_reading_sync: "synchrones Lesen",
        ds_writing_sync: "synchrones Schreiben",
        src_mode_page: "Mode Page 2Ah",
        src_get_performance: "GET PERFORMANCE",
        src_get_performance_read: "GET PERFORMANCE (lesen)",
        src_other: "sonstige",
    },
    burn: BurnTexts {
        island_header: "🔥 Brennen",
        burn_btn: "🔥 Brennen",
        compose_burn_btn: "🔥 Zusammenstellen & brennen",
        speed_label: "Geschwindigkeit:",
        speed_max: "Maximal",
        speed_max_prefix: "max.",
        source_label: "Quelle:",
        source_iso: "ISO-Datei",
        source_files: "Dateien",
        multi_drive_hint: "Bei mehreren Laufwerken: die gewählte Quelle und die Einstellungen gehen an alle Laufwerke, auf denen du gleichzeitig brennst — unterschiedliche Inhalte pro Laufwerk werden nicht unterstützt.",
        iso_filter: "ISO-Images",
        all_files_filter: "Alle Dateien",
        add_files: "➕ Dateien hinzufügen…",
        add_folder: "📁 Ordner hinzufügen…",
        no_files: "Noch keine Dateien ausgewählt — Dateien oder Ordner hinzufügen.",
        estimate_word: "Schätzung",
        items_word: "Element(e)",
        volume_label: "Volume-Name:",
        compose_hint: "Erstellt ein ISO9660-Image (Rock Ridge + Joliet, ISO-Level 3) mit libisofs und brennt es direkt — ohne Zwischendatei.",
        ms_import_hint: "ℹ Vorhandene Session wird importiert — neue Dateien kommen zum bestehenden Inhalt hinzu.",
        inspect_first: "(zuerst inspizieren; Brennen erfordert leere oder appendable Medien)",
        simulate_warn: "⚠ Simulation ist aktiv, aber dieses Laufwerk/Medium kann nicht simulieren (alle BD-Medien, DVD-R DL und wiederbeschreibbare Medien können nie simulieren). Schalte „Simulation“ in Einstellungen → Brennen aus — Achtung: ohne Simulation wird wirklich geschrieben.",
        uses_settings_hint: "Verwendet die Einstellungen rechts: Geschwindigkeit, Schreibmodus, Simulation, Multi-Session, Padding. Löschen und der Herstellungsversuch laufen über Wartung auf der Medieninsel.",
        done_phase_prefix: "100% geschrieben — Laufwerk ist noch beschäftigt:",
        phase_prefix: "Phase",
        sector_word: "Sektor",
        buffer_word: "Puffer",
        elapsed_word: "⏱ vergangen",
        simulate_banner: "SIMULATION — es wird nichts endgültig geschrieben",
        other_drive_burn: "Auf einem anderen Laufwerk läuft gerade ein Brennauftrag.",
    },
    image: DiscImageTexts {
        island_header: "💾 Disc-Image (Daten)",
        make_copy_btn: "💾 Kopie erstellen",
        copy_hint: "Liest alle Datenblöcke (2048 B) in eine Datei — geeignet für CD/DVD/BD-Daten, nicht für CD-Audio.",
        other_drive_read: "Auf einem anderen Laufwerk läuft gerade eine Kopie.",
        blocks_suffix: "Blöcke",
        default_file: "Kopie.iso",
    },
    maint: MaintTexts {
        title: "Wartung",
        other_drive_maint: "Auf einem anderen Laufwerk läuft ein Wartungsauftrag.",
        erase_quick: "🧽 Löschen (schnell)",
        erase_full: "🧽 Löschen (vollständig)",
        restore_header: "🛠 Herstellungsversuch für dieses Medium (fortgeschritten)",
        restore_explain: "Versucht, das Medium neu zu formatieren (MMC FORMAT UNIT) — gedacht als Rettung für Discs in einem falschen Zustand. Das Ergebnis hängt von Laufwerk und Firmware ab; ein fehlgeschlagener Versuch ist für die Daten risikolos, kann die Disc aber vorübergehend unlesbar machen (ein Power-Cycle des Laufwerks hilft meistens).",
        restore_start_btn: "🛠 Herstellungsversuch starten",
        options_label: "Optionen:",
        dm_enable: "Defect Management aktivieren",
        dm_enable_hover: "Ein: das Laufwerk reserviert beim Herstellungsversuch Spare-Bereiche und remappt schlechte Blöcke (empfohlen). Aus: schnelleres Brennen, kein Remapping — nur auf zuverlässigen Medien.",
        cert_enable: "Zertifizierung aktivieren",
        cert_enable_hover: "Ein: das Laufwerk prüft beim Herstellungsversuch die gesamte Oberfläche — gründlich und langsam, und bei manchen Laufwerken der Grund, warum der Versuch fehlschlägt. Aus: Schnellformat; die Prüfung erfolgt danach beim Brennen.",
        overwritable_hint: "Direkt wiederbeschreibbare Medien müssen nicht gelöscht werden — direktes Beschreiben genügt. Ein Herstellungsversuch (oben) ist nur in Sonderfällen nötig.",
        not_rewritable_hint: "Dieses Medium ist nicht wiederbeschreibbar — Löschen oder ein Herstellungsversuch ist nicht möglich.",
        kind_erase: "wird gelöscht",
        kind_restore: "Herstellungsversuch läuft",
    },
    app_log: AppLogTexts {
        started: "Burnr gestartet — libburn wird vom System geladen…",
        settings_loaded: "Einstellungen aus der vorherigen Sitzung geladen",
        worker_stopped: "Worker-Thread wurde beendet",
        scan_requested: "Scan angefragt",
        inspect_requested: "Inspektionsanfrage für Laufwerk",
        reloading: "libburn (neu) laden…",
        rescan_after_reload: "Automatischer Scan folgt nach dem Neuladen.",
        auto_scan_started: "Automatischer Scan nach dem Neuladen gestartet",
        no_output_file: "Keine Ausgabedatei angegeben",
        read_requested: "Disc-Kopie angefragt für Laufwerk",
        cancel_read_requested: "Abbruch der Kopie angefragt",
        no_iso_file: "Keine ISO-Datei angegeben",
        iso_missing: "ISO-Datei existiert nicht",
        burn_requested: "Brennauftrag für Laufwerk",
        with_word: "mit",
        cancel_burn_requested: "Abbruch des Brennauftrags angefragt",
        cancel_maint_requested: "Abbruch des Wartungsauftrags angefragt",
        erase_requested: "Löschanfrage für Laufwerk",
        quick_word: "schnell",
        full_word: "vollständig",
        restore_requested: "Herstellungsversuch angefragt",
        no_burn_files: "Keine Dateien zum Brennen ausgewählt",
        burn_data_requested: "Data-Image-Brennauftrag für Laufwerk",
        files_cleared: "Dateiliste nach erfolgreichem Brennen geleert",
        reinspect_after_maint: "Automatische Neuinspektion nach Wartung…",
        media_cleared_after_eject: "Medieninformationen nach Auswurf gelöscht",
    },
};

// ─────────────────────────────────────────────────────────────────────────────
// Worker-logmeldingen (stap 10b-6).
//
// De worker-logregels staan als methodes op `Lang`: één match per melding
// houdt tekst en argumenten per taal bij elkaar — voor ~90 regels met
// uiteenlopende argumenten is dat compacter en foutbestendiger dan
// prefix-/suffix-velden. Pure-technische regels (libburn/libisofs-passthrough,
// SCSI-log, format-diagnostiek) staan bewust NIET hier: die zijn
// taalneutraal (Engelse technische terminologie).
impl Lang {
    // ── Levenscyclus / lading ──

    pub fn w_reload_busy(self) -> String {
        match self {
            Lang::EnUs => "Reload is not possible while jobs are active".to_string(),
            Lang::NlNl => "Herladen is niet mogelijk tijdens actieve jobs".to_string(),
            Lang::DeDe => "Neuladen ist nicht möglich, während Aufträge aktiv sind".to_string(),
        }
    }

    pub fn w_scan_busy(self) -> String {
        match self {
            Lang::EnUs => "Scanning is not possible while jobs are active".to_string(),
            Lang::NlNl => "Scannen is niet mogelijk tijdens actieve jobs".to_string(),
            Lang::DeDe => "Scan ist nicht möglich, während Aufträge aktiv sind".to_string(),
        }
    }

    pub fn w_read_busy(self) -> String {
        match self {
            Lang::EnUs => "Reading is not possible while jobs are active".to_string(),
            Lang::NlNl => "Lezen is niet mogelijk tijdens actieve jobs".to_string(),
            Lang::DeDe => "Lesen ist nicht möglich, während Aufträge aktiv sind".to_string(),
        }
    }

    pub fn w_inspect_busy_job(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Drive {index} is busy with a job — inspection skipped"),
            Lang::NlNl => format!("Station {index} is bezig met een job — inspectie overgeslagen"),
            Lang::DeDe => format!(
                "Laufwerk {index} ist mit einem Auftrag beschäftigt — Inspektion übersprungen"
            ),
        }
    }

    pub fn w_cancel_no_read(self) -> String {
        match self {
            Lang::EnUs => "Cancel requested, but no copy is running".to_string(),
            Lang::NlNl => "Annuleren gevraagd, maar er is geen kopie bezig".to_string(),
            Lang::DeDe => "Abbruch angefragt, aber es läuft keine Kopie".to_string(),
        }
    }

    pub fn w_cancel_requested(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Cancellation requested for drive {index}"),
            Lang::NlNl => format!("Annuleren gevraagd voor station {index}"),
            Lang::DeDe => format!("Abbruch für Laufwerk {index} angefragt"),
        }
    }

    pub fn w_cancel_no_job(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("No active job on drive {index}"),
            Lang::NlNl => format!("Geen actieve job op station {index}"),
            Lang::DeDe => format!("Kein aktiver Auftrag auf Laufwerk {index}"),
        }
    }

    pub fn w_device_open_mode(self, exclusive: bool) -> String {
        match self {
            Lang::EnUs => format!(
                "Device open mode: {}",
                if exclusive {
                    "exclusive (O_EXCL)"
                } else {
                    "non-exclusive (suitable for automount)"
                }
            ),
            Lang::NlNl => format!(
                "Apparaat-openingsmodus: {}",
                if exclusive {
                    "exclusief (O_EXCL)"
                } else {
                    "niet-exclusief (geschikt bij automount)"
                }
            ),
            Lang::DeDe => format!(
                "Geräteöffnungsmodus: {}",
                if exclusive {
                    "exklusiv (O_EXCL)"
                } else {
                    "nicht-exklusiv (geeignet bei Automount)"
                }
            ),
        }
    }

    pub fn w_burn_init_failed(self, cand: &str) -> String {
        match self {
            Lang::EnUs => format!("`{cand}`: burn_initialize() failed"),
            Lang::NlNl => format!("`{cand}`: burn_initialize() mislukte"),
            Lang::DeDe => format!("`{cand}`: burn_initialize() fehlgeschlagen"),
        }
    }

    pub fn w_isofs_init_failed(self, cand: &str) -> String {
        match self {
            Lang::EnUs => format!("`{cand}`: iso_init() failed"),
            Lang::NlNl => format!("`{cand}`: iso_init() mislukte"),
            Lang::DeDe => format!("`{cand}`: iso_init() fehlgeschlagen"),
        }
    }

    pub fn w_lib_loaded(self, version: &str, path: &str) -> String {
        match self {
            Lang::EnUs => format!("libburn {version} loaded via `{path}`"),
            Lang::NlNl => format!("libburn {version} geladen via `{path}`"),
            Lang::DeDe => format!("libburn {version} geladen über `{path}`"),
        }
    }

    pub fn w_lib_failed(self, last_err: &str) -> String {
        match self {
            Lang::EnUs => format!("libburn not loaded — {last_err}"),
            Lang::NlNl => format!("libburn niet geladen — {last_err}"),
            Lang::DeDe => format!("libburn nicht geladen — {last_err}"),
        }
    }

    pub fn w_isofs_loaded(self, version: &str, path: &str) -> String {
        match self {
            Lang::EnUs => format!("libisofs {version} loaded via `{path}`"),
            Lang::NlNl => format!("libisofs {version} geladen via `{path}`"),
            Lang::DeDe => format!("libisofs {version} geladen über `{path}`"),
        }
    }

    pub fn w_isofs_unavailable_warn(self, last_err: &str) -> String {
        match self {
            Lang::EnUs => {
                format!("libisofs not loaded — “compose files” is not available ({last_err})")
            }
            Lang::NlNl => format!(
                "libisofs niet geladen — “bestanden samenstellen” is niet beschikbaar ({last_err})"
            ),
            Lang::DeDe => format!(
                "libisofs nicht geladen — „Dateien zusammenstellen“ ist nicht verfügbar ({last_err})"
            ),
        }
    }

    pub fn w_lib_finished(self) -> String {
        match self {
            Lang::EnUs => "libburn shut down (burn_finish)".to_string(),
            Lang::NlNl => "libburn afgesloten (burn_finish)".to_string(),
            Lang::DeDe => "libburn beendet (burn_finish)".to_string(),
        }
    }

    pub fn w_isofs_finished(self) -> String {
        match self {
            Lang::EnUs => "libisofs shut down (iso_finish)".to_string(),
            Lang::NlNl => "libisofs afgesloten (iso_finish)".to_string(),
            Lang::DeDe => "libisofs beendet (iso_finish)".to_string(),
        }
    }

    // ── Scan ──

    pub fn w_scan_no_lib(self) -> String {
        match self {
            Lang::EnUs => "Scan requested, but libburn is not loaded".to_string(),
            Lang::NlNl => "Scan aangevraagd, maar libburn is niet geladen".to_string(),
            Lang::DeDe => "Scan angefragt, aber libburn ist nicht geladen".to_string(),
        }
    }

    pub fn w_scanning(self) -> String {
        match self {
            Lang::EnUs => "Scanning for disc drives…".to_string(),
            Lang::NlNl => "Scannen naar schijfstations…".to_string(),
            Lang::DeDe => "Suche nach Disc-Laufwerken…".to_string(),
        }
    }

    pub fn w_scan_failed(self, ret: i32) -> String {
        match self {
            Lang::EnUs => format!("Scan failed (libburn error code {ret})"),
            Lang::NlNl => format!("Scan mislukt (libburn-foutcode {ret})"),
            Lang::DeDe => format!("Scan fehlgeschlagen (libburn-Fehlercode {ret})"),
        }
    }

    pub fn w_scan_done(self, count: usize) -> String {
        match self {
            Lang::EnUs => format!("Scan complete: {count} drive(s) found"),
            Lang::NlNl => format!("Scan voltooid: {count} station(s) gevonden"),
            Lang::DeDe => format!("Scan abgeschlossen: {count} Laufwerk(e) gefunden"),
        }
    }

    // ── Inspectie ──

    pub fn w_inspect_no_lib(self) -> String {
        match self {
            Lang::EnUs => "Inspection requested, but libburn is not loaded".to_string(),
            Lang::NlNl => "Inspectie aangevraagd, maar libburn is niet geladen".to_string(),
            Lang::DeDe => "Inspektion angefragt, aber libburn ist nicht geladen".to_string(),
        }
    }

    pub fn w_inspect_unknown(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Inspection requested for unknown drive {index}"),
            Lang::NlNl => format!("Inspectie aangevraagd voor onbekend station {index}"),
            Lang::DeDe => format!("Inspektion für unbekanntes Laufwerk {index} angefragt"),
        }
    }

    pub fn w_inspect_grab(self, index: usize, name: &str) -> String {
        match self {
            Lang::EnUs => format!("Drive {index} ({name}): grabbing…"),
            Lang::NlNl => format!("Station {index} ({name}): grabben…"),
            Lang::DeDe => format!("Laufwerk {index} ({name}): grabben…"),
        }
    }

    /// Gedeelde grab-foutmelding (inspectie, branden, wissen, lezen).
    pub fn w_grab_failed(self, index: usize, adr: &str) -> String {
        let target = if adr.is_empty() {
            match self {
                Lang::EnUs => format!("drive {index}"),
                Lang::NlNl => format!("station {index}"),
                Lang::DeDe => format!("Laufwerk {index}"),
            }
        } else {
            adr.to_string()
        };
        match self {
            Lang::EnUs => format!(
                "Grab failed for {target} — drive may be busy (e.g. automount by the \
                 file manager). Turn off “Exclusive open” (Settings → Device) or \
                 unmount the disc."
            ),
            Lang::NlNl => format!(
                "Grab mislukt voor {target} — drive mogelijk bezet (bijv. automount \
                 door de bestandsbeheerder). Zet “Exclusief openen” uit \
                 (Instellingen → Apparaat) of unmount de schijf."
            ),
            Lang::DeDe => format!(
                "Grab fehlgeschlagen für {target} — Laufwerk möglicherweise belegt \
                 (z. B. Automount durch den Dateimanager). Schalte „Exklusiv öffnen“ \
                 aus (Einstellungen → Gerät) oder hänge die Disc aus."
            ),
        }
    }

    pub fn w_inspect_grabbed(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: grab succeeded; determining media status…"),
            Lang::NlNl => format!("Station {index}: grab gelukt; media-status bepalen…"),
            Lang::DeDe => {
                format!("Laufwerk {index}: Grab erfolgreich; Medienstatus wird ermittelt…")
            }
        }
    }

    pub fn w_inspect_media(self, index: usize, status: DiscStatus) -> String {
        let label = self.texts().media.disc_status_label(status);
        match self {
            Lang::EnUs => format!("Drive {index}: media = {label}"),
            Lang::NlNl => format!("Station {index}: media = {label}"),
            Lang::DeDe => format!("Laufwerk {index}: Medium = {label}"),
        }
    }

    pub fn w_inspect_profile(self, index: usize, name: &str, profile_no: i32) -> String {
        match self {
            Lang::EnUs => {
                format!("Drive {index}: media type = {name} (profile 0x{profile_no:02X})")
            }
            Lang::NlNl => {
                format!("Station {index}: mediatype = {name} (profiel 0x{profile_no:02X})")
            }
            Lang::DeDe => {
                format!("Laufwerk {index}: Medientyp = {name} (Profil 0x{profile_no:02X})")
            }
        }
    }

    pub fn w_inspect_capacity(self, index: usize, size: &str) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: readable capacity ≈ {size}"),
            Lang::NlNl => format!("Station {index}: leesbare capaciteit ≈ {size}"),
            Lang::DeDe => format!("Laufwerk {index}: lesbare Kapazität ≈ {size}"),
        }
    }

    pub fn w_inspect_media_code(self, index: usize, code: &str, manu: &str) -> String {
        let manu_part = if manu.is_empty() {
            String::new()
        } else {
            format!(" — {manu}")
        };
        match self {
            Lang::EnUs => format!("Drive {index}: media code = {code}{manu_part}"),
            Lang::NlNl => format!("Station {index}: mediacode = {code}{manu_part}"),
            Lang::DeDe => format!("Laufwerk {index}: Mediacode = {code}{manu_part}"),
        }
    }

    pub fn w_inspect_dm_active(self, index: usize, free: i32, alloc: i32) -> String {
        match self {
            Lang::EnUs => format!(
                "Drive {index}: defect management active — spare free {free} of {alloc} blocks"
            ),
            Lang::NlNl => format!(
                "Station {index}: defect management actief — spare vrij {free} van {alloc} blokken"
            ),
            Lang::DeDe => format!(
                "Laufwerk {index}: Defect Management aktiv — Spare frei {free} von {alloc} Blöcken"
            ),
        }
    }

    pub fn w_inspect_dm_off(self) -> String {
        match self {
            Lang::EnUs => "No BD spare info — defect management not active".to_string(),
            Lang::NlNl => "Geen BD spare-info — defect management niet actief".to_string(),
            Lang::DeDe => "Keine BD-Spare-Info — Defect Management nicht aktiv".to_string(),
        }
    }

    pub fn w_inspect_toc(self, index: usize, sessions: usize, tracks: usize) -> String {
        match self {
            Lang::EnUs => {
                format!("Drive {index}: TOC read — {sessions} session(s), {tracks} track(s)")
            }
            Lang::NlNl => {
                format!("Station {index}: TOC gelezen — {sessions} sessie(s), {tracks} track(s)")
            }
            Lang::DeDe => {
                format!("Laufwerk {index}: TOC gelesen — {sessions} Session(s), {tracks} Track(s)")
            }
        }
    }

    pub fn w_inspect_speeds(self, index: usize, n: usize) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: {n} speed(s) read"),
            Lang::NlNl => format!("Station {index}: {n} snelheid(s) gelezen"),
            Lang::DeDe => format!("Laufwerk {index}: {n} Geschwindigkeit(en) gelesen"),
        }
    }

    pub fn w_inspect_done(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: inspection done, drive released"),
            Lang::NlNl => format!("Station {index}: inspectie klaar, station vrijgegeven"),
            Lang::DeDe => format!("Laufwerk {index}: Inspektion fertig, Laufwerk freigegeben"),
        }
    }

    // ── Branden (ISO) ──

    pub fn w_burn_no_lib(self) -> String {
        match self {
            Lang::EnUs => "Burn requested, but libburn is not loaded".to_string(),
            Lang::NlNl => "Branden aangevraagd, maar libburn is niet geladen".to_string(),
            Lang::DeDe => "Brennen angefragt, aber libburn ist nicht geladen".to_string(),
        }
    }

    pub fn w_burn_unknown(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Burn requested for unknown drive {index}"),
            Lang::NlNl => format!("Branden aangevraagd voor onbekend station {index}"),
            Lang::DeDe => format!("Brennen für unbekanntes Laufwerk {index} angefragt"),
        }
    }

    pub fn w_burn_no_iso(self) -> String {
        match self {
            Lang::EnUs => "Burn requested without an ISO file".to_string(),
            Lang::NlNl => "Branden aangevraagd zonder ISO-bestand".to_string(),
            Lang::DeDe => "Brennen ohne ISO-Datei angefragt".to_string(),
        }
    }

    pub fn w_raw_iso_unsupported(self) -> String {
        match self {
            Lang::EnUs => "Write mode RAW is not supported for ISO files (requires 2352-byte source data)".to_string(),
            Lang::NlNl => "Schrijfmodus RAW wordt niet ondersteund voor ISO-bestanden (vereist 2352-byte brondata)".to_string(),
            Lang::DeDe => "Schreibmodus RAW wird für ISO-Dateien nicht unterstützt (erfordert 2352-Byte-Quelldaten)".to_string(),
        }
    }

    pub fn w_raw_data_unsupported(self) -> String {
        match self {
            Lang::EnUs => "Write mode RAW is not supported for data images".to_string(),
            Lang::NlNl => "Schrijfmodus RAW wordt niet ondersteund voor data-images".to_string(),
            Lang::DeDe => "Schreibmodus RAW wird für Data-Images nicht unterstützt".to_string(),
        }
    }

    pub fn w_burn_open_failed(self, path: &str, err: &str) -> String {
        match self {
            Lang::EnUs => format!("Cannot open ISO file `{path}`: {err}"),
            Lang::NlNl => format!("Kan ISO-bestand `{path}` niet openen: {err}"),
            Lang::DeDe => format!("ISO-Datei `{path}` konnte nicht geöffnet werden: {err}"),
        }
    }

    pub fn w_empty_iso(self) -> String {
        match self {
            Lang::EnUs => "ISO file is empty".to_string(),
            Lang::NlNl => "ISO-bestand is leeg".to_string(),
            Lang::DeDe => "ISO-Datei ist leer".to_string(),
        }
    }

    pub fn w_padding_note(self) -> String {
        match self {
            Lang::EnUs => "File size is not a multiple of 2048 bytes; the last sector will be padded with zeros".to_string(),
            Lang::NlNl => "Bestandsgrootte is geen veelvoud van 2048 bytes; laatste sector wordt aangevuld met nullen".to_string(),
            Lang::DeDe => "Die Dateigröße ist kein Vielfaches von 2048 Bytes; der letzte Sektor wird mit Nullen aufgefüllt".to_string(),
        }
    }

    pub fn w_burn_starting(self, index: usize, path: &str, sectors: i32) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: starting burn job — `{path}` ({sectors} blocks)"),
            Lang::NlNl => {
                format!("Station {index}: brandjob starten — `{path}` ({sectors} blokken)")
            }
            Lang::DeDe => {
                format!("Laufwerk {index}: Brennauftrag starten — `{path}` ({sectors} Blöcke)")
            }
        }
    }

    /// Instellingenregel vóór een brandjob; `timestamps` alleen bij de
    /// data-image-variant.
    pub fn w_settings_line(
        self,
        simulate: bool,
        multi: MultiSession,
        padding: i32,
        overburn: bool,
        timestamps: Option<bool>,
    ) -> String {
        let on_off = |v: bool| match self {
            Lang::EnUs => {
                if v {
                    "on"
                } else {
                    "off"
                }
            }
            Lang::NlNl => {
                if v {
                    "aan"
                } else {
                    "uit"
                }
            }
            Lang::DeDe => {
                if v {
                    "ein"
                } else {
                    "aus"
                }
            }
        };
        let multi_word = match multi {
            MultiSession::Yes => match self {
                Lang::EnUs => "yes",
                Lang::NlNl => "ja",
                Lang::DeDe => "ja",
            },
            MultiSession::No => match self {
                Lang::EnUs => "no",
                Lang::NlNl => "nee",
                Lang::DeDe => "nein",
            },
        };
        let ts = match (self, timestamps) {
            (_, None) => String::new(),
            (Lang::EnUs, Some(true)) => ", timestamps=preserved".to_string(),
            (Lang::EnUs, Some(false)) => ", timestamps=recording time".to_string(),
            (Lang::NlNl, Some(true)) => ", tijdstempels=behouden".to_string(),
            (Lang::NlNl, Some(false)) => ", tijdstempels=opnametijd".to_string(),
            (Lang::DeDe, Some(true)) => ", Zeitstempel=behalten".to_string(),
            (Lang::DeDe, Some(false)) => ", Zeitstempel=Aufnahmezeit".to_string(),
        };
        match self {
            Lang::EnUs => format!(
                "Settings: simulation={}, multi-session={}, padding={padding} KiB, overburn={}{ts}",
                on_off(simulate),
                multi_word,
                on_off(overburn)
            ),
            Lang::NlNl => format!(
                "Instellingen: simulatie={}, multi-session={}, padding={padding} KiB, overburn={}{ts}",
                on_off(simulate),
                multi_word,
                on_off(overburn)
            ),
            Lang::DeDe => format!(
                "Einstellungen: Simulation={}, Multi-Session={}, Padding={padding} KiB, Overburn={}{ts}",
                on_off(simulate),
                multi_word,
                on_off(overburn)
            ),
        }
    }

    pub fn w_iso_size_warning(self, size: &str, avail: &str) -> String {
        match self {
            Lang::EnUs => {
                format!("ISO file (≈ {size}) seems larger than the available space (≈ {avail})")
            }
            Lang::NlNl => {
                format!("ISO-bestand (≈ {size}) lijkt groter dan de beschikbare ruimte (≈ {avail})")
            }
            Lang::DeDe => {
                format!("ISO-Datei (≈ {size}) scheint größer als der verfügbare Platz (≈ {avail})")
            }
        }
    }

    pub fn w_image_size_warning(self, size: &str, avail: &str) -> String {
        match self {
            Lang::EnUs => {
                format!("Image (≈ {size}) seems larger than the available space (≈ {avail})")
            }
            Lang::NlNl => {
                format!("Image (≈ {size}) lijkt groter dan de beschikbare ruimte (≈ {avail})")
            }
            Lang::DeDe => {
                format!("Image (≈ {size}) scheint größer als der verfügbare Platz (≈ {avail})")
            }
        }
    }

    pub fn w_speed(self, speed_max: bool, kbps: i32) -> String {
        let detail = if speed_max {
            match self {
                Lang::EnUs => "maximum".to_string(),
                Lang::NlNl => "maximaal".to_string(),
                Lang::DeDe => "maximal".to_string(),
            }
        } else {
            match self {
                Lang::EnUs => format!("maximum {kbps} kB/s"),
                Lang::NlNl => format!("maximaal {kbps} kB/s"),
                Lang::DeDe => format!("maximal {kbps} kB/s"),
            }
        };
        match self {
            Lang::EnUs => format!("Speed: {detail}"),
            Lang::NlNl => format!("Snelheid: {detail}"),
            Lang::DeDe => format!("Geschwindigkeit: {detail}"),
        }
    }

    pub fn w_bad_path_nul(self) -> String {
        match self {
            Lang::EnUs => "Invalid path (contains NUL byte)".to_string(),
            Lang::NlNl => "Ongeldig pad (bevat NUL-byte)".to_string(),
            Lang::DeDe => "Ungültiger Pfad (enthält NUL-Byte)".to_string(),
        }
    }

    pub fn w_burn_started(self, index: usize, simulate: bool) -> String {
        let sim = if simulate {
            match self {
                Lang::EnUs => " (SIMULATION — laser off)",
                Lang::NlNl => " (SIMULATIE — laser uit)",
                Lang::DeDe => " (SIMULATION — Laser aus)",
            }
        } else {
            ""
        };
        match self {
            Lang::EnUs => format!("Drive {index}: burning started{sim}…"),
            Lang::NlNl => format!("Station {index}: branden gestart{sim}…"),
            Lang::DeDe => format!("Laufwerk {index}: Brennen gestartet{sim}…"),
        }
    }

    // ── Data-image (libisofs) ──

    pub fn w_isofs_missing(self) -> String {
        match self {
            Lang::EnUs => "libisofs is not loaded — composing files is not available".to_string(),
            Lang::NlNl => {
                "libisofs is niet geladen — bestanden samenstellen is niet beschikbaar".to_string()
            }
            Lang::DeDe => {
                "libisofs ist nicht geladen — Dateien zusammenstellen ist nicht verfügbar"
                    .to_string()
            }
        }
    }

    pub fn w_burn_no_files(self) -> String {
        match self {
            Lang::EnUs => "No files selected to burn".to_string(),
            Lang::NlNl => "Geen bestanden gekozen om te branden".to_string(),
            Lang::DeDe => "Keine Dateien zum Brennen ausgewählt".to_string(),
        }
    }

    pub fn w_burn_file_missing(self, p: &str) -> String {
        match self {
            Lang::EnUs => format!("File/folder does not exist: `{p}`"),
            Lang::NlNl => format!("Bestand/map bestaat niet: `{p}`"),
            Lang::DeDe => format!("Datei/Ordner existiert nicht: `{p}`"),
        }
    }

    pub fn w_data_image_starting(self, index: usize, n: usize) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: composing data image from {n} item(s)…"),
            Lang::NlNl => format!("Station {index}: data-image samenstellen uit {n} item(s)…"),
            Lang::DeDe => {
                format!("Laufwerk {index}: Data-Image aus {n} Element(en) zusammenstellen…")
            }
        }
    }

    pub fn w_bad_volume_nul(self) -> String {
        match self {
            Lang::EnUs => "Invalid volume name (contains NUL byte)".to_string(),
            Lang::NlNl => "Ongeldige volumenaam (bevat NUL-byte)".to_string(),
            Lang::DeDe => "Ungültiger Volumename (enthält NUL-Byte)".to_string(),
        }
    }

    pub fn w_image_create_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the ISO image".to_string(),
            Lang::NlNl => "Kan de ISO-image niet aanmaken".to_string(),
            Lang::DeDe => "ISO-Image konnte nicht erstellt werden".to_string(),
        }
    }

    pub fn w_import_start(self, start_block: i32) -> String {
        match self {
            Lang::EnUs => format!(
                "Existing session (start LBA {start_block}) will be imported — new files are added to the existing content…"
            ),
            Lang::NlNl => format!(
                "Bestaande sessie (begin LBA {start_block}) wordt geïmporteerd — nieuwe bestanden komen bij de bestaande inhoud…"
            ),
            Lang::DeDe => format!(
                "Vorhandene Session (Start-LBA {start_block}) wird importiert — neue Dateien kommen zum bestehenden Inhalt hinzu…"
            ),
        }
    }

    pub fn w_bad_dev_nul(self) -> String {
        match self {
            Lang::EnUs => "Invalid device path (contains NUL byte)".to_string(),
            Lang::NlNl => "Ongeldig apparaatpad (bevat NUL-byte)".to_string(),
            Lang::DeDe => "Ungültiger Gerätepfad (enthält NUL-Byte)".to_string(),
        }
    }

    pub fn w_import_open_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot open the disc as a read source".to_string(),
            Lang::NlNl => "Kan de schijf niet als leesbron openen".to_string(),
            Lang::DeDe => "Die Disc konnte nicht als Lesequelle geöffnet werden".to_string(),
        }
    }

    pub fn w_read_opts_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the read options".to_string(),
            Lang::NlNl => "Kan de lees-opties niet aanmaken".to_string(),
            Lang::DeDe => "Die Leseoptionen konnten nicht erstellt werden".to_string(),
        }
    }

    pub fn w_import_done(self, blocks: i32, size: &str) -> String {
        match self {
            Lang::EnUs => format!("Existing session imported: {blocks} blocks (≈ {size})"),
            Lang::NlNl => format!("Bestaande sessie geïmporteerd: {blocks} blokken (≈ {size})"),
            Lang::DeDe => format!("Vorhandene Session importiert: {blocks} Blöcke (≈ {size})"),
        }
    }

    pub fn w_import_failed(self) -> String {
        match self {
            Lang::EnUs => {
                "Importing the existing session failed — see the libisofs messages".to_string()
            }
            Lang::NlNl => {
                "Importeren van de bestaande sessie mislukt — zie de libisofs-meldingen".to_string()
            }
            Lang::DeDe => {
                "Importieren der vorhandenen Session fehlgeschlagen — siehe die libisofs-Meldungen"
                    .to_string()
            }
        }
    }

    pub fn w_added_folder(self, p: &str) -> String {
        match self {
            Lang::EnUs => format!("Added: {p} (folder, recursive)"),
            Lang::NlNl => format!("Toegevoegd: {p} (map, recursief)"),
            Lang::DeDe => format!("Hinzugefügt: {p} (Ordner, rekursiv)"),
        }
    }

    pub fn w_added_file(self, p: &str) -> String {
        match self {
            Lang::EnUs => format!("Added: {p} (file)"),
            Lang::NlNl => format!("Toegevoegd: {p} (bestand)"),
            Lang::DeDe => format!("Hinzugefügt: {p} (Datei)"),
        }
    }

    pub fn w_item_skipped(self, e: &str) -> String {
        match self {
            Lang::EnUs => format!("Item skipped: {e}"),
            Lang::NlNl => format!("Item overgeslagen: {e}"),
            Lang::DeDe => format!("Element übersprungen: {e}"),
        }
    }

    pub fn w_nothing_added(self) -> String {
        match self {
            Lang::EnUs => "No item could be added to the image".to_string(),
            Lang::NlNl => "Geen enkel item kon aan de image worden toegevoegd".to_string(),
            Lang::DeDe => "Kein Element konnte zum Image hinzugefügt werden".to_string(),
        }
    }

    pub fn w_nwa_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot determine the next writable address (NWA) — continuing multi-session failed".to_string(),
            Lang::NlNl => "Kan de volgende schrijfadres (NWA) niet bepalen — multi-session voortzetten mislukt".to_string(),
            Lang::DeDe => "Die nächste Schreibadresse (NWA) konnte nicht ermittelt werden — Fortsetzen der Multi-Session fehlgeschlagen".to_string(),
        }
    }

    pub fn w_nwa(self, nwa: i32) -> String {
        match self {
            Lang::EnUs => format!("New session starts at LBA {nwa}"),
            Lang::NlNl => format!("Nieuwe sessie begint op LBA {nwa}"),
            Lang::DeDe => format!("Neue Session beginnt bei LBA {nwa}"),
        }
    }

    pub fn w_isofs_opts_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the libisofs write options".to_string(),
            Lang::NlNl => "Kan de libisofs write-opts niet aanmaken".to_string(),
            Lang::DeDe => "Die libisofs-Write-Optionen konnten nicht erstellt werden".to_string(),
        }
    }

    pub fn w_layout(self) -> String {
        match self {
            Lang::EnUs => "Calculating image layout (this may take a while)…".to_string(),
            Lang::NlNl => "Image-layout berekenen (kan even duren)…".to_string(),
            Lang::DeDe => "Image-Layout wird berechnet (kann eine Weile dauern)…".to_string(),
        }
    }

    pub fn w_compose_failed(self) -> String {
        match self {
            Lang::EnUs => "Composing the image failed — see the libisofs messages".to_string(),
            Lang::NlNl => {
                "Samenstellen van de image mislukt — zie de libisofs-meldingen".to_string()
            }
            Lang::DeDe => {
                "Zusammenstellen des Images fehlgeschlagen — siehe die libisofs-Meldungen"
                    .to_string()
            }
        }
    }

    pub fn w_fifo_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the FIFO around the image source".to_string(),
            Lang::NlNl => "Kan de FIFO om de image-bron niet aanmaken".to_string(),
            Lang::DeDe => "Die FIFO um die Image-Quelle konnte nicht erstellt werden".to_string(),
        }
    }

    pub fn w_size_unknown(self) -> String {
        match self {
            Lang::EnUs => "Image size unknown — composition aborted".to_string(),
            Lang::NlNl => "Imagegrootte onbekend — samenstellen afgebroken".to_string(),
            Lang::DeDe => "Imagegröße unbekannt — Zusammenstellen abgebrochen".to_string(),
        }
    }

    pub fn w_image_ready(self, sectors: i32, size: &str, n: usize) -> String {
        match self {
            Lang::EnUs => format!("Image ready: {sectors} blocks (≈ {size}) from {n} item(s)"),
            Lang::NlNl => format!("Image klaar: {sectors} blokken (≈ {size}) uit {n} item(s)"),
            Lang::DeDe => format!("Image fertig: {sectors} Blöcke (≈ {size}) aus {n} Element(en)"),
        }
    }

    pub fn w_model_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot build the libburn model".to_string(),
            Lang::NlNl => "Kan het libburn-model niet opbouwen".to_string(),
            Lang::DeDe => "Das libburn-Modell konnte nicht aufgebaut werden".to_string(),
        }
    }

    pub fn w_attach_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot attach the libisofs source to the track".to_string(),
            Lang::NlNl => "Kan de libisofs-bron niet aan de track koppelen".to_string(),
            Lang::DeDe => {
                "Die libisofs-Quelle konnte nicht an den Track gekoppelt werden".to_string()
            }
        }
    }

    // ── Grab / media-check / schrijfmodus ──

    pub fn w_media_not_writable(self, status: DiscStatus) -> String {
        let label = self.texts().media.disc_status_label(status);
        match self {
            Lang::EnUs => format!(
                "Media is not writable (status: {label}) — blank or appendable media required"
            ),
            Lang::NlNl => format!(
                "Media is niet beschrijfbaar (status: {label}) — lege of onvolledige media nodig"
            ),
            Lang::DeDe => format!(
                "Medium ist nicht beschreibbar (Status: {label}) — leere oder appendable Medien erforderlich"
            ),
        }
    }

    pub fn w_media_appendable(self) -> String {
        match self {
            Lang::EnUs => {
                "Media is appendable — the new session will be appended to it".to_string()
            }
            Lang::NlNl => {
                "Media is onvolledig (appendable) — de nieuwe sessie wordt eraan toegevoegd"
                    .to_string()
            }
            Lang::DeDe => {
                "Medium ist appendable — die neue Session wird daran angehängt".to_string()
            }
        }
    }

    pub fn w_make_opts_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the write options".to_string(),
            Lang::NlNl => "Kan de write-opts niet aanmaken".to_string(),
            Lang::DeDe => "Die Write-Optionen konnten nicht erstellt werden".to_string(),
        }
    }

    pub fn w_overwritable_note(self, profile_no: i32) -> String {
        match self {
            Lang::EnUs => format!(
                "Media (profile 0x{profile_no:02X}) is directly overwritable — multi-session does not apply; the media always stays writable"
            ),
            Lang::NlNl => format!(
                "Media (profiel 0x{profile_no:02X}) is direct overschrijfbaar — multi-session is niet van toepassing; de media blijft altijd beschrijfbaar"
            ),
            Lang::DeDe => format!(
                "Medium (Profil 0x{profile_no:02X}) ist direkt wiederbeschreibbar — Multi-Session trifft nicht zu; das Medium bleibt immer beschreibbar"
            ),
        }
    }

    pub fn w_overburn_note(self) -> String {
        match self {
            Lang::EnUs => "Overburn/force on: some conformity checks are ignored".to_string(),
            Lang::NlNl => {
                "Overburn/force aan: enkele conformiteitschecks worden genegeerd".to_string()
            }
            Lang::DeDe => {
                "Overburn/Force ein: einige Konformitätsprüfungen werden ignoriert".to_string()
            }
        }
    }

    pub fn w_write_mode_failed(self, reasons: &str) -> String {
        let r = if reasons.is_empty() {
            match self {
                Lang::EnUs => "unknown reason",
                Lang::NlNl => "onbekende reden",
                Lang::DeDe => "unbekannter Grund",
            }
        } else {
            reasons
        };
        match self {
            Lang::EnUs => format!("No suitable write mode found: {r}"),
            Lang::NlNl => format!("Geen geschikte schrijfmodus gevonden: {r}"),
            Lang::DeDe => format!("Kein geeigneter Schreibmodus gefunden: {r}"),
        }
    }

    pub fn w_write_mode_auto(self, name: &str) -> String {
        match self {
            Lang::EnUs => format!("Write mode (auto): {name}"),
            Lang::NlNl => format!("Schrijfmodus (auto): {name}"),
            Lang::DeDe => format!("Schreibmodus (auto): {name}"),
        }
    }

    pub fn w_tao_unsupported(self) -> String {
        match self {
            Lang::EnUs => "TAO + MODE1 is not supported by this drive/media".to_string(),
            Lang::NlNl => "TAO + MODE1 wordt niet door deze drive/media ondersteund".to_string(),
            Lang::DeDe => {
                "TAO + MODE1 wird von diesem Laufwerk/Medium nicht unterstützt".to_string()
            }
        }
    }

    pub fn w_write_mode_tao(self) -> String {
        match self {
            Lang::EnUs => "Write mode: TAO (track-at-once)".to_string(),
            Lang::NlNl => "Schrijfmodus: TAO (track-at-once)".to_string(),
            Lang::DeDe => "Schreibmodus: TAO (track-at-once)".to_string(),
        }
    }

    pub fn w_sao_unsupported(self) -> String {
        match self {
            Lang::EnUs => "SAO + SAO block type is not supported by this drive/media".to_string(),
            Lang::NlNl => {
                "SAO + SAO-bloktype wordt niet door deze drive/media ondersteund".to_string()
            }
            Lang::DeDe => {
                "SAO + SAO-Blocktyp wird von diesem Laufwerk/Medium nicht unterstützt".to_string()
            }
        }
    }

    pub fn w_write_mode_sao(self) -> String {
        match self {
            Lang::EnUs => "Write mode: SAO (session-at-once)".to_string(),
            Lang::NlNl => "Schrijfmodus: SAO (session-at-once)".to_string(),
            Lang::DeDe => "Schreibmodus: SAO (session-at-once)".to_string(),
        }
    }

    pub fn w_precheck_failed(self, reasons: &str) -> String {
        let r = if reasons.is_empty() {
            match self {
                Lang::EnUs => "unknown reason",
                Lang::NlNl => "onbekende reden",
                Lang::DeDe => "unbekannter Grund",
            }
        } else {
            reasons
        };
        match self {
            Lang::EnUs => format!("This write mode is rejected for this drive/media: {r}"),
            Lang::NlNl => format!("Deze schrijfmodus wordt afgewezen voor deze drive/media: {r}"),
            Lang::DeDe => {
                format!("Dieser Schreibmodus wird für dieses Laufwerk/Medium abgelehnt: {r}")
            }
        }
    }

    pub fn w_simulation_hint(self) -> String {
        match self {
            Lang::EnUs => "Tip: “Simulation (laser off)” is not supported by this drive/media. Simulation practically only works with CD media; all BD media, DVD-R DL and overwritable media (DVD+RW, DVD-RAM, DVD-RW RO) can never simulate. Turn “Simulation” off in Settings → Burning — note: without simulation everything is written for real.".to_string(),
            Lang::NlNl => "Tip: “Simulatie (laser uit)” wordt door deze drive/media niet ondersteund. Simulatie werkt praktisch alleen bij CD-media; alle BD-media, DVD-R DL en overbeschrijfbare media (DVD+RW, DVD-RAM, DVD-RW RO) kunnen nooit simuleren. Zet “Simulatie” uit in Instellingen → Branden — let op: zonder simulatie wordt er écht geschreven.".to_string(),
            Lang::DeDe => "Tipp: „Simulation (Laser aus)“ wird von diesem Laufwerk/Medium nicht unterstützt. Simulation funktioniert praktisch nur bei CD-Medien; alle BD-Medien, DVD-R DL und wiederbeschreibbare Medien (DVD+RW, DVD-RAM, DVD-RW RO) können nie simulieren. Schalte „Simulation“ in Einstellungen → Brennen aus — Achtung: ohne Simulation wird wirklich geschrieben.".to_string(),
        }
    }
}

impl Lang {
    // ── Job-polling / eject / afronding ──

    pub fn w_maint_progress(self, kind: MaintKind, pct: i32) -> String {
        match (self, kind) {
            (Lang::EnUs, MaintKind::Erase) => format!("Erasing… {pct}%"),
            (Lang::NlNl, MaintKind::Erase) => format!("Wissen… {pct}%"),
            (Lang::DeDe, MaintKind::Erase) => format!("Löschen… {pct}%"),
            (Lang::EnUs, MaintKind::Format) => format!("Restore attempt… {pct}%"),
            (Lang::NlNl, MaintKind::Format) => format!("Herstelpoging… {pct}%"),
            (Lang::DeDe, MaintKind::Format) => format!("Herstellungsversuch… {pct}%"),
        }
    }

    pub fn w_eject_mounted(self, adr: &str) -> String {
        match self {
            Lang::EnUs => format!("Disc `{adr}` is mounted — unmounting first for the eject…"),
            Lang::NlNl => {
                format!("Schijf `{adr}` is aangekoppeld — eerst unmounten voor de eject…")
            }
            Lang::DeDe => {
                format!("Disc `{adr}` ist eingehängt — wird zuerst für den Auswurf ausgehängt…")
            }
        }
    }

    pub fn w_regrab_failed(self, attempt: i32) -> String {
        match self {
            Lang::EnUs => format!("Re-grab attempt {attempt} failed — retrying in a second…"),
            Lang::NlNl => format!("Re-grab poging {attempt} mislukt — nogmaals over een seconde…"),
            Lang::DeDe => {
                format!("Re-Grab-Versuch {attempt} fehlgeschlagen — Wiederholung in einer Sekunde…")
            }
        }
    }

    pub fn w_eject_sent(self) -> String {
        match self {
            Lang::EnUs => "Eject request sent".to_string(),
            Lang::NlNl => "Eject-verzoek verzonden".to_string(),
            Lang::DeDe => "Auswurfanfrage gesendet".to_string(),
        }
    }

    pub fn w_eject_manual(self) -> String {
        match self {
            Lang::EnUs => "Could not re-grab the drive for the eject — manual eject needed".to_string(),
            Lang::NlNl => "Kon het station niet opnieuw grabben voor de eject — eject handmatig nodig".to_string(),
            Lang::DeDe => "Das Laufwerk konnte für den Auswurf nicht erneut gegrabbed werden — manuelles Auswerfen nötig".to_string(),
        }
    }

    pub fn w_drive_released(self) -> String {
        match self {
            Lang::EnUs => "Drive released".to_string(),
            Lang::NlNl => "Drive vrijgegeven".to_string(),
            Lang::DeDe => "Laufwerk freigegeben".to_string(),
        }
    }

    pub fn w_burn_cancelled(self) -> String {
        match self {
            Lang::EnUs => "Burn job cancelled".to_string(),
            Lang::NlNl => "Brandjob geannuleerd".to_string(),
            Lang::DeDe => "Brennauftrag abgebrochen".to_string(),
        }
    }

    pub fn w_burn_done(self, index: usize, simulate: bool, keep_open: bool) -> String {
        let sim = if simulate {
            match self {
                Lang::EnUs => " (simulation)",
                Lang::NlNl => " (simulatie)",
                Lang::DeDe => " (Simulation)",
            }
        } else {
            ""
        };
        let media = match (self, keep_open) {
            (Lang::EnUs, true) => "stays appendable",
            (Lang::EnUs, false) => "is closed",
            (Lang::NlNl, true) => "blijft appendable",
            (Lang::NlNl, false) => "is afgesloten",
            (Lang::DeDe, true) => "bleibt appendable",
            (Lang::DeDe, false) => "ist abgeschlossen",
        };
        match self {
            Lang::EnUs => format!("Drive {index}: burn job done{sim} — media {media}"),
            Lang::NlNl => format!("Station {index}: brandjob klaar{sim} — media {media}"),
            Lang::DeDe => format!("Laufwerk {index}: Brennauftrag fertig{sim} — Medium {media}"),
        }
    }

    pub fn w_burn_summary(self, index: usize, size: &str, dur: &str, kbps: f64) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: {size} in {dur} — average {kbps:.0} kB/s"),
            Lang::NlNl => format!("Station {index}: {size} in {dur} — gemiddeld {kbps:.0} kB/s"),
            Lang::DeDe => {
                format!("Laufwerk {index}: {size} in {dur} — durchschnittlich {kbps:.0} kB/s")
            }
        }
    }

    pub fn w_burn_failed(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!(
                "Burn job on drive {index} failed — the drive reports the write did not \
                 complete properly. Check the media (quality/warping) and try a lower \
                 speed or another disc"
            ),
            Lang::NlNl => format!(
                "Brandjob op station {index} mislukt — de drive meldt dat de \
                 schrijfactie niet goed is verlopen. Controleer de media \
                 (kwaliteit/kromming) en probeer eventueel een lagere \
                 snelheid of andere schijf"
            ),
            Lang::DeDe => format!(
                "Brennauftrag auf Laufwerk {index} fehlgeschlagen — das Laufwerk meldet, \
                 dass der Schreibvorgang nicht korrekt abgeschlossen wurde. Prüfe das \
                 Medium (Qualität/Verwölbung) und versuche gegebenenfalls eine niedrigere \
                 Geschwindigkeit oder eine andere Disc"
            ),
        }
    }

    pub fn w_erase_cancelled(self) -> String {
        match self {
            Lang::EnUs => "Erase cancelled".to_string(),
            Lang::NlNl => "Wissen geannuleerd".to_string(),
            Lang::DeDe => "Löschen abgebrochen".to_string(),
        }
    }

    pub fn w_erased(self) -> String {
        match self {
            Lang::EnUs => "Media erased".to_string(),
            Lang::NlNl => "Media gewist".to_string(),
            Lang::DeDe => "Medium gelöscht".to_string(),
        }
    }

    pub fn w_erase_failed(self) -> String {
        match self {
            Lang::EnUs => "Erase failed — see the libburn messages above".to_string(),
            Lang::NlNl => "Wissen mislukt — zie de libburn-meldingen hierboven".to_string(),
            Lang::DeDe => "Löschen fehlgeschlagen — siehe die libburn-Meldungen oben".to_string(),
        }
    }

    pub fn w_restore_cancelled(self) -> String {
        match self {
            Lang::EnUs => "Restore attempt cancelled".to_string(),
            Lang::NlNl => "Herstelpoging geannuleerd".to_string(),
            Lang::DeDe => "Herstellungsversuch abgebrochen".to_string(),
        }
    }

    pub fn w_restore_done(self) -> String {
        match self {
            Lang::EnUs => "Restore attempt succeeded — media re-formatted".to_string(),
            Lang::NlNl => "Herstelpoging geslaagd — media opnieuw geformatteerd".to_string(),
            Lang::DeDe => "Herstellungsversuch erfolgreich — Medium neu formatiert".to_string(),
        }
    }

    pub fn w_restore_failed(self) -> String {
        match self {
            Lang::EnUs => "Restore attempt failed — see the libburn messages above. Tips: try toggling ‘Enable certification’ and ‘Enable defect management’ — some drives refuse BD-RE without spare areas or with full certification".to_string(),
            Lang::NlNl => "Herstelpoging mislukt — zie de libburn-meldingen hierboven. Tips: wissel ‘Certificering activeren’ en ‘Defect management activeren’ eens uit — sommige drives weigeren BD-RE zonder spare-gebieden of met volledige certificatie".to_string(),
            Lang::DeDe => "Herstellungsversuch fehlgeschlagen — siehe die libburn-Meldungen oben. Tipps: wechsle zwischen „Zertifizierung aktivieren“ und „Defect Management aktivieren“ — manche Laufwerke verweigern BD-RE ohne Spare-Bereiche oder mit voller Zertifizierung".to_string(),
        }
    }

    // ── Brandmodel (libburn disc/session/track) ──

    pub fn w_model_create_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the disc/session/track model".to_string(),
            Lang::NlNl => "Kan disc/session/track-model niet aanmaken".to_string(),
            Lang::DeDe => "Disc/Session/Track-Modell konnte nicht erstellt werden".to_string(),
        }
    }

    pub fn w_model_add_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot add session/track to the model".to_string(),
            Lang::NlNl => "Kan session/track niet aan het model toevoegen".to_string(),
            Lang::DeDe => "Session/Track konnte nicht zum Modell hinzugefügt werden".to_string(),
        }
    }

    pub fn w_file_source_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the file source (file unreadable?)".to_string(),
            Lang::NlNl => "Kan de file-bron niet aanmaken (bestand onleesbaar?)".to_string(),
            Lang::DeDe => {
                "Die Dateiquelle konnte nicht erstellt werden (Datei unlesbar?)".to_string()
            }
        }
    }

    pub fn w_fifo_source_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot create the FIFO source".to_string(),
            Lang::NlNl => "Kan de FIFO-bron niet aanmaken".to_string(),
            Lang::DeDe => "Die FIFO-Quelle konnte nicht erstellt werden".to_string(),
        }
    }

    pub fn w_source_attach_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot attach the source to the track".to_string(),
            Lang::NlNl => "Kan de bron niet aan de track koppelen".to_string(),
            Lang::DeDe => "Die Quelle konnte nicht an den Track gekoppelt werden".to_string(),
        }
    }

    pub fn w_track_size_failed(self) -> String {
        match self {
            Lang::EnUs => "Cannot set the track size".to_string(),
            Lang::NlNl => "Kan de trackgrootte niet instellen".to_string(),
            Lang::DeDe => "Die Trackgröße konnte nicht festgelegt werden".to_string(),
        }
    }

    // ── Unmount (eject-diagnose) ──

    pub fn w_unmounted(self, dev: &str) -> String {
        match self {
            Lang::EnUs => format!("Mount of `{dev}` released"),
            Lang::NlNl => format!("Aankoppeling van `{dev}` is opgeheven"),
            Lang::DeDe => format!("Einhängung von `{dev}` wurde gelöst"),
        }
    }

    pub fn w_unmount_failed(self, dev: &str, err: &str) -> String {
        match self {
            Lang::EnUs => format!("Unmount of `{dev}` failed: {err}"),
            Lang::NlNl => format!("Unmount van `{dev}` mislukt: {err}"),
            Lang::DeDe => format!("Aushängen von `{dev}` fehlgeschlagen: {err}"),
        }
    }

    pub fn w_unmount_unavailable(self, e: &str) -> String {
        match self {
            Lang::EnUs => format!("`udisksctl` not available for unmount: {e}"),
            Lang::NlNl => format!("`udisksctl` niet beschikbaar voor unmount: {e}"),
            Lang::DeDe => format!("`udisksctl` nicht verfügbar zum Aushängen: {e}"),
        }
    }

    // ── Wissen ──

    pub fn w_erase_no_lib(self) -> String {
        match self {
            Lang::EnUs => "Erase requested, but libburn is not loaded".to_string(),
            Lang::NlNl => "Wissen aangevraagd, maar libburn is niet geladen".to_string(),
            Lang::DeDe => "Löschen angefragt, aber libburn ist nicht geladen".to_string(),
        }
    }

    pub fn w_erase_unknown(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Erase requested for unknown drive {index}"),
            Lang::NlNl => format!("Wissen aangevraagd voor onbekend station {index}"),
            Lang::DeDe => format!("Löschen für unbekanntes Laufwerk {index} angefragt"),
        }
    }

    pub fn w_erase_starting(self, index: usize, fast: bool) -> String {
        let mode = match (self, fast) {
            (Lang::EnUs, true) => "quick",
            (Lang::EnUs, false) => "full",
            (Lang::NlNl, true) => "snel",
            (Lang::NlNl, false) => "volledig",
            (Lang::DeDe, true) => "schnell",
            (Lang::DeDe, false) => "vollständig",
        };
        match self {
            Lang::EnUs => format!("Drive {index}: erasing media ({mode})…"),
            Lang::NlNl => format!("Station {index}: media wissen ({mode})…"),
            Lang::DeDe => format!("Laufwerk {index}: Medium wird gelöscht ({mode})…"),
        }
    }

    pub fn w_erase_overwritable(self, profile_no: i32) -> String {
        match self {
            Lang::EnUs => format!(
                "Media (profile 0x{profile_no:02X}) is directly overwritable — erasing is not \
                 needed; use the “Restore attempt” to restore the disc"
            ),
            Lang::NlNl => format!(
                "Media (profiel 0x{profile_no:02X}) is direct overschrijfbaar — wissen is niet \
                 nodig; gebruik de “Herstelpoging” om de schijf te herstellen"
            ),
            Lang::DeDe => format!(
                "Medium (Profil 0x{profile_no:02X}) ist direkt wiederbeschreibbar — Löschen ist \
                 nicht nötig; verwende den „Herstellungsversuch“, um die Disc wiederherzustellen"
            ),
        }
    }

    pub fn w_erase_not_rewritable(self) -> String {
        match self {
            Lang::EnUs => "This media is not rewritable — erasing is not possible".to_string(),
            Lang::NlNl => {
                "Deze media is niet herbeschrijfbaar — wissen is niet mogelijk".to_string()
            }
            Lang::DeDe => {
                "Dieses Medium ist nicht wiederbeschreibbar — Löschen ist nicht möglich".to_string()
            }
        }
    }

    // ── Herstelpoging (format) ──

    pub fn w_restore_no_lib(self) -> String {
        match self {
            Lang::EnUs => "Restore attempt requested, but libburn is not loaded".to_string(),
            Lang::NlNl => "Herstelpoging aangevraagd, maar libburn is niet geladen".to_string(),
            Lang::DeDe => {
                "Herstellungsversuch angefragt, aber libburn ist nicht geladen".to_string()
            }
        }
    }

    pub fn w_restore_unknown(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Restore attempt requested for unknown drive {index}"),
            Lang::NlNl => format!("Herstelpoging aangevraagd voor onbekend station {index}"),
            Lang::DeDe => format!("Herstellungsversuch für unbekanntes Laufwerk {index} angefragt"),
        }
    }

    pub fn w_restore_default_size(self) -> String {
        match self {
            Lang::EnUs => "Restore attempt: default size…".to_string(),
            Lang::NlNl => "Herstelpoging: standaardgrootte…".to_string(),
            Lang::DeDe => "Herstellungsversuch: Standardgröße…".to_string(),
        }
    }

    pub fn w_restore_not_applicable(self, profile_no: i32) -> String {
        match self {
            Lang::EnUs => format!(
                "Restore attempt does not apply to this media (profile 0x{profile_no:02X}) — \
                 for CD-RW/DVD-RW sequential, “Erase” is the right action"
            ),
            Lang::NlNl => format!(
                "Herstelpoging is niet van toepassing op deze media (profiel 0x{profile_no:02X}) \
                 — voor CD-RW/DVD-RW sequentieel is “Wissen” de juiste actie"
            ),
            Lang::DeDe => format!(
                "Herstellungsversuch trifft auf dieses Medium nicht zu (Profil 0x{profile_no:02X}) \
                 — bei CD-RW/DVD-RW sequential ist „Löschen“ die richtige Aktion"
            ),
        }
    }

    pub fn w_restore_no_dm_type(self) -> String {
        match self {
            Lang::EnUs => "The drive does not offer format type 0x31 (BD-RE without defect \
                 management) — libburn will refuse that format; turn off ‘Disable defect \
                 management’"
                .to_string(),
            Lang::NlNl => "De drive biedt geen format-type 0x31 (BD-RE zonder defect \
                 management) aan — libburn weigert dit format dan; zet \
                 ‘Defect management uitschakelen’ uit"
                .to_string(),
            Lang::DeDe => "Das Laufwerk bietet keinen Formattyp 0x31 (BD-RE ohne Defect \
                 Management) an — libburn wird dieses Format ablehnen; schalte \
                 „Defect Management deaktivieren“ aus"
                .to_string(),
        }
    }

    pub fn w_restore_dm_off(self) -> String {
        match self {
            Lang::EnUs => "Defect management will be disabled for this restore attempt — faster burning, but bad blocks are no longer remapped".to_string(),
            Lang::NlNl => "Defect management wordt bij deze herstelpoging uitgeschakeld — sneller branden, maar slechte blokken worden niet meer hermapd".to_string(),
            Lang::DeDe => "Defect Management wird bei diesem Herstellungsversuch deaktiviert — schnelleres Brennen, aber schlechte Blöcke werden nicht mehr remappt".to_string(),
        }
    }

    pub fn w_restore_cert_skip(self) -> String {
        match self {
            Lang::EnUs => "Certification will be skipped — libburn chooses format type 0x00 without certification (quick format)".to_string(),
            Lang::NlNl => "Certificatie wordt overgeslagen — libburn kiest format-type 0x00 zonder certificatie (snelformat)".to_string(),
            Lang::DeDe => "Zertifizierung wird übersprungen — libburn wählt Formattyp 0x00 ohne Zertifizierung (Schnellformat)".to_string(),
        }
    }

    pub fn w_restore_running(self) -> String {
        match self {
            Lang::EnUs => "Restore attempt: the media is being re-formatted; this can take several minutes…".to_string(),
            Lang::NlNl => "Herstelpoging: de media wordt opnieuw geformatteerd; dit kan enkele minuten duren…".to_string(),
            Lang::DeDe => "Herstellungsversuch: das Medium wird neu formatiert; das kann einige Minuten dauern…".to_string(),
        }
    }

    // ── Schijfkopie lezen ──

    pub fn w_read_no_lib(self) -> String {
        match self {
            Lang::EnUs => "Copy requested, but libburn is not loaded".to_string(),
            Lang::NlNl => "Kopie aangevraagd, maar libburn is niet geladen".to_string(),
            Lang::DeDe => "Kopie angefragt, aber libburn ist nicht geladen".to_string(),
        }
    }

    pub fn w_read_unknown(self, index: usize) -> String {
        match self {
            Lang::EnUs => format!("Copy requested for unknown drive {index}"),
            Lang::NlNl => format!("Kopie aangevraagd voor onbekend station {index}"),
            Lang::DeDe => format!("Kopie für unbekanntes Laufwerk {index} angefragt"),
        }
    }

    pub fn w_read_no_path(self) -> String {
        match self {
            Lang::EnUs => "Copy requested without an output file".to_string(),
            Lang::NlNl => "Kopie aangevraagd zonder uitvoerbestand".to_string(),
            Lang::DeDe => "Kopie ohne Ausgabedatei angefragt".to_string(),
        }
    }

    /// Prefix "Station 0" / "Drive 0" / "Laufwerk 0" voor logregels die een
    /// extern opgebouwde melding voorgaan.
    pub fn w_station(self, index: usize) -> String {
        format!("{} {index}", self.texts().drives.unnamed_prefix)
    }

    pub fn w_read_start(self, index: usize, path: &str) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: starting disc copy to `{path}`…"),
            Lang::NlNl => format!("Station {index}: schijfkopie starten naar `{path}`…"),
            Lang::DeDe => format!("Laufwerk {index}: Disc-Kopie starten nach `{path}`…"),
        }
    }

    pub fn w_read_exists(self, path: &str) -> String {
        match self {
            Lang::EnUs => format!("Output file already exists: `{path}` — choose a different name"),
            Lang::NlNl => format!("Uitvoerbestand bestaat al: `{path}` — kies een andere naam"),
            Lang::DeDe => {
                format!("Ausgabedatei existiert bereits: `{path}` — wähle einen anderen Namen")
            }
        }
    }

    pub fn w_read_no_capacity(self) -> String {
        match self {
            Lang::EnUs => "No readable capacity — media blank or not data media (CD audio is not yet supported)".to_string(),
            Lang::NlNl => "Geen leesbare capaciteit — media leeg of geen datamedia (CD-audio wordt nog niet ondersteund)".to_string(),
            Lang::DeDe => "Keine lesbare Kapazität — Medium leer oder kein Datenmedium (CD-Audio wird noch nicht unterstützt)".to_string(),
        }
    }

    pub fn w_read_blocks(self, index: usize, blocks: i64, size: &str) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: reading {blocks} blocks (≈ {size})…"),
            Lang::NlNl => format!("Station {index}: {blocks} blokken (≈ {size}) lezen…"),
            Lang::DeDe => format!("Laufwerk {index}: {blocks} Blöcke (≈ {size}) lesen…"),
        }
    }

    pub fn w_read_create_failed(self, path: &str, err: &str) -> String {
        match self {
            Lang::EnUs => format!("Cannot create `{path}`: {err}"),
            Lang::NlNl => format!("Kan `{path}` niet aanmaken: {err}"),
            Lang::DeDe => format!("`{path}` konnte nicht erstellt werden: {err}"),
        }
    }

    pub fn w_read_cancelled(self, index: usize, blocks: i64, path: &str) -> String {
        match self {
            Lang::EnUs => format!(
                "Drive {index}: copy cancelled after {blocks} blocks; `{path}` is left (incomplete)"
            ),
            Lang::NlNl => format!(
                "Station {index}: kopie geannuleerd na {blocks} blokken; `{path}` blijft (onvolledig) staan"
            ),
            Lang::DeDe => format!(
                "Laufwerk {index}: Kopie nach {blocks} Blöcken abgebrochen; `{path}` bleibt (unvollständig) stehen"
            ),
        }
    }

    pub fn w_read_error(self, index: usize, err: &str) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: {err}"),
            Lang::NlNl => format!("Station {index}: {err}"),
            Lang::DeDe => format!("Laufwerk {index}: {err}"),
        }
    }

    pub fn w_read_done(self, index: usize, blocks: i64, size: &str, path: &str) -> String {
        match self {
            Lang::EnUs => format!("Drive {index}: disc copy done — {blocks} ({size}) → `{path}`"),
            Lang::NlNl => {
                format!("Station {index}: schijfkopie klaar — {blocks} ({size}) → `{path}`")
            }
            Lang::DeDe => {
                format!("Laufwerk {index}: Disc-Kopie fertig — {blocks} ({size}) → `{path}`")
            }
        }
    }
}
