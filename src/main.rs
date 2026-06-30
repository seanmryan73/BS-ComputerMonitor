// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes
// Version : 2026.06.23

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_variables, unused_imports)]

mod app;
mod collector;
mod fps_collector;
mod icon_art;
mod models;
mod ping_collector;
mod theme;
#[cfg(windows)]
mod tray;
mod ui;

fn build_icon() -> egui::IconData {
    const SIZE: u32 = 32;
    egui::IconData { rgba: icon_art::draw_icon_rgba(SIZE), width: SIZE, height: SIZE }
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BC Computer Monitor")
            .with_inner_size([350.0, 400.0])
            .with_min_inner_size([110.0, 200.0])
            .with_decorations(false)
            .with_transparent(false)
            .with_icon(std::sync::Arc::new(build_icon())),
        ..Default::default()
    };

    eframe::run_native(
        "BC Computer Monitor",
        native_options,
        Box::new(|cc| Ok(Box::new(app::MonitorApp::new(cc)))),
    )
}
