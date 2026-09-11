//! Centraal paneel: details van het geselecteerde station + media-inspectie.

use crate::app::{App, LibState, ScanState};
use crate::ffi::DiscStatus;
use crate::ui::colors;
use crate::worker::{
    disc_label, drive_status_label, format_blocks, profile_fallback_name, speed_multiplier_label,
    speed_source_label,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    egui::CentralPanel::default().show(ui, |ui| {
        match &app.lib_state {
            LibState::Loading => {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.spinner();
                    ui.label("libburn wordt geladen…");
                });
            }
            LibState::Failed { error } => {
                let error = error.clone();
                lib_failed_card(ui, app, &error);
            }
            LibState::Loaded { .. } => match app.selected {
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.weak("Selecteer links een station om de details te zien.");
                    });
                }
                Some(i) => {
                    if let Some(d) = app.drives.get(i).cloned() {
                        drive_details(ui, app, &d);
                    }
                }
            },
        };
    });
}

fn lib_failed_card(ui: &mut egui::Ui, app: &mut App, error: &str) {
    ui.group(|ui| {
        ui.colored_label(colors::BAD, "⚠ libburn kon niet geladen worden");
        ui.weak(error);
        ui.separator();
        ui.label(
            "Deze GUI gebruikt de libburn die op je systeem staat (runtime-linking, \
             niet meegecompileerd).",
        );
        ui.label("• Installeer de runtime:  sudo apt install libburn4   (Debian/Ubuntu)");
        ui.label("• Of wijs hieronder een libburn-shared object aan (bijv. /usr/lib/x86_64-linux-gnu/libburn.so.4)");
        ui.label("• Of start met de omgevingsvariabele:  LIBBURN_SO=/pad/naar/libburn.so.4");

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Pad:");
            ui.add(
                egui::TextEdit::singleline(&mut app.custom_so)
                    .hint_text("/usr/lib/x86_64-linux-gnu/libburn.so.4")
                    .desired_width(320.0),
            );
            if ui.button("Opnieuw laden").clicked() && !app.custom_so.trim().is_empty() {
                let path = app.custom_so.trim().to_string();
                app.reload_library(Some(path));
            }
        });
    });
}

fn drive_details(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    ui.heading(format!("📀 {}", d.display_name()));
    ui.weak(format!(
        "Firmware-revisie: {}",
        if d.revision.is_empty() {
            "—"
        } else {
            &d.revision
        }
    ));
    ui.separator();

    egui::Grid::new("drive_info_grid")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Apparaat");
            ui.label(if d.adr.is_empty() {
                "(adres onbekend)".to_string()
            } else {
                d.adr.clone()
            });
            ui.end_row();

            ui.strong("Buffer");
            ui.label(if d.buffer_size_kb > 0 {
                format!("{} KB", d.buffer_size_kb)
            } else {
                "—".to_string()
            });
            ui.end_row();

            ui.strong("TAO-bloktypen");
            ui.monospace(format!("0x{:04X}", d.tao_block_types as u16));
            ui.end_row();

            ui.strong("SAO-bloktypen");
            ui.monospace(format!("0x{:04X}", d.sao_block_types as u16));
            ui.end_row();

            ui.strong("RAW-bloktypen");
            ui.monospace(format!("0x{:04X}", d.raw_block_types as u16));
            ui.end_row();

            ui.strong("Packet-bloktypen");
            ui.monospace(format!("0x{:04X}", d.packet_block_types as u16));
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Mogelijkheden").strong());
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("lezen:").color(colors::DIM));
        for (name, ok) in [
            ("CD-R", d.caps.read_cdr),
            ("CD-RW", d.caps.read_cdrw),
            ("DVD-R", d.caps.read_dvdr),
            ("DVD-RAM", d.caps.read_dvdram),
            ("DVD-ROM", d.caps.read_dvdrom),
            ("C2-fouten", d.caps.c2_errors),
        ] {
            chip(ui, name, ok);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("schrijven:").color(colors::DIM));
        for (name, ok) in [
            ("CD-R", d.caps.write_cdr),
            ("CD-RW", d.caps.write_cdrw),
            ("DVD-R", d.caps.write_dvdr),
            ("DVD-RAM", d.caps.write_dvdram),
        ] {
            chip(ui, name, ok);
        }
        chip(ui, "simulatie", d.caps.write_simulate);
    });

    ui.add_space(12.0);
    media_card(ui, app, d);
}

fn chip(ui: &mut egui::Ui, name: &str, ok: bool) {
    let text = if ok {
        egui::RichText::new(format!("✓ {name}")).color(colors::OK)
    } else {
        egui::RichText::new(format!("✗ {name}")).color(colors::DIM)
    };
    ui.label(text);
}

fn media_card(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Media").strong());
        ui.separator();

        let inspecting = app.busy_drive == Some(d.index);
        let scan_ready = app.scan_state == ScanState::Done;

        if let Some(err) = &d.inspect_error {
            ui.colored_label(colors::BAD, format!("⚠ {err}"));
        }

        ui.horizontal(|ui| {
            if inspecting {
                ui.spinner();
                ui.label("Station inspecteren (grab → status → snelheden → release)…");
            } else {
                if ui
                    .add_enabled(scan_ready, egui::Button::new("🔍 Media inspecteren"))
                    .clicked()
                {
                    app.request_inspect(d.index);
                }
                if !scan_ready {
                    ui.weak("(eerst een scan uitvoeren)");
                }
            }
        });

        if let Some(media) = &d.media {
            ui.add_space(4.0);
            let (disc_color, disc_text) = match media.disc_status {
                DiscStatus::Blank => (colors::OK, "Leeg — klaar om te beschrijven"),
                DiscStatus::Empty => (colors::DIM, "Geen schijf"),
                DiscStatus::Appendable => (colors::WARN, "Onvolledig — extra sessie mogelijk"),
                DiscStatus::Full => (colors::ACCENT, "Vol / afgesloten (alleen lezen)"),
                DiscStatus::Unsuitable => (colors::BAD, "Onbruikbare media"),
                _ => (colors::DIM, disc_label(media.disc_status)),
            };
            ui.colored_label(disc_color, format!("Media: {disc_text}"));
            ui.label(format!(
                "Station: {}",
                drive_status_label(media.drive_status)
            ));

            // Mediatype (SCSI-profiel).
            if !media.profile_name.is_empty() {
                ui.label(format!(
                    "Mediatype: {} (profiel 0x{:02X})",
                    media.profile_name, media.profile_no
                ));
            } else if media.profile_no > 0 {
                let fallback = profile_fallback_name(media.profile_no);
                ui.label(format!(
                    "Mediatype: {} (profiel 0x{:02X})",
                    if fallback.is_empty() {
                        "onbekend"
                    } else {
                        fallback
                    },
                    media.profile_no
                ));
            }
            ui.horizontal_wrapped(|ui| {
                chip(ui, "herbeschrijfbaar", media.erasable);
                if let Some(blocks) = media.read_capacity_blocks {
                    ui.label(format!(
                        "Leesbare capaciteit: {} ({} blokken)",
                        format_blocks(blocks),
                        blocks
                    ));
                }
            });

            ui.add_space(4.0);
            toc_section(ui, media);

            if media.speeds.is_empty() {
                ui.weak("Geen snelheidsinformatie beschikbaar.");
            } else {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Snelheden (kB/s)").strong());
                egui::Grid::new("speeds_grid")
                    .striped(true)
                    .num_columns(5)
                    .spacing([16.0, 3.0])
                    .show(ui, |ui| {
                        ui.strong("Bron");
                        ui.strong("Profiel");
                        ui.strong("Schrijven");
                        ui.strong("Lezen");
                        ui.strong("End-LBA");
                        ui.end_row();
                        for s in &media.speeds {
                            ui.label(speed_source_label(s.source));
                            ui.label(if s.profile_name.is_empty() {
                                "—".to_string()
                            } else {
                                s.profile_name.clone()
                            });
                            ui.label(format!(
                                "{}  {}",
                                s.write_speed,
                                speed_multiplier_label(s.profile_loaded, s.write_speed)
                            ));
                            ui.label(format!(
                                "{}  {}",
                                s.read_speed,
                                speed_multiplier_label(s.profile_loaded, s.read_speed)
                            ));
                            ui.label(if s.end_lba > 0 {
                                s.end_lba.to_string()
                            } else {
                                "—".to_string()
                            });
                            ui.end_row();
                        }
                    });
            }
        } else if !inspecting {
            ui.weak("Nog niet geïnspecteerd — klik op “Media inspecteren”.");
        }
    });
}

/// TOC-weergave: per sessie de tracks met type, startadres en grootte.
fn toc_section(ui: &mut egui::Ui, media: &crate::worker::MediaInfo) {
    if media.sessions.is_empty() {
        match media.disc_status {
            DiscStatus::Empty => {}
            DiscStatus::Blank => {
                ui.weak("Lege media — nog geen TOC.");
            }
            _ => {
                ui.weak("Geen TOC beschikbaar voor deze media.");
            }
        }
        return;
    }

    ui.label(egui::RichText::new("Inhoud (TOC)").strong());
    egui::Grid::new("toc_grid")
        .striped(true)
        .num_columns(6)
        .spacing([16.0, 3.0])
        .show(ui, |ui| {
            ui.strong("Sessie");
            ui.strong("Track");
            ui.strong("Type");
            ui.strong("Start-LBA");
            ui.strong("Blokken");
            ui.strong("≈ Grootte");
            ui.end_row();

            for s in &media.sessions {
                for t in &s.tracks {
                    ui.label(t.session.to_string());
                    ui.label(t.track_no.to_string());
                    ui.label(if t.is_data {
                        if t.copy_permitted {
                            "data · kopie ok"
                        } else {
                            "data"
                        }
                    } else if t.copy_permitted {
                        "audio · kopie ok"
                    } else {
                        "audio"
                    });
                    ui.monospace(t.start_lba.to_string());
                    ui.label(if t.blocks > 0 {
                        t.blocks.to_string()
                    } else {
                        "—".to_string()
                    });
                    ui.label(if t.blocks > 0 {
                        format_blocks(t.blocks)
                    } else {
                        "—".to_string()
                    });
                    ui.end_row();
                }
                ui.label(
                    egui::RichText::new(format!(
                        "sessie {} — LBA {} … {}",
                        s.index + 1,
                        s.start_lba,
                        s.end_lba.saturating_sub(1)
                    ))
                    .small()
                    .color(colors::DIM),
                );
                ui.end_row();
            }
        });

    if media.incomplete_sessions > 0 {
        ui.label(
            egui::RichText::new(format!(
                "⚠ {} onvolledige sessie(s) aanwezig",
                media.incomplete_sessions
            ))
            .small()
            .color(colors::WARN),
        );
    }
}
