//! LibBurn GUI — een moderne egui-interface rond de systeem-libburn.
//!
//! libburn wordt tijdens runtime geladen (`libburn.so.4`), niet meegecompileerd.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ffi;
mod logger;
mod settings;
mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([940.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "LibBurn GUI",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
