#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod collector;
mod fps_collector;
mod models;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BS Computer Monitor")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([720.0, 480.0])
            .with_decorations(false)
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "BS Computer Monitor",
        native_options,
        Box::new(|cc| Ok(Box::new(app::MonitorApp::new(cc)))),
    )
}
