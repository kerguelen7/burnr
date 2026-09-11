//! UI-panelen van de applicatie.

pub mod details_panel;
pub mod drives_panel;
pub mod log_panel;
pub mod settings_panel;
pub mod top_bar;

/// Vaste accentkleuren die door de panelen worden gedeeld.
pub mod colors {
    use egui::Color32;

    pub const OK: Color32 = Color32::from_rgb(108, 200, 128);
    pub const WARN: Color32 = Color32::from_rgb(232, 182, 92);
    pub const BAD: Color32 = Color32::from_rgb(238, 112, 112);
    pub const DIM: Color32 = Color32::from_rgb(120, 128, 140);
    pub const ACCENT: Color32 = Color32::from_rgb(120, 170, 250);
}
