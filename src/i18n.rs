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
    // Groeit mee met de migratie: settings, details, drives, media,
    // logmeldingen (app/worker), help.
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
};
