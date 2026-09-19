//! About-box: icoon, naam + versie, tagline en de runtime-geladen
//! library-versies (ℹ-knopje in de top bar).

use crate::app::{App, LibState};
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if !app.show_about {
        return;
    }
    let t = app.lang.texts();
    let modal = egui::Modal::new(egui::Id::new("about_modal")).show(ui.ctx(), |ui| {
        ui.set_min_width(360.0);
        ui.horizontal(|ui| {
            if let Some(icon) = &app.icon {
                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(48.0, 48.0)));
            }
            ui.vertical(|ui| {
                ui.strong(t.about.title);
                ui.weak(format!(
                    "Burnr {} — A lightweight libburn GUI for optical discs",
                    env!("CARGO_PKG_VERSION")
                ));
            });
        });
        ui.add_space(4.0);
        ui.separator();

        match &app.lib_state {
            LibState::Loaded { version, path } => {
                ui.label(format!("libburn {version} · {path}"));
            }
            LibState::Loading => {
                ui.weak(t.details.loading);
            }
            LibState::Failed { .. } => {
                ui.colored_label(colors::BAD, t.about.lib_failed);
            }
        }
        match &app.isofs_version {
            Some(v) => {
                ui.label(format!("libisofs {v}"));
            }
            None => {
                ui.weak(t.about.isofs_missing);
            }
        }
        ui.weak(t.about.runtime_note);
        // Licentie: naam + standard-notice zijn conventioneel taalneutraal.
        ui.weak(format!(
            "{}: GNU General Public License version 3 or later",
            t.about.license_label
        ));
        ui.weak(
            "This program comes with ABSOLUTELY NO WARRANTY; for details, \
             see the LICENSE file.",
        );

        ui.add_space(4.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(t.about.close).clicked() {
                app.show_about = false;
            }
        });
    });
    if modal.should_close() {
        app.show_about = false;
    }
}
