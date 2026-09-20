//! Bovenbalk: titel, libburn-status, taalkeuze, zoom, thema en de scan-knop.

use crate::app::{App, LibState, MAX_ZOOM, MIN_ZOOM, ScanState, zoom_in, zoom_out};
use crate::i18n::Lang;
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let t = app.lang.texts();
    let c = colors::palette(ui.ctx());
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
                    ui.colored_label(c.ok, format!("● libburn {version}"));
                    ui.weak(format!("({path})"));
                }
                LibState::Failed { .. } => {
                    ui.colored_label(c.bad, format!("● {}", t.top_bar.not_loaded));
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

                // Kleurthema (stap 10b-8); namen zijn taalneutraal.
                let mut want_theme = app.theme;
                let theme_resp = egui::ComboBox::from_id_salt("theme_choice")
                    .selected_text(app.theme.label())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for th in crate::app::Theme::ALL {
                            ui.selectable_value(&mut want_theme, th, th.label());
                        }
                    });
                theme_resp.response.on_hover_text(t.top_bar.theme_hover);
                if want_theme != app.theme {
                    app.set_theme(ui.ctx(), want_theme);
                }

                // Zoomregeling (stap 10c): doet exact hetzelfde als de
                // sneltoetsen Ctrl+ / Ctrl− / Ctrl+0 — beide lopen via
                // app::zoom_step met de eigen grenzen (80–160%). De labels
                // zijn symbolen + percentage, dus taalneutraal; alleen de
                // hovers staan in de tekstcatalogus. De layout is
                // right-to-left, dus visueel verschijnt het als − % +.
                let zoom = ui.ctx().zoom_factor();
                if ui
                    .add_enabled(zoom < MAX_ZOOM, egui::Button::new("+"))
                    .on_hover_text(format!(
                        "{} ({})",
                        t.top_bar.zoom_in_hover,
                        ui.ctx()
                            .format_shortcut(&egui::gui_zoom::kb_shortcuts::ZOOM_IN)
                    ))
                    .clicked()
                {
                    zoom_in(ui.ctx());
                }
                if ui
                    .add_enabled(
                        zoom != 1.0,
                        // Normale tekstgrootte, zodat het percentage gelijk
                        // loopt met de rest van de top bar; vaste breedte
                        // tegen het verspringen van 100% → 110%.
                        egui::Button::new(format!("{:.0}%", zoom * 100.0))
                            .min_size(egui::vec2(52.0, 20.0)),
                    )
                    .on_hover_text(format!(
                        "{} ({})",
                        t.top_bar.zoom_reset_hover,
                        ui.ctx()
                            .format_shortcut(&egui::gui_zoom::kb_shortcuts::ZOOM_RESET)
                    ))
                    .clicked()
                {
                    ui.ctx().set_zoom_factor(1.0);
                }
                if ui
                    .add_enabled(zoom > MIN_ZOOM, egui::Button::new("−"))
                    .on_hover_text(format!(
                        "{} ({})",
                        t.top_bar.zoom_out_hover,
                        ui.ctx()
                            .format_shortcut(&egui::gui_zoom::kb_shortcuts::ZOOM_OUT)
                    ))
                    .clicked()
                {
                    zoom_out(ui.ctx());
                }

                // About-box.
                if ui
                    .small_button("ℹ")
                    .on_hover_text(t.top_bar.about_hover)
                    .clicked()
                {
                    app.show_about = true;
                }
            });
        });
        ui.add_space(4.0);
    });
}
