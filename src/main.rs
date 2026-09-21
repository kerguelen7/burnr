//! Burnr — een lichte egui-interface rond de systeem-libburn.
//!
//! libburn wordt tijdens runtime geladen (`libburn.so.4`), niet meegecompileerd.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ffi;
mod i18n;
mod isofs;
mod logger;
mod raii;
mod settings;
mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    // Optioneel bestandspad van de bestandsbeheerder (rechtsklik → openen
    // met Burnr): Exec=burnr %f in burnr.desktop geeft het .iso-pad door.
    let cli_iso = std::env::args().nth(1);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([940.0, 600.0])
            // Wayland: koppelt het venster aan de toekomstige
            // burnr.desktop (icoon in de dock/taskbar).
            .with_app_id("burnr")
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Burnr",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, cli_iso)))),
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
