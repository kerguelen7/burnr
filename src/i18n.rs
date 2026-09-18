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
    pub top_bar: TopBarTexts,
    pub log: LogTexts,
    pub settings: SettingsTexts,
    pub drives: DrivesTexts,
    // Groeit mee met de migratie: details, logmeldingen (app/worker), help.
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
};
