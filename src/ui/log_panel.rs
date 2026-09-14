//! Onderpaneel: feedback-log met niveaufilter en auto-scroll.

use crate::app::App;
use crate::logger::Level;
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    egui::Panel::bottom("log_panel")
        .default_size(190.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Log").strong());
                ui.separator();
                ui.label(egui::RichText::new("toon:").small());
                filter_toggle(ui, app, 0, "info");
                filter_toggle(ui, app, 1, "ok");
                filter_toggle(ui, app, 2, "waarschuwing");
                filter_toggle(ui, app, 3, "fout");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Wissen").clicked() {
                        app.log.clear();
                    }
                    if ui
                        .small_button("💾 Opslaan…")
                        .on_hover_text("Sla de zichtbare logregels op in een bestand")
                        .clicked()
                    {
                        if let Some(p) = rfd::FileDialog::new()
                            .set_file_name("libburn_gui-log.txt")
                            .save_file()
                        {
                            match std::fs::File::create(&p) {
                                Ok(mut f) => {
                                    use std::io::Write;
                                    for e in app.log.entries() {
                                        let _ = writeln!(
                                            f,
                                            "{} [{:>4}] {}",
                                            e.time,
                                            e.level.tag().trim(),
                                            e.msg
                                        );
                                    }
                                    let _ = f.flush();
                                    app.log.push(
                                        Level::Success,
                                        format!("Log opgeslagen: {}", p.display()),
                                    );
                                }
                                Err(e) => {
                                    app.log
                                        .push(Level::Error, format!("Log opslaan mislukt: {e}"));
                                }
                            }
                        }
                    }
                    ui.checkbox(&mut app.auto_scroll, "auto-scroll");
                });
            });
            ui.separator();
            if let Some(p) = &app.log_file {
                ui.label(
                    egui::RichText::new(format!("Sessielog: {}", p.display()))
                        .small()
                        .color(colors::DIM),
                );
            }

            egui::ScrollArea::vertical()
                .stick_to_bottom(app.auto_scroll)
                .show(ui, |ui| {
                    for e in app.log.entries() {
                        if !app.log_filter[level_index(e.level)] {
                            continue;
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(egui::RichText::new(&e.time).small().color(colors::DIM));
                            ui.monospace(
                                egui::RichText::new(e.level.tag())
                                    .small()
                                    .color(e.level.color()),
                            );
                            ui.label(egui::RichText::new(&e.msg).small());
                        });
                    }
                });
        });
}

fn filter_toggle(ui: &mut egui::Ui, app: &mut App, idx: usize, label: &str) {
    let mut on = app.log_filter[idx];
    if ui.toggle_value(&mut on, label).changed() {
        app.log_filter[idx] = on;
    }
}

fn level_index(level: Level) -> usize {
    match level {
        Level::Info => 0,
        Level::Success => 1,
        Level::Warning => 2,
        Level::Error => 3,
    }
}
