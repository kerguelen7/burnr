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
    /// Fel oranje voor nadruk (bijv. de brandsnelheid tijdens het branden).
    pub const ORANGE: Color32 = Color32::from_rgb(255, 145, 30);
}

/// Kleine schuif-schakelaar (aan/uit) in de paneelstijl — een fraaier
/// alternatief voor een checkbox bij positief geformuleerde opties.
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired = egui::vec2(28.0, 15.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    // Vloeiende overgang van de knop tussen uit- en aan-stand.
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.12);
    let track = if *on { colors::OK } else { colors::DIM };
    let track = track.gamma_multiply(0.35 + 0.65 * t);
    let knob_x = egui::lerp(rect.left() + 7.5..=rect.right() - 7.5, t);
    let center = egui::pos2(knob_x, rect.center().y);

    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(7), track);
    ui.painter()
        .circle_filled(center, 5.0, egui::Color32::WHITE);

    // Builder-stijl: on_hover_cursor neemt self — dus als laatste stap.
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
