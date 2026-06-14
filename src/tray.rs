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
        let show_item = MenuItem::new("Show BS Monitor", true, None);
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
            .with_tooltip("BS Computer Monitor")
            .with_icon(icon)
            .build()
            .ok()?;

        Some(Self { _icon: tray, rx })
    }

    pub fn set_tooltip(&self, text: &str) {
        let _ = self._icon.set_tooltip(Some(text));
    }
}

/// Generates the same 32×32 "BS" bitmap used for the window icon.
fn build_icon() -> Option<tray_icon::Icon> {
    const W: u32 = 32;
    const H: u32 = 32;
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    let bg = [0x07u8, 0x07, 0x0B, 0xFF];
    let fg = [0x38u8, 0x96, 0xD8, 0xFF];
    for c in rgba.chunks_exact_mut(4) { c.copy_from_slice(&bg); }

    const B: [[u8; 5]; 7] = [
        [1,1,1,1,0], [1,0,0,0,1], [1,0,0,0,1],
        [1,1,1,1,0], [1,0,0,0,1], [1,0,0,0,1], [1,1,1,1,0],
    ];
    const S: [[u8; 5]; 7] = [
        [0,1,1,1,1], [1,0,0,0,0], [1,0,0,0,0],
        [0,1,1,1,0], [0,0,0,0,1], [0,0,0,0,1], [1,1,1,1,0],
    ];

    for (glyph, x0) in [(&B as &[[u8; 5]; 7], 3usize), (&S, 16)] {
        for (row, bits) in glyph.iter().enumerate() {
            for (col, &on) in bits.iter().enumerate() {
                if on == 0 { continue; }
                for dy in 0..2usize {
                    for dx in 0..2usize {
                        let (px, py) = (x0 + col * 2 + dx, 9 + row * 2 + dy);
                        if px < W as usize && py < H as usize {
                            let i = (py * W as usize + px) * 4;
                            rgba[i..i + 4].copy_from_slice(&fg);
                        }
                    }
                }
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, W, H).ok()
}
