//! Windows system tray icon — live stats tooltip, right-click Exit / Show.

use std::sync::mpsc;

use tray_icon::{
    MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub enum TrayCmd {
    ShowWindow,
    Exit,
}

pub struct TrayHandle {
    _icon: TrayIcon, // must remain alive for the tray icon to persist
    pub rx: mpsc::Receiver<TrayCmd>,
}

impl TrayHandle {
    pub fn build() -> Option<Self> {
        let (tx, rx) = mpsc::channel::<TrayCmd>();

        // Menu items
        let show_item = MenuItem::new("Show BC Monitor", true, None);
        let exit_item = MenuItem::new("Exit", true, None);
        let show_id   = show_item.id().clone();
        let exit_id   = exit_item.id().clone();

        let menu = Menu::new();
        let sep  = PredefinedMenuItem::separator();
        let _    = menu.append_items(&[&show_item, &sep, &exit_item]);

        // Menu click → channel
        let tx_menu = tx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if e.id == show_id {
                let _ = tx_menu.send(TrayCmd::ShowWindow);
            } else if e.id == exit_id {
                let _ = tx_menu.send(TrayCmd::Exit);
            }
        }));

        // Left-click on tray icon → show window
        let tx_tray = tx;
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = e {
                let _ = tx_tray.send(TrayCmd::ShowWindow);
            }
        }));

        let icon = build_icon()?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("BC Computer Monitor")
            .with_icon(icon)
            .build()
            .ok()?;

        Some(Self { _icon: tray, rx })
    }

    pub fn set_tooltip(&self, text: &str) {
        let _ = self._icon.set_tooltip(Some(text));
    }
}

/// Generates the same 32×32 "CM" bitmap used for the window icon.
fn build_icon() -> Option<tray_icon::Icon> {
    const SIZE: u32 = 32;
    tray_icon::Icon::from_rgba(crate::icon_art::draw_icon_rgba(SIZE), SIZE, SIZE).ok()
}
