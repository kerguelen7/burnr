//! Bovenbalk: titel, libburn-status en de scan-knop.

use crate::app::{App, LibState, ScanState};
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    egui::Panel::top("top_bar").show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if let Some(icon) = &app.icon {
                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(20.0, 20.0)));
            }
            ui.heading("Burnr");
            ui.small(egui::RichText::new("A lightweight libburn GUI for optical discs").weak());
            ui.separator();

            match &app.lib_state {
                LibState::Loading => {
                    ui.spinner();
                    ui.weak("libburn laden…");
                }
                LibState::Loaded { version, path } => {
                    ui.colored_label(colors::OK, format!("● libburn {version}"));
                    ui.weak(format!("({path})"));
                }
                LibState::Failed { .. } => {
                    ui.colored_label(colors::BAD, "● libburn niet geladen");
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let scanning = app.scan_state == ScanState::Scanning;
                let label = if scanning {
                    "⏳ Scannen…"
                } else {
                    "🔄 Scannen"
                };
                let loading = matches!(app.lib_state, LibState::Loading);
                if ui
                    .add_enabled(!scanning && !loading, egui::Button::new(label))
                    .clicked()
                {
                    app.request_scan();
                }
            });
        });
        ui.add_space(4.0);
    });
}
