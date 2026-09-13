//! Instellingen die de gebruiker via de GUI kan aanpassen.
//!
//! De waarden worden bij het branden/kopiëren doorgegeven aan libburn en
//! persistent opgeslagen (stap 8, eframe-persistence).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WriteMode {
    Auto,
    Tao,
    Sao,
    Raw,
}

impl WriteMode {
    pub fn label(self) -> &'static str {
        match self {
            WriteMode::Auto => "Auto",
            WriteMode::Tao => "TAO (track-at-once)",
            WriteMode::Sao => "SAO (session-at-once)",
            WriteMode::Raw => "RAW",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MultiSession {
    Auto,
    KeepOpen,
    Close,
}

impl MultiSession {
    pub fn label(self) -> &'static str {
        match self {
            MultiSession::Auto => "Auto (media wordt afgesloten)",
            MultiSession::KeepOpen => "Open laten (multi-session)",
            MultiSession::Close => "Afsluiten (finalize)",
        }
    }
}

/// Brand-instellingen. `#[serde(default)]` zorgt dat opslagbestanden uit
/// oudere versies (met inmiddels verwijderde velden) zonder fouten laden.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BurnSettings {
    /// Schrijfmodus: Auto (libburn kiest), TAO of SAO.
    pub write_mode: WriteMode,
    /// Simulatie-brand (laser uit) — alleen op schrijf-eenmalige media.
    pub simulate: bool,
    pub multi_session: MultiSession,
    /// Padding in KiB vóór de payload.
    pub padding_kib: i32,
    pub overburn: bool,
    pub underrun_proof: bool,
    /// Schijf uitwerpen na afloop.
    pub eject_after: bool,
    /// Bij bestands-branden: originele bestandsdatums behouden i.p.v. de
    /// opnametijd (Rock Ridge + directoryrecords).
    pub keep_timestamps: bool,
    /// Bij formatteren van BD/DVD-RAM: defect management proberen uit te
    /// schakelen (sneller branden, geen hermapping van slechte blokken).
    pub disable_dm_on_format: bool,
}

impl Default for BurnSettings {
    fn default() -> Self {
        Self {
            write_mode: WriteMode::Auto,
            simulate: false,
            multi_session: MultiSession::Auto,
            padding_kib: 0,
            overburn: false,
            underrun_proof: true,
            eject_after: true,
            keep_timestamps: true,
            disable_dm_on_format: false,
        }
    }
}
