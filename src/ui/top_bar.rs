//! Bovenbalk: titel, libburn-status, taalkeuze en de scan-knop.

use crate::app::{App, LibState, ScanState};
use crate::i18n::Lang;
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let t = app.lang.texts();
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
                    ui.weak(t.top_bar.loading);
                }
                LibState::Loaded { version, path } => {
                    ui.colored_label(colors::OK, format!("● libburn {version}"));
                    ui.weak(format!("({path})"));
                }
                LibState::Failed { .. } => {
                    ui.colored_label(colors::BAD, format!("● {}", t.top_bar.not_loaded));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let scanning = app.scan_state == ScanState::Scanning;
                let label = if scanning {
                    t.top_bar.scanning
                } else {
                    t.top_bar.scan
                };
                let loading = matches!(app.lib_state, LibState::Loading);
                if ui
                    .add_enabled(!scanning && !loading, egui::Button::new(label))
                    .clicked()
                {
                    app.request_scan();
                }

                // Taalkeuze: endoniemen (English/Nederlands/Deutsch) zijn
                // taalneutraal en hoeven dus niet vertaald te worden. De
                // worker krijgt de wissel mee zodat zijn logregels volgen.
                let mut want = app.lang;
                egui::ComboBox::from_id_salt("lang_choice")
                    .selected_text(app.lang.endonym())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for lang in Lang::ALL {
                            ui.selectable_value(&mut want, lang, lang.endonym());
                        }
                    });
                if want != app.lang {
                    app.set_language(want);
                }
            });
        });
        ui.add_space(4.0);
    });
}
