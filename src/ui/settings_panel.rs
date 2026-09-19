//! Rechterpaneel: alle instellingen die bij het branden aan libburn worden
//! doorgegeven. Teksten via de i18n-catalogus (stap 10b-1).

use crate::app::App;
use crate::settings::{MultiSession, WriteMode};
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let t = app.lang.texts();
    let c = colors::palette(ui.ctx());
    egui::Panel::right("settings_panel")
        .default_size(310.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.heading(t.settings.title);
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                burn_section(ui, app);
                ui.add_space(4.0);
                device_section(ui, app);

                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(t.settings.apply_hint)
                        .small()
                        .color(c.dim),
                );
            });
        });
}

fn burn_section(ui: &mut egui::Ui, app: &mut App) {
    // Multi-session is alleen zinvol op schrijf-eenmalige media; op
    // overschrijfbare media negeert de worker de vlag toch al. Zonder
    // inspectie (geen media bekend) laten we de keuze gewoon actief.
    let multi_applicable = app
        .selected
        .and_then(|i| app.drives.get(i))
        .and_then(|d| d.media.as_ref())
        .is_none_or(|m| !crate::worker::profile_is_overwritable(m.profile_no));

    let t = app.lang.texts();
    let c = colors::palette(ui.ctx());
    egui::CollapsingHeader::new(egui::RichText::new(t.settings.burn_header).strong())
        .default_open(true)
        .show(ui, |ui| {
            let s = &mut app.settings;

            ui.label(egui::RichText::new(t.settings.speed).strong());
            ui.weak(t.settings.speed_desc);

            ui.add_space(4.0);
            ui.label(egui::RichText::new(t.settings.write_mode).strong());
            for mode in [
                WriteMode::Auto,
                WriteMode::Tao,
                WriteMode::Sao,
                WriteMode::Raw,
            ] {
                ui.radio_value(&mut s.write_mode, mode, mode.label());
            }

            ui.add_space(4.0);
            ui.checkbox(&mut s.simulate, t.settings.simulate);
            ui.checkbox(&mut s.overburn, t.settings.overburn);
            ui.checkbox(&mut s.underrun_proof, t.settings.underrun);

            ui.add_space(4.0);
            ui.add_enabled_ui(multi_applicable, |ui| {
                ui.label(egui::RichText::new(t.settings.multi_session).strong())
                    .on_hover_text(t.settings.multi_session_hover);
                for m in [MultiSession::No, MultiSession::Yes] {
                    ui.radio_value(&mut s.multi_session, m, t.settings.ms_label(m));
                }
                if !multi_applicable {
                    ui.label(
                        egui::RichText::new(t.settings.ms_not_applicable)
                            .small()
                            .color(c.dim),
                    );
                }
            });

            ui.add_space(4.0);
            ui.label(egui::RichText::new(t.settings.general).strong());
            ui.horizontal(|ui| {
                ui.label(t.settings.padding);
                ui.add(egui::DragValue::new(&mut s.padding_kib).range(0..=100_000))
                    .on_hover_text(t.settings.padding_hover);
            });

            ui.add_space(4.0);
            ui.checkbox(&mut s.keep_timestamps, t.settings.keep_timestamps)
                .on_hover_text(t.settings.keep_timestamps_hover);

            ui.add_space(4.0);
            ui.checkbox(&mut s.eject_after, t.settings.eject_after)
                .on_hover_text(t.settings.eject_after_hover);
        });
}

fn device_section(ui: &mut egui::Ui, app: &mut App) {
    let t = app.lang.texts();
    egui::CollapsingHeader::new(egui::RichText::new(t.settings.device_header).strong())
        .default_open(false)
        .show(ui, |ui| {
            let mut want = app.exclusive_open;
            if ui.checkbox(&mut want, t.settings.exclusive_open).changed() {
                app.set_exclusive_open(want);
            }
            ui.weak(format!(
                "{}: {}",
                t.settings.current_mode,
                t.settings.mode_label(app.exclusive_open)
            ));
            ui.weak(t.settings.exclusive_hint);
            ui.weak(t.settings.exclusive_reload);
        });
}
