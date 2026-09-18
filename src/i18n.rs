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
    // Groeit mee met de migratie: branden/schijfkopie/onderhoud,
    // logmeldingen (app/worker), help.
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
};
