//! Onderpaneel: feedback-log met niveaufilter en auto-scroll.

use crate::app::App;
use crate::i18n::LogTexts;
use crate::logger::Level;
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let t = app.lang.texts();
    egui::Panel::bottom("log_panel")
        .default_size(190.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t.log.title).strong());
                ui.separator();
                ui.label(egui::RichText::new(t.log.show).small());
                for level in Level::FILTER_ORDER {
                    filter_toggle(ui, app, level, &t.log);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(t.log.clear).clicked() {
                        app.log.clear();
                    }
                    if ui
                        .small_button(t.log.save)
                        .on_hover_text(t.log.save_hover)
                        .clicked()
                    {
                        if let Some(p) = rfd::FileDialog::new()
                            .set_file_name("burnr-log.txt")
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
                                        format!("{}: {}", t.log.saved_prefix, p.display()),
                                    );
                                }
                                Err(e) => {
                                    app.log.push(
                                        Level::Error,
                                        format!("{}: {e}", t.log.save_failed_prefix),
                                    );
                                }
                            }
                        }
                    }
                    ui.checkbox(&mut app.auto_scroll, t.log.auto_scroll);
                });
            });
            ui.separator();
            if let Some(p) = &app.log_file {
                ui.label(
                    egui::RichText::new(format!("{}: {}", t.log.session_log, p.display()))
                        .small()
                        .color(colors::DIM),
                );
            }

            egui::ScrollArea::vertical()
                .stick_to_bottom(app.auto_scroll)
                .show(ui, |ui| {
                    for e in app.log.entries() {
                        if !app.log_filter.allows(e.level) {
                            continue;
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(egui::RichText::new(&e.time).small().color(colors::DIM));
                            ui.monospace(
                                egui::RichText::new(t.log.level_tag(e.level))
                                    .small()
                                    .color(e.level.color()),
                            );
                            ui.label(egui::RichText::new(&e.msg).small());
                        });
                    }
                });
        });
}

fn filter_toggle(ui: &mut egui::Ui, app: &mut App, level: Level, t: &LogTexts) {
    let mut on = app.log_filter.allows(level);
    if ui.toggle_value(&mut on, t.filter_label(level)).changed() {
        app.log_filter.set(level, on);
    }
}
