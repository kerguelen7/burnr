//! Rechterpaneel: alle instellingen die later aan libburn worden gekoppeld.

use crate::app::App;
use crate::settings::{MultiSession, WriteMode};
use crate::ui::colors;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    egui::Panel::right("settings_panel")
        .default_size(310.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.heading("Instellingen");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                burn_section(ui, app);
                ui.add_space(4.0);
                device_section(ui, app);

                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "ℹ Deze waarden worden toegepast bij het branden (🔥 Branden \
                         in het mediakaartje).",
                    )
                    .small()
                    .color(colors::DIM),
                );
            });
        });
}

fn burn_section(ui: &mut egui::Ui, app: &mut App) {
    egui::CollapsingHeader::new(egui::RichText::new("🔥 Branden").strong())
        .default_open(true)
        .show(ui, |ui| {
            let s = &mut app.settings;

            ui.label(egui::RichText::new("Snelheid").strong());
            ui.weak(
                "De drive brandt op de hoogste door media en drive toegelaten \
                 snelheid.",
            );

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Schrijfmodus").strong());
            for mode in [
                WriteMode::Auto,
                WriteMode::Tao,
                WriteMode::Sao,
                WriteMode::Raw,
            ] {
                ui.radio_value(&mut s.write_mode, mode, mode.label());
            }

            ui.add_space(4.0);
            ui.checkbox(&mut s.simulate, "Simulatie (laser uit)");
            ui.checkbox(&mut s.overburn, "Overburn toestaan");
            ui.checkbox(&mut s.underrun_proof, "Buffer-underrun-beveiliging");

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Multi-session").strong());
            egui::ComboBox::from_id_salt("settings.multi_session")
                .selected_text(s.multi_session.label())
                .show_ui(ui, |ui| {
                    for m in [
                        MultiSession::Auto,
                        MultiSession::KeepOpen,
                        MultiSession::Close,
                    ] {
                        ui.selectable_value(&mut s.multi_session, m, m.label());
                    }
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Padding (KiB):");
                ui.add(egui::DragValue::new(&mut s.padding_kib).range(0..=100_000))
                    .on_hover_text(
                        "Schrijft deze hoeveelheid nullen achter de trackdata \
                         (tail). Voor data-discs is 0 normaal; enkele KiB kan \
                         helpen bij bepaalde oude CD-spelers of audio-brandingen. \
                         Heeft geen invloed op multi-session.",
                    );
            });

            ui.add_space(4.0);
            ui.checkbox(&mut s.keep_timestamps, "Originele bestandsdatums behouden")
                .on_hover_text(
                    "Aan: bestanden krijgen hun eigen datum/tijd op de schijf \
                 (Rock Ridge + directoryrecords). Uit: alles krijgt de \
                 opnametijd.",
                );

            ui.add_space(4.0);
            ui.checkbox(&mut s.eject_after, "Schijf uitwerpen na afloop")
                .on_hover_text(
                    "Na een geslaagde brand wordt de schijf uit de drive \
                 genomen. Een aangekoppelde schijf wordt eerst ge-unmount.",
                );
        });
}

fn device_section(ui: &mut egui::Ui, app: &mut App) {
    egui::CollapsingHeader::new(egui::RichText::new("💽 Apparaat").strong())
        .default_open(false)
        .show(ui, |ui| {
            let mut want = app.exclusive_open;
            if ui
                .checkbox(&mut want, "Exclusief openen (O_EXCL)")
                .changed()
            {
                app.set_exclusive_open(want);
            }
            ui.weak(format!(
                "Huidige modus: {}",
                if app.exclusive_open {
                    "exclusief (O_EXCL)"
                } else {
                    "niet-exclusief"
                }
            ));
            ui.weak(
                "Uitzetten als de bestandsbeheerder de schijf aankoppelt \
                 (automount, bijv. Nemo/udisks2).",
            );
            ui.weak("Wijzigen herlaadt libburn en scant opnieuw.");
        });
}
