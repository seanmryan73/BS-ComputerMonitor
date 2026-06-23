// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes
// Version : 2026.06.23

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_variables, unused_imports)]

mod app;
mod collector;
mod fps_collector;
mod models;
mod ping_collector;
mod theme;
#[cfg(windows)]
mod tray;
mod ui;

fn build_icon() -> egui::IconData {
    const W: usize = 32;
    const H: usize = 32;
    let mut rgba = vec![0u8; W * H * 4];

    let bg: [u8; 4] = [0x07, 0x07, 0x0B, 0xFF];
    let fg: [u8; 4] = [0xFF, 0x14, 0x93, 0xFF]; // neon pink accent

    for chunk in rgba.chunks_exact_mut(4) {
        chunk.copy_from_slice(&bg);
    }

    // 5×7 bitmap glyphs — drawn at 2× scale (each pixel = 2×2 block)
    const B: [[u8; 5]; 7] = [
        [1,1,1,1,0],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,1,1,1,0],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,1,1,1,0],
    ];
    const C: [[u8; 5]; 7] = [
        [0,1,1,1,0],
        [1,0,0,0,0],
        [1,0,0,0,0],
        [1,0,0,0,0],
        [1,0,0,0,0],
        [1,0,0,0,0],
        [0,1,1,1,0],
    ];

    // B at x=3, C at x=16, both at y=9 — each glyph 10×14 px at 2× scale
    for (glyph, x0) in [(&B as &[[u8; 5]; 7], 3usize), (&C, 16usize)] {
        for (row, bits) in glyph.iter().enumerate() {
            for (col, &on) in bits.iter().enumerate() {
                if on == 0 { continue; }
                for dy in 0..2usize {
                    for dx in 0..2usize {
                        let px = x0 + col * 2 + dx;
                        let py = 9 + row * 2 + dy;
                        if px < W && py < H {
                            let i = (py * W + px) * 4;
                            rgba[i..i + 4].copy_from_slice(&fg);
                        }
                    }
                }
            }
        }
    }

    egui::IconData { rgba, width: W as u32, height: H as u32 }
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
