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
    // Compacte kop: naam + apparaatpad; technische details zijn inklapbaar.
    ui.horizontal(|ui| {
        ui.heading(format!("📀 {}", d.display_name()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(if d.adr.is_empty() {
                    "(adres onbekend)".to_string()
                } else {
                    d.adr.clone()
                })
                .small()
                .color(colors::DIM),
            );
        });
    });

    egui::CollapsingHeader::new(egui::RichText::new("⚙ Technische details").small())
        .default_open(false)
        .show(ui, |ui| {
            ui.weak(format!(
                "Firmware-revisie: {}",
                if d.revision.is_empty() {
                    "—"
                } else {
                    &d.revision
                }
            ));
            egui::Grid::new("drive_info_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
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

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Mogelijkheden").strong());
            // Bitvelden dekken CD/DVD-RAM; BD en DVD+R/DVD+RW/DVD-R DL staan
            // alleen in de profiellijst van de drive.
            let has_profile =
                |codes: &[i32]| d.supported_profiles.iter().any(|p| codes.contains(p));
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("lezen:").color(colors::DIM));
                for (name, ok) in [
                    ("CD-R", d.caps.read_cdr),
                    ("CD-RW", d.caps.read_cdrw),
                    ("DVD-R", d.caps.read_dvdr),
                    ("DVD-RAM", d.caps.read_dvdram),
                    ("DVD-ROM", d.caps.read_dvdrom),
                    ("BD-ROM", has_profile(&[0x40])),
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
                    ("DVD+R", has_profile(&[0x1B])),
                    ("DVD+RW", has_profile(&[0x1A])),
                    ("DVD-R DL", has_profile(&[0x15, 0x16])),
                    ("DVD+R DL", has_profile(&[0x2B])),
                    ("BD-R", has_profile(&[0x41, 0x42])),
                    ("BD-RE", has_profile(&[0x43])),
                ] {
                    chip(ui, name, ok);
                }
                chip(ui, "simulatie", d.caps.write_simulate);
            });
        });

    ui.add_space(8.0);
    media_card(ui, app, d);

    // Eiland 2: branden (bronkeuze + voortgang).
    ui.add_space(8.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            led(ui, app.burn_led);
            ui.label(egui::RichText::new("🔥 Branden").strong());
        });
        burn_section(ui, app, d);
    });

    // Eiland 3: schijfkopie lezen.
    ui.add_space(8.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new("💾 Schijfkopie (data)").strong());
        read_section(ui, app, d);
    });
}

fn chip(ui: &mut egui::Ui, name: &str, ok: bool) {
    let text = if ok {
        egui::RichText::new(format!("✓ {name}")).color(colors::OK)
    } else {
        egui::RichText::new(format!("✗ {name}")).color(colors::DIM)
    };
    ui.label(text);
}

/// Status-LED: groen = klaar/geslaagd, oranje = bezig, rood = fout,
/// grijs = inactief.
fn led(ui: &mut egui::Ui, state: crate::app::JobLed) {
    use crate::app::JobLed;
    let (color, tip) = match state {
        JobLed::Idle => (colors::DIM, "inactief"),
        JobLed::Busy => (colors::WARN, "bezig…"),
        JobLed::Ok => (colors::OK, "klaar — geslaagd"),
        JobLed::Error => (colors::BAD, "mislukt"),
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(11.0, 11.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
    resp.on_hover_text(format!("Status: {tip}"));
}

/// Formatteert seconden als m:ss of h:mm:ss.
fn fmt_dur(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m:02}:{sec:02}")
    }
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
            let (disc_color, disc_text) = match media.disc_status {
                DiscStatus::Blank => (colors::OK, "Leeg — klaar om te beschrijven"),
                DiscStatus::Empty => (colors::DIM, "Geen schijf"),
                DiscStatus::Appendable => (colors::WARN, "Onvolledig — extra sessie mogelijk"),
                DiscStatus::Full => (colors::ACCENT, "Vol / afgesloten (alleen lezen)"),
                DiscStatus::Unsuitable => (colors::BAD, "Onbruikbare media"),
                _ => (colors::DIM, disc_label(media.disc_status)),
            };
            ui.colored_label(disc_color, format!("Media: {disc_text}"));
            ui.weak(format!(
                "Station: {}",
                drive_status_label(media.drive_status)
            ));

            // Compacte info in twee kolommen (label–waarde-paren naast elkaar).
            // De Media-codes (media_code1/2) zijn redundant met de mediacode
            // en zijn daarom weggelaten.
            let mediatype_val = if !media.profile_name.is_empty() {
                format!("{} (0x{:02X})", media.profile_name, media.profile_no)
            } else if media.profile_no > 0 {
                let fallback = profile_fallback_name(media.profile_no);
                format!(
                    "{} (0x{:02X})",
                    if fallback.is_empty() {
                        "onbekend"
                    } else {
                        fallback
                    },
                    media.profile_no
                )
            } else {
                "—".to_string()
            };

            let mediacode_val = media
                .media_id
                .as_ref()
                .map(|mid| {
                    let manu = mid
                        .manufacturer
                        .as_ref()
                        .map(|m| format!(" — {m}"))
                        .unwrap_or_default();
                    format!("{}{manu}", mid.product_id)
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "—".to_string());

            let rewritable =
                media.erasable || crate::worker::profile_is_rewritable(media.profile_no);
            let book_val = media
                .media_id
                .as_ref()
                .and_then(|mid| mid.book_type.clone())
                .unwrap_or_else(|| "—".to_string());
            let capacity_val = media
                .read_capacity_blocks
                .map(|b| format!("{} ({} blokken)", format_blocks(b), b))
                .unwrap_or_else(|| "— (lege schijf)".to_string());
            let dm_val = if matches!(media.profile_no, 0x41 | 0x42 | 0x43) {
                match media.bd_spare {
                    Some((alloc, free)) => {
                        format!("actief (vrij {free} van {alloc})")
                    }
                    None => "uit".to_string(),
                }
            } else {
                "—".to_string()
            };

            egui::Grid::new("media_info_grid")
                .num_columns(4)
                .spacing([10.0, 3.0])
                .show(ui, |ui| {
                    ui.weak("Type:");
                    ui.label(mediatype_val);
                    ui.weak("Mediacode:");
                    ui.label(mediacode_val);
                    ui.end_row();

                    ui.weak("Herbeschrijfbaar:");
                    ui.label(if rewritable { "ja" } else { "nee" });
                    ui.weak("Book type:");
                    ui.label(book_val);
                    ui.end_row();

                    ui.weak("Capaciteit (leesbaar):");
                    ui.label(capacity_val).on_hover_text(
                        "Hoeveel data er leesbaar op de schijf staat. Op een lege \
                         schrijf-eenmalige schijf is er niets te lezen ('-'); op \
                         RW/RE-media meldt de drive de geformatteerde capaciteit.",
                    );
                    ui.weak("Defect mgmt:");
                    ui.label(dm_val);
                    ui.end_row();
                });

            ui.add_space(4.0);
            toc_section(ui, media);

            if !media.speeds.is_empty() {
                egui::CollapsingHeader::new(
                    egui::RichText::new(format!("Snelheden ({} entrees)", media.speeds.len()))
                        .small(),
                )
                .default_open(false)
                .show(ui, |ui| {
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
                });
            }

            ui.add_space(6.0);
            ui.separator();
            maint_section(ui, app, d);
        } else if !inspecting {
            ui.weak("Nog niet geïnspecteerd — klik op “Media inspecteren”.");
        }
    });
}

/// Schijfkopie-sectie: pad + startknop, of voortgang + annuleren tijdens het lezen.
fn read_section(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    if let Some(r) = app.active_read.clone() {
        if r.index == d.index {
            ui.add(egui::ProgressBar::new(r.fraction()).show_percentage());
            ui.horizontal(|ui| {
                ui.label(format!("{} / {} blokken", r.blocks_done, r.total_blocks));
                ui.label(format!("{:.0} kB/s", r.kbps));
                if ui.button("📄 Kies…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_file_name("kopie.iso")
                        .save_file()
                    {
                        app.read_path = p.display().to_string();
                    }
                }
                if ui.button("⏹ Annuleren").clicked() {
                    app.cancel_read();
                }
            });
        } else {
            ui.weak("Er draait momenteel een kopie op een ander station.");
        }
        return;
    }

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.read_path)
                .hint_text("~/kopie.iso")
                .desired_width(180.0),
        );
        if ui.button("📄 Kies…").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name("kopie.iso")
                .save_file()
            {
                app.read_path = p.display().to_string();
            }
        }
        let ready = app.scan_state == ScanState::Done && app.busy_drive.is_none();
        if ui
            .add_enabled(ready, egui::Button::new("💾 Kopie maken"))
            .clicked()
        {
            let path = app.read_path.clone();
            app.request_read(d.index, path);
        }
    });
    ui.weak(
        "Leest alle datablokken (2048 B) naar één bestand — geschikt voor \
         CD/DVD/BD-data, niet voor CD-audio.",
    );
}

/// Brand-sectie: ISO-pad + startknop, of voortgang + annuleren tijdens het branden.
/// Brand-sectie: bronkeuze (ISO-bestand of bestandsselectie), startknop en
/// voortgang met annuleren.
fn burn_section(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    use crate::app::BurnSourceKind;

    if let Some(b) = app.active_burn.clone() {
        if b.index == d.index {
            let frac = b.fraction();
            ui.add(egui::ProgressBar::new(frac).show_percentage());
            // De dataphase kan op 100% eindigen terwijl de drive nog bezig is
            // met lead-out/track-afsluiting; de fase maakt dat zichtbaar.
            if frac >= 1.0 {
                ui.colored_label(
                    colors::WARN,
                    format!("100% geschreven — drive is nog bezig: {}…", b.phase),
                );
            } else {
                ui.label(format!("Fase: {}", b.phase));
            }
            ui.horizontal(|ui| {
                ui.label(format!("sector {} / {}", b.sector, b.sectors));
                ui.label(format!("{:.0} kB/s", b.kbps));
                ui.label(format!("buffer {:.0}%", b.buffer_pct));
                ui.label(format!("fifo {:.0}%", b.fifo_pct));
                if ui.button("⏹ Annuleren").clicked() {
                    app.cancel_burn();
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("⏱ verstreken {}", fmt_dur(b.elapsed_secs)));
                if b.eta_secs > 0.0 {
                    ui.label(format!("ETA {}", fmt_dur(b.eta_secs)));
                }
            });
            if b.simulate {
                ui.colored_label(
                    colors::WARN,
                    "SIMULATIE — er wordt niets definitief geschreven",
                );
            }
        } else {
            ui.weak("Er draait momenteel een brandjob op een ander station.");
        }
        return;
    }

    let media_ok = d
        .media
        .as_ref()
        .is_some_and(|m| matches!(m.disc_status, DiscStatus::Blank | DiscStatus::Appendable));
    let ready = app.scan_state == ScanState::Done
        && app.busy_drive.is_none()
        && app.active_read.is_none()
        && media_ok;

    // Bronkeuze: ISO-bestand of eigen bestandsselectie (libisofs).
    ui.horizontal(|ui| {
        ui.label("Bron:");
        ui.radio_value(
            &mut app.burn_source_kind,
            BurnSourceKind::IsoFile,
            "ISO-bestand",
        );
        ui.radio_value(
            &mut app.burn_source_kind,
            BurnSourceKind::FileSet,
            "Bestanden",
        );
    });

    match app.burn_source_kind {
        BurnSourceKind::IsoFile => {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.burn_path)
                        .hint_text("~/image.iso")
                        .desired_width(180.0),
                );
                if ui.button("📄 Kies…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("ISO-images", &["iso", "img"])
                        .add_filter("Alle bestanden", &["*"])
                        .pick_file()
                    {
                        app.burn_path = p.display().to_string();
                    }
                }
                if ui
                    .add_enabled(ready, egui::Button::new("🔥 Branden"))
                    .clicked()
                {
                    let path = app.burn_path.clone();
                    app.request_burn(d.index, path);
                }
            });
        }
        BurnSourceKind::FileSet => {
            ui.horizontal(|ui| {
                if ui.button("➕ Bestanden…").clicked() {
                    // pick_files(): meerdere bestanden kiezen met Ctrl/Shift.
                    if let Some(paths) = rfd::FileDialog::new().pick_files() {
                        for p in paths {
                            app.add_burn_file(p.display().to_string());
                        }
                    }
                }
                if ui.button("📁 Map…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        app.add_burn_file(p.display().to_string());
                    }
                }
            });

            if app.burn_files.is_empty() {
                ui.weak("Nog geen bestanden gekozen — voeg bestanden of mappen toe.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        let files = app.burn_files.clone();
                        for (i, p) in files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.small_button("✖").clicked() {
                                    app.remove_burn_file(i);
                                }
                                ui.label(egui::RichText::new(p).small());
                            });
                        }
                    });
                ui.label(format!(
                    "≈ {} (schatting, {} item(s))",
                    crate::worker::format_blocks(((app.burn_files_size + 2047) / 2048) as i32),
                    app.burn_files.len()
                ));
            }

            ui.horizontal(|ui| {
                ui.label("Volume-naam:");
                ui.add(egui::TextEdit::singleline(&mut app.volume_id).desired_width(160.0));
                if ui
                    .add_enabled(
                        ready && !app.burn_files.is_empty(),
                        egui::Button::new("🔥 Samenstellen & branden"),
                    )
                    .clicked()
                {
                    app.request_burn_files(d.index);
                }
            });
            ui.weak(
                "Maakt een ISO9660-image (Rock Ridge + Joliet, ISO-niveau 3) met \
                 libisofs en brandt die direct — geen tussenbestand.",
            );
        }
    }

    if !media_ok {
        ui.weak("(inspecteer eerst; branden vereist lege of onvolledige media)");
    }
    let media_cant_simulate = d
        .media
        .as_ref()
        .is_some_and(|m| crate::worker::profile_cant_simulate(m.profile_no));
    if app.settings.simulate && (!d.caps.write_simulate || media_cant_simulate) {
        ui.colored_label(
            colors::WARN,
            "⚠ Simulatie staat aan, maar deze drive/media kan niet simuleren \
             (alle BD-media, DVD-R DL en overbeschrijfbare media kunnen nooit \
             simuleren). Zet “Simulatie” uit in Instellingen → Branden — \
             let op: zonder simulatie wordt er écht geschreven.",
        );
    }
    ui.weak(
        "Gebruikt de instellingen rechts: snelheid, schrijfmodus, simulatie, \
         multi-session, padding, wissen vooraf.",
    );
}

/// Onderhoud-sectie (stap 6): losse wis- en format-acties op de media.
fn maint_section(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    use crate::worker::{profile_is_formattable, profile_is_overwritable, profile_is_rewritable};

    ui.label(egui::RichText::new("Onderhoud").strong());

    if let Some(m) = app.active_maint.clone() {
        if m.index == d.index {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Media {}…", m.kind.label()));
                if m.pct > 0.0 {
                    ui.label(format!("{:.0}%", m.pct));
                }
                if ui.button("⏹ Annuleren").clicked() {
                    app.cancel_maint();
                }
            });
        } else {
            ui.weak("Er draait een onderhoudsjob op een ander station.");
        }
        return;
    }

    let profile = d.media.as_ref().map(|m| m.profile_no).unwrap_or(0);
    let rewritable = d
        .media
        .as_ref()
        .is_some_and(|m| m.erasable || profile_is_rewritable(m.profile_no));
    let overwritable = profile_is_overwritable(profile);
    let formattable = profile_is_formattable(profile);
    let ready = app.scan_state == ScanState::Done
        && app.busy_drive.is_none()
        && app.active_read.is_none()
        && app.active_burn.is_none();

    // Wissen: alleen zinvol op herbeschrijfbare media die niet direct
    // overschrijfbaar is (CD-RW, DVD-RW sequentieel). Voor overschrijfbare
    // media (DVD+RW, DVD-RAM, BD-RE) is formatteren de juiste actie.
    let can_erase = ready && rewritable && !overwritable;

    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_erase, egui::Button::new("🧽 Wissen (snel)"))
            .clicked()
        {
            app.request_erase(d.index, true);
        }
        if ui
            .add_enabled(can_erase, egui::Button::new("🧽 Wissen (volledig)"))
            .clicked()
        {
            app.request_erase(d.index, false);
        }
        if ui
            .add_enabled(ready && formattable, egui::Button::new("⚙ Formatteren"))
            .on_hover_text(
                "Volledige format: wist alle data op de schijf en herstelt de \
                 format-structuren. Kan enkele minuten of langer duren.",
            )
            .clicked()
        {
            app.request_format(d.index);
        }
    });

    // Defect management uitschakelen bij het formatteren (stap 8) — alleen
    // relevant voor media met DM (BD, DVD-RAM).
    if formattable && matches!(profile, 0x12 | 0x41 | 0x42 | 0x43) {
        ui.checkbox(
            &mut app.settings.disable_dm_on_format,
            "Defect management uitschakelen",
        )
        .on_hover_text(
            "Bij het formatteren: geen spare-gebieden en geen hermapping van \
             slechte blokken. Branden gaat daarna een stuk sneller — gebruik \
             dit alleen op betrouwbare media.",
        );
    }
    if !ready {
        ui.weak("(eerst een scan uitvoeren)");
    } else if overwritable {
        ui.weak("Direct overschrijfbare media hoeft niet gewist te worden — formatteren herstelt de schijf.");
    } else if !rewritable && !formattable {
        ui.weak("Deze media is niet herbeschrijfbaar — wissen/formatteren is niet mogelijk.");
    }
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
