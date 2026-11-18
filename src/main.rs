#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod checks;
mod config;
mod dns;
mod icons;
mod sysdns;
mod theme;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1220.0, 800.0])
            .with_min_inner_size([1040.0, 680.0])
            .with_title("NeuroFix"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "NeuroFix",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
