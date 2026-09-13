//! Linkerpaneel: lijst met gevonden schijfstations.

use crate::app::{App, LibState, ScanState};
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    egui::Panel::left("drives_panel")
        .default_size(280.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Stations");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🔄").clicked() {
                        app.request_scan();
                    }
                });
            });
            ui.separator();

            match &app.scan_state {
                ScanState::Scanning => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Bezig met scannen…");
                    });
                }
                ScanState::Failed { error } => {
                    ui.colored_label(colors::BAD, format!("Scan mislukt: {error}"));
                }
                _ => {}
            }

            if app.drives.is_empty() && app.scan_state != ScanState::Scanning {
                match &app.lib_state {
                    LibState::Loaded { .. } => {
                        if app.exclusive_open {
                            ui.colored_label(
                                colors::WARN,
                                "Geen stations gevonden terwijl “Exclusief openen” \
                                 aan staat. Een aangekoppelde schijf (automount) kan \
                                 de drive blokkeren — zet de modus uit \
                                 (Instellingen → Apparaat) en scan opnieuw.",
                            );
                        } else {
                            ui.weak(
                                "Geen stations gevonden. Sluit een brander aan en scan opnieuw.",
                            );
                        }
                    }
                    _ => {
                        ui.weak("Stations verschijnen hier zodra libburn geladen is.");
                    }
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for d in &app.drives {
                    let selected = app.selected == Some(d.index);
                    let inspecting = app.busy_drive == Some(d.index);
                    let inspected = d.media.is_some();

                    let dot = if inspecting {
                        colors::WARN
                    } else if inspected {
                        colors::OK
                    } else {
                        colors::DIM
                    };

                    let title = d.display_name();
                    let sub = if d.adr.is_empty() {
                        "(adres onbekend)".to_string()
                    } else {
                        d.adr.clone()
                    };
                    let label = egui::RichText::new(format!("{title}\n{sub}"));

                    // Knop krijgt de breedte minus de LED-kolom, zodat het
                    // statusstip nooit over de tekst heen valt.
                    ui.horizontal(|ui| {
                        let btn_w = (ui.available_width() - 16.0).max(80.0);
                        let resp = ui.add_sized(
                            egui::vec2(btn_w, 38.0),
                            egui::Button::selectable(selected, label),
                        );
                        if resp.clicked() {
                            app.selected = Some(d.index);
                        }
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 3.5, dot);
                    });
                }
            });
        });
}
