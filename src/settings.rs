//! Instellingen die de gebruiker via de GUI kan aanpassen.
//!
//! Stap 1: de waarden worden beheerd en getoond; de koppeling naar
//! `burn_write_opts` / `burn_drive_set_speed` volgt in de brand-stap.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlankMode {
    Fast,
    Full,
}

impl BlankMode {
    pub fn label(self) -> &'static str {
        match self {
            BlankMode::Fast => "Snel (blank fast)",
            BlankMode::Full => "Volledig (blank full)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Clone, Debug)]
pub struct BurnSettings {
    /// `true` = maximale snelheid, anders `speed_kbps` gebruiken.
    pub speed_max: bool,
    /// Eigen snelheid in kB/s (libburn-eenheid: 1000 bytes/s).
    pub speed_kbps: i32,
    pub write_mode: WriteMode,
    /// Simulatie-brand (laser uit).
    pub simulate: bool,
    pub multi_session: MultiSession,
    /// Padding in KiB vóór de payload.
    pub padding_kib: i32,
    pub overburn: bool,
    pub underrun_proof: bool,
    /// Media eerst wissen (herbeschrijfbare media).
    pub blank_first: bool,
    pub blank_mode: BlankMode,
    /// Schijf uitwerpen na afloop.
    pub eject_after: bool,
    /// Bij bestands-branden: originele bestandsdatums behouden i.p.v. de
    /// opnametijd (Rock Ridge + directoryrecords).
    pub keep_timestamps: bool,
}

impl Default for BurnSettings {
    fn default() -> Self {
        Self {
            speed_max: true,
            speed_kbps: 4234, // ≈ 24× CD
            write_mode: WriteMode::Auto,
            simulate: false,
            multi_session: MultiSession::Auto,
            padding_kib: 0,
            overburn: false,
            underrun_proof: true,
            blank_first: false,
            blank_mode: BlankMode::Fast,
            eject_after: true,
            keep_timestamps: true,
        }
    }
}
