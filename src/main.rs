// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes
// Version : 2026.10.09

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_variables, unused_imports)]

mod app;
mod collector;
mod fps_collector;
mod icon_art;
mod logging;
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

/// Single-instance guard. A second instance would kill the first one's ETW
/// session (they share the session name) and FindWindowW could target the
/// wrong window. Returns false if another instance already holds the mutex,
/// after restoring/focusing that instance's window.
#[cfg(windows)]
fn single_instance_or_focus_existing() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    let name: Vec<u16> = "Local\\BCComputerMonitor.SingleInstance\0".encode_utf16().collect();
    // Handle intentionally leaked — the OS releases it on process exit.
    let created = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) };
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if created.is_ok() && !already {
        return true;
    }

    let title: Vec<u16> = "BS Computer Monitor\0".encode_utf16().collect();
    unsafe {
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr()));
        if hwnd.0 != 0 {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
    false
}

#[cfg(not(windows))]
fn single_instance_or_focus_existing() -> bool { true }

fn app_display_name() -> &'static str {
    "BS Computer Monitor"
}

fn main() -> eframe::Result<()> {
    // First, before anything can log: a file in %LOCALAPPDATA%, since the release build has no
    // console and a bare env_logger wrote to a stderr nobody could read.
    logging::init();
    logging::install_panic_hook();

    if !single_instance_or_focus_existing() {
        log::info!("another instance is already running — exiting");
        return Ok(());
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BS Computer Monitor")
            .with_inner_size([350.0, 400.0])
            .with_min_inner_size([110.0, 200.0])
            .with_decorations(false)
            .with_transparent(false)
            .with_icon(std::sync::Arc::new(build_icon())),
        ..Default::default()
    };

    eframe::run_native(
        "BS Computer Monitor",
        native_options,
        Box::new(|cc| Ok(Box::new(app::MonitorApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::app_display_name;

    #[test]
    fn app_name_uses_bs_branding() {
        assert_eq!(app_display_name(), "BS Computer Monitor");
    }
}
