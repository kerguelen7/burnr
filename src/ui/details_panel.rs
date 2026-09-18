//! Centraal paneel: details van het geselecteerde station + media-inspectie.

use crate::app::{App, LibState, ScanState};
use crate::ffi::DiscStatus;
use crate::i18n::{CommonTexts, MediaTexts};
use crate::ui::colors;
use crate::worker::{format_blocks, profile_fallback_name, speed_multiplier_label};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let t = app.lang.texts();
    egui::CentralPanel::default().show(ui, |ui| {
        match &app.lib_state {
            LibState::Loading => {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.spinner();
                    ui.label(t.details.loading);
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
                        ui.weak(t.details.select_hint);
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
    let t = app.lang.texts();
    ui.group(|ui| {
        ui.colored_label(colors::BAD, t.details.lib_failed_title);
        ui.weak(error);
        ui.separator();
        ui.label(t.details.lib_failed_body);
        ui.label(t.details.lib_failed_install);
        ui.label(t.details.lib_failed_manual);
        ui.label(t.details.lib_failed_env);

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(t.details.path_label);
            ui.add(
                egui::TextEdit::singleline(&mut app.custom_so)
                    .hint_text("/usr/lib/x86_64-linux-gnu/libburn.so.4")
                    .desired_width(320.0),
            );
            if ui.button(t.details.reload).clicked() && !app.custom_so.trim().is_empty() {
                let path = app.custom_so.trim().to_string();
                app.reload_library(Some(path));
            }
        });
    });
}

fn drive_details(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    let t = app.lang.texts();
    // Compacte kop: naam + apparaatpad; technische details zijn inklapbaar.
    ui.horizontal(|ui| {
        ui.heading(format!("📀 {}", d.display_name()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(if d.adr.is_empty() {
                    t.drives.address_unknown.to_string()
                } else {
                    d.adr.clone()
                })
                .small()
                .color(colors::DIM),
            );
        });
    });

    egui::CollapsingHeader::new(egui::RichText::new(t.details.tech_details).small())
        .default_open(false)
        .show(ui, |ui| {
            ui.weak(format!(
                "{}: {}",
                t.details.firmware_prefix,
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
                    ui.strong(t.details.buffer);
                    ui.label(if d.buffer_size_kb > 0 {
                        format!("{} KB", d.buffer_size_kb)
                    } else {
                        "—".to_string()
                    });
                    ui.end_row();

                    ui.strong(t.details.tao_blocks);
                    ui.monospace(format!("0x{:04X}", d.tao_block_types as u16));
                    ui.end_row();

                    ui.strong(t.details.sao_blocks);
                    ui.monospace(format!("0x{:04X}", d.sao_block_types as u16));
                    ui.end_row();

                    ui.strong(t.details.raw_blocks);
                    ui.monospace(format!("0x{:04X}", d.raw_block_types as u16));
                    ui.end_row();

                    ui.strong(t.details.packet_blocks);
                    ui.monospace(format!("0x{:04X}", d.packet_block_types as u16));
                    ui.end_row();
                });

            ui.add_space(4.0);
            ui.label(egui::RichText::new(t.details.capabilities).strong());
            // Bitvelden dekken CD/DVD-RAM; BD en DVD+R/DVD+RW/DVD-R DL staan
            // alleen in de profiellijst van de drive.
            let has_profile =
                |codes: &[i32]| d.supported_profiles.iter().any(|p| codes.contains(p));
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(t.details.caps_read).color(colors::DIM));
                for (name, ok) in [
                    ("CD-R", d.caps.read_cdr),
                    ("CD-RW", d.caps.read_cdrw),
                    ("DVD-R", d.caps.read_dvdr),
                    ("DVD-RAM", d.caps.read_dvdram),
                    ("DVD-ROM", d.caps.read_dvdrom),
                    ("BD-ROM", has_profile(&[0x40])),
                    (t.details.caps_c2, d.caps.c2_errors),
                ] {
                    chip(ui, name, ok);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(t.details.caps_write).color(colors::DIM));
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
                chip(ui, t.details.caps_simulate, d.caps.write_simulate);
            });
        });

    ui.add_space(8.0);
    media_card(ui, app, d);

    // Eiland 2: branden (bronkeuze + voortgang).
    ui.add_space(8.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            led(ui, app.burn_led, &t.common);
            ui.label(egui::RichText::new(t.burn.island_header).strong());
        });
        burn_section(ui, app, d);
    });

    // Eiland 3: schijfkopie lezen.
    ui.add_space(8.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new(t.image.island_header).strong());
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
fn led(ui: &mut egui::Ui, state: crate::app::JobLed, t: &CommonTexts) {
    use crate::app::JobLed;
    let (color, tip) = match state {
        JobLed::Idle => (colors::DIM, t.led_idle),
        JobLed::Busy => (colors::WARN, t.led_busy),
        JobLed::Ok => (colors::OK, t.led_ok),
        JobLed::Error => (colors::BAD, t.led_error),
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(11.0, 11.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
    resp.on_hover_text(format!("{}: {tip}", t.status_prefix));
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
    let t = app.lang.texts();
    ui.group(|ui| {
        ui.label(egui::RichText::new(t.media.title).strong());
        ui.separator();

        let inspecting = app.busy_drive == Some(d.index);
        let scan_ready = app.scan_state == ScanState::Done;

        if let Some(err) = &d.inspect_error {
            ui.colored_label(colors::BAD, format!("⚠ {err}"));
        }

        ui.horizontal(|ui| {
            if inspecting {
                ui.spinner();
                ui.label(t.media.inspecting);
            } else {
                if ui
                    .add_enabled(scan_ready, egui::Button::new(t.media.inspect_btn))
                    .clicked()
                {
                    app.request_inspect(d.index);
                }
                if !scan_ready {
                    ui.weak(t.media.waiting_for_scan);
                }
            }
        });

        if let Some(media) = &d.media {
            let disc_color = match media.disc_status {
                DiscStatus::Blank => colors::OK,
                DiscStatus::Empty => colors::DIM,
                DiscStatus::Appendable => colors::WARN,
                DiscStatus::Full => colors::ACCENT,
                DiscStatus::Unsuitable => colors::BAD,
                _ => colors::DIM,
            };
            ui.colored_label(
                disc_color,
                format!(
                    "{}: {}",
                    t.media.media_prefix,
                    t.media.disc_status_label(media.disc_status)
                ),
            );
            ui.weak(format!(
                "{}: {}",
                t.media.drive_prefix,
                t.media.drive_status_label(media.drive_status)
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
                        t.media.unknown
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
            let capacity_val = match media.read_capacity_blocks {
                // Lege overbeschrijfbare media (DVD+RW, BD-RE, …): de drive
                // meldt de geformatteerde capaciteit, al staat er geen data op.
                Some(b) if media.disc_status == DiscStatus::Blank => {
                    format!("{} {}", format_blocks(b), t.media.cap_blank_formatted)
                }
                Some(b) => format!("{} ({} {})", format_blocks(b), b, t.media.cap_blocks_suffix),
                None => match media.disc_status {
                    DiscStatus::Empty | DiscStatus::Unready => t.media.cap_no_disc.to_string(),
                    DiscStatus::Blank => t.media.cap_nothing_written.to_string(),
                    DiscStatus::Unsuitable => t.media.cap_unsuitable.to_string(),
                    // Beschreven media zonder leesbare 2048-byte blokken:
                    // typisch CD-audio (wordt nog niet ondersteund).
                    _ => t.media.cap_no_data.to_string(),
                },
            };
            let dm_val = if matches!(media.profile_no, 0x41..=0x43) {
                t.media.dm_label(media.bd_spare)
            } else {
                "—".to_string()
            };

            egui::Grid::new("media_info_grid")
                .num_columns(4)
                .spacing([10.0, 3.0])
                .show(ui, |ui| {
                    ui.weak(t.media.type_label);
                    ui.label(mediatype_val);
                    ui.weak(t.media.media_code);
                    ui.label(mediacode_val);
                    ui.end_row();

                    ui.weak(t.media.rewritable);
                    ui.label(if rewritable { t.media.yes } else { t.media.no });
                    ui.weak(t.media.book_type);
                    ui.label(book_val);
                    ui.end_row();

                    ui.weak(t.media.readable);
                    ui.label(capacity_val).on_hover_text(t.media.readable_hover);
                    ui.weak(t.media.defect_mgmt);
                    ui.label(dm_val);
                    ui.end_row();
                });

            ui.add_space(4.0);
            toc_section(ui, media, &t.media);

            if !media.speeds.is_empty() {
                egui::CollapsingHeader::new(
                    egui::RichText::new(t.media.speeds_header(media.speeds.len())).small(),
                )
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("speeds_grid")
                        .striped(true)
                        .num_columns(5)
                        .spacing([16.0, 3.0])
                        .show(ui, |ui| {
                            ui.strong(t.media.col_source);
                            ui.strong(t.media.col_profile);
                            ui.strong(t.media.col_write);
                            ui.strong(t.media.col_read);
                            ui.strong(t.media.col_end_lba);
                            ui.end_row();
                            for s in &media.speeds {
                                ui.label(t.media.speed_source_label(s.source));
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
            ui.weak(t.media.not_inspected);
        }
    });
}

/// Schijfkopie-sectie: pad + startknop, of voortgang + annuleren tijdens het lezen.
fn read_section(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    let t = app.lang.texts();
    if let Some(r) = app.active_read.clone() {
        if r.index == d.index {
            ui.add(egui::ProgressBar::new(r.fraction()).show_percentage());
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} / {} {}",
                    r.blocks_done, r.total_blocks, t.image.blocks_suffix
                ));
                ui.label(format!("{:.0} kB/s", r.kbps));
                if ui.button(t.common.choose).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_file_name(t.image.default_file)
                        .save_file()
                    {
                        app.read_path = p.display().to_string();
                    }
                }
                if ui.button(t.common.cancel).clicked() {
                    app.cancel_read();
                }
            });
        } else {
            ui.weak(t.image.other_drive_read);
        }
        return;
    }

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.read_path)
                .hint_text("~/kopie.iso")
                .desired_width(180.0),
        );
        if ui.button(t.common.choose).clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name(t.image.default_file)
                .save_file()
            {
                app.read_path = p.display().to_string();
            }
        }
        let ready = app.scan_state == ScanState::Done && app.busy_drive.is_none();
        if ui
            .add_enabled(ready, egui::Button::new(t.image.make_copy_btn))
            .clicked()
        {
            let path = app.read_path.clone();
            app.request_read(d.index, path);
        }
    });
    ui.weak(t.image.copy_hint);
}

/// Brand-sectie: ISO-pad + startknop, of voortgang + annuleren tijdens het branden.
/// Brand-sectie: bronkeuze (ISO-bestand of bestandsselectie), startknop en
/// voortgang met annuleren.
fn burn_section(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    use crate::app::BurnSourceKind;
    let t = app.lang.texts();

    if let Some(b) = app
        .active_burns
        .iter()
        .find(|b| b.index == d.index)
        .cloned()
    {
        if b.index == d.index {
            let frac = b.fraction();
            ui.add(egui::ProgressBar::new(frac).show_percentage());
            // De dataphase kan op 100% eindigen terwijl de drive nog bezig is
            // met lead-out/track-afsluiting; de fase maakt dat zichtbaar.
            if frac >= 1.0 {
                ui.colored_label(
                    colors::WARN,
                    format!("{} {}…", t.burn.done_phase_prefix, b.phase),
                );
            } else {
                ui.label(format!("{}: {}", t.burn.phase_prefix, b.phase));
            }
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} {} / {}",
                    t.burn.sector_word, b.sector, b.sectors
                ));
                ui.label(
                    egui::RichText::new(format!("{:.0} kB/s", b.kbps))
                        .strong()
                        .color(colors::ORANGE),
                );
                ui.label(format!("{} {:.0}%", t.burn.buffer_word, b.buffer_pct));
                ui.label(format!("fifo {:.0}%", b.fifo_pct));
                if ui.button(t.common.cancel).clicked() {
                    app.cancel_burn(d.index);
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} {}",
                    t.burn.elapsed_word,
                    fmt_dur(b.elapsed_secs)
                ));
                if b.eta_secs > 0.0 {
                    ui.label(format!("ETA {}", fmt_dur(b.eta_secs)));
                }
            });
            if b.simulate {
                ui.colored_label(colors::WARN, t.burn.simulate_banner);
            }
        } else {
            ui.weak(t.burn.other_drive_burn);
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

    // Snelheid (stap 9): Maximaal (default) of een lagere maximum gekozen uit
    // de snelheden die de drive zelf meldt voor de ingelegde media — geen
    // vrije waarden, want libburn brandt nooit sneller dan media/drive
    // toelaten. Vooral nuttig voor mindere kwaliteit media.
    let speed_options: Vec<(i32, String)> = d
        .media
        .as_ref()
        .map(|m| {
            let mut opts: Vec<(i32, String)> = m
                .speeds
                .iter()
                .filter(|sp| sp.write_speed > 0)
                .map(|sp| {
                    (
                        sp.write_speed,
                        crate::worker::speed_multiplier_label(sp.profile_loaded, sp.write_speed),
                    )
                })
                .collect();
            opts.sort_by_key(|o| std::cmp::Reverse(o.0));
            opts.dedup_by(|a, b| a.0 == b.0);
            opts
        })
        .unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(t.burn.speed_label);
        let current = if app.settings.speed_max {
            t.burn.speed_max.to_string()
        } else {
            format!(
                "{} {} kB/s",
                t.burn.speed_max_prefix, app.settings.speed_kbps
            )
        };
        egui::ComboBox::from_id_salt("burn_speed")
            .selected_text(current)
            .show_ui(ui, |ui| {
                ui.selectable_label(app.settings.speed_max, t.burn.speed_max)
                    .clicked()
                    .then(|| app.settings.speed_max = true);
                for (kb, mult) in &speed_options {
                    let label = format!("{} {kb} kB/s — {mult}", t.burn.speed_max_prefix);
                    if ui
                        .selectable_label(
                            !app.settings.speed_max && app.settings.speed_kbps == *kb,
                            label,
                        )
                        .clicked()
                    {
                        app.settings.speed_max = false;
                        app.settings.speed_kbps = *kb;
                    }
                }
            });
    });

    // Bronkeuze: ISO-bestand of eigen bestandsselectie (libisofs).
    ui.horizontal(|ui| {
        ui.label(t.burn.source_label);
        ui.radio_value(
            &mut app.burn_source_kind,
            BurnSourceKind::IsoFile,
            t.burn.source_iso,
        );
        ui.radio_value(
            &mut app.burn_source_kind,
            BurnSourceKind::FileSet,
            t.burn.source_files,
        );
    });
    if app.drives.len() > 1 {
        ui.weak(t.burn.multi_drive_hint);
    }

    match app.burn_source_kind {
        BurnSourceKind::IsoFile => {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.burn_path)
                        .hint_text("~/image.iso")
                        .desired_width(180.0),
                );
                if ui.button(t.common.choose).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter(t.burn.iso_filter, &["iso", "img"])
                        .add_filter(t.burn.all_files_filter, &["*"])
                        .pick_file()
                    {
                        app.burn_path = p.display().to_string();
                    }
                }
                if ui
                    .add_enabled(ready, egui::Button::new(t.burn.burn_btn))
                    .clicked()
                {
                    let path = app.burn_path.clone();
                    app.request_burn(d.index, path);
                }
            });
        }
        BurnSourceKind::FileSet => {
            ui.horizontal(|ui| {
                if ui.button(t.burn.add_files).clicked() {
                    // pick_files(): meerdere bestanden kiezen met Ctrl/Shift.
                    if let Some(paths) = rfd::FileDialog::new().pick_files() {
                        for p in paths {
                            app.add_burn_file(p.display().to_string());
                        }
                    }
                }
                if ui.button(t.burn.add_folder).clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        app.add_burn_file(p.display().to_string());
                    }
                }
            });

            if app.burn_files.is_empty() {
                ui.weak(t.burn.no_files);
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
                    "≈ {} ({}, {} {})",
                    crate::worker::format_blocks(app.burn_files_size.div_ceil(2048) as i32),
                    t.burn.estimate_word,
                    app.burn_files.len(),
                    t.burn.items_word
                ));
            }

            ui.horizontal(|ui| {
                ui.label(t.burn.volume_label);
                ui.add(egui::TextEdit::singleline(&mut app.volume_id).desired_width(160.0));
                if ui
                    .add_enabled(
                        ready && !app.burn_files.is_empty(),
                        egui::Button::new(t.burn.compose_burn_btn),
                    )
                    .clicked()
                {
                    app.request_burn_files(d.index);
                }
            });
            ui.weak(t.burn.compose_hint);
            // Multi-session import (stap 7): bij appendable schrijf-eenmalige
            // media wordt de bestaande sessie geïmporteerd.
            if d.media
                .as_ref()
                .is_some_and(|m| m.disc_status == DiscStatus::Appendable)
            {
                ui.colored_label(colors::OK, t.burn.ms_import_hint);
            }
        }
    }

    if !media_ok {
        ui.weak(t.burn.inspect_first);
    }
    let media_cant_simulate = d
        .media
        .as_ref()
        .is_some_and(|m| crate::worker::profile_cant_simulate(m.profile_no));
    if app.settings.simulate && (!d.caps.write_simulate || media_cant_simulate) {
        ui.colored_label(colors::WARN, t.burn.simulate_warn);
    }
    ui.weak(t.burn.uses_settings_hint);
}

/// Onderhoud-sectie (stap 6): losse wis-acties plus de herstelpoging (format).
fn maint_section(ui: &mut egui::Ui, app: &mut App, d: &crate::worker::DriveEntry) {
    use crate::worker::{profile_is_formattable, profile_is_overwritable, profile_is_rewritable};
    let t = app.lang.texts();

    ui.label(egui::RichText::new(t.maint.title).strong());

    if let Some(m) = app
        .active_maints
        .iter()
        .find(|m| m.index == d.index)
        .cloned()
    {
        if m.index == d.index {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("{} {}…", t.media.title, t.maint.kind_label(m.kind)));
                if m.pct > 0.0 {
                    ui.label(format!("{:.0}%", m.pct));
                }
                if ui.button(t.common.cancel).clicked() {
                    app.cancel_maint(d.index);
                }
            });
        } else {
            ui.weak(t.maint.other_drive_maint);
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
        && !app.job_active_on(d.index);

    // Wissen: alleen zinvol op herbeschrijfbare media die niet direct
    // overschrijfbaar is (CD-RW, DVD-RW sequentieel). Overschrijfbare media
    // (DVD+RW, DVD-RAM, BD-RE) is direct beschrijfbaar — wissen is er niet
    // van toepassing; een herstelpoging is er optioneel.
    let can_erase = ready && rewritable && !overwritable;

    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_erase, egui::Button::new(t.maint.erase_quick))
            .clicked()
        {
            app.request_erase(d.index, true);
        }
        if ui
            .add_enabled(can_erase, egui::Button::new(t.maint.erase_full))
            .clicked()
        {
            app.request_erase(d.index, false);
        }
    });

    // Herstelpoging (voorheen "formatteren"): een reddingsactie voor media
    // in een verkeerde toestand. De uitkomst is drive-afhankelijk, daarom
    // achter een uitklapblok en expliciet als poging geformuleerd.
    if formattable {
        ui.add_space(4.0);
        egui::CollapsingHeader::new(egui::RichText::new(t.maint.restore_header).small())
            .default_open(false)
            .show(ui, |ui| {
                ui.weak(t.maint.restore_explain);
                ui.add_space(2.0);
                if ui
                    .add_enabled(ready, egui::Button::new(t.maint.restore_start_btn))
                    .clicked()
                {
                    app.request_format(d.index);
                }
                ui.add_space(2.0);
                ui.weak(t.maint.options_label);

                // Positieve formulering: de schakelaar toont of het mechanisme
                // AAN staat; de instellingen slaan de uitschakel-vlaggen op.
                let mut dm_actief = !app.settings.disable_dm_on_format;
                ui.horizontal(|ui| {
                    crate::ui::toggle_switch(ui, &mut dm_actief);
                    ui.label(t.maint.dm_enable)
                        .on_hover_text(t.maint.dm_enable_hover);
                });
                app.settings.disable_dm_on_format = !dm_actief;

                let mut cert_actief = !app.settings.format_skip_certification;
                ui.horizontal(|ui| {
                    crate::ui::toggle_switch(ui, &mut cert_actief);
                    ui.label(t.maint.cert_enable)
                        .on_hover_text(t.maint.cert_enable_hover);
                });
                app.settings.format_skip_certification = !cert_actief;
            });
    }
    if overwritable {
        ui.weak(t.maint.overwritable_hint);
    } else if !rewritable && !formattable {
        ui.weak(t.maint.not_rewritable_hint);
    }
}

/// TOC-weergave: per sessie de tracks met type, startadres en grootte.
fn toc_section(ui: &mut egui::Ui, media: &crate::worker::MediaInfo, t: &MediaTexts) {
    if media.sessions.is_empty() {
        match media.disc_status {
            DiscStatus::Empty => {}
            DiscStatus::Blank => {
                ui.weak(t.toc_blank);
            }
            _ => {
                ui.weak(t.toc_none);
            }
        }
        return;
    }

    // Inklapbaar: bij multi-session discs groeit de tabel met elke sessie;
    // de kop toont het aantal sessies/tracks zodat het eiland compact blijft.
    let total_tracks: usize = media.sessions.iter().map(|s| s.tracks.len()).sum();
    let header = t.toc_header(media.sessions.len(), total_tracks);
    egui::CollapsingHeader::new(egui::RichText::new(header).small())
        .default_open(media.sessions.len() <= 3)
        .show(ui, |ui| {
            egui::Grid::new("toc_grid")
                .striped(true)
                .num_columns(6)
                .spacing([16.0, 3.0])
                .show(ui, |ui| {
                    ui.strong(t.col_session);
                    ui.strong(t.col_track);
                    ui.strong(t.col_type);
                    ui.strong(t.col_start_lba);
                    ui.strong(t.col_blocks);
                    ui.strong(t.col_size);
                    ui.end_row();

                    for s in &media.sessions {
                        for track in &s.tracks {
                            ui.label(track.session.to_string());
                            ui.label(track.track_no.to_string());
                            ui.label(if track.is_data {
                                if track.copy_permitted {
                                    t.toc_data_copy
                                } else {
                                    t.toc_data
                                }
                            } else if track.copy_permitted {
                                t.toc_audio_copy
                            } else {
                                t.toc_audio
                            });
                            ui.monospace(track.start_lba.to_string());
                            ui.label(if track.blocks > 0 {
                                track.blocks.to_string()
                            } else {
                                "—".to_string()
                            });
                            ui.label(if track.blocks > 0 {
                                format_blocks(track.blocks)
                            } else {
                                "—".to_string()
                            });
                            ui.end_row();
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {} — LBA {} … {}",
                                t.toc_session_word,
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
                        "⚠ {} {}",
                        media.incomplete_sessions, t.toc_incomplete
                    ))
                    .small()
                    .color(colors::WARN),
                );
            }
        });
}
