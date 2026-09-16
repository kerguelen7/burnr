//! LibBurn GUI — een moderne egui-interface rond de systeem-libburn.
//!
//! libburn wordt tijdens runtime geladen (`libburn.so.4`), niet meegecompileerd.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ffi;
mod isofs;
mod logger;
mod settings;
mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([940.0, 600.0])
            // Wayland: koppelt het venster aan de toekomstige
            // libburn_gui.desktop (icoon in de dock/taskbar).
            .with_app_id("libburn_gui")
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "LibBurn GUI",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

/// Decodeert het ingebedde venstericoon (assets/icon.png) naar ruwe RGBA
/// voor `egui::IconData`. Het bestand zit in de binary (include_bytes!),
/// dus een leesfout is een build-probleem en mag hard falen.
fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("ingebed venstericoon (assets/icon.png) is geen geldige PNG")
        .into_rgba8();
    let (width, height) = img.dimensions();
    egui::IconData {
        width,
        height,
        rgba: img.into_raw(),
    }
}
