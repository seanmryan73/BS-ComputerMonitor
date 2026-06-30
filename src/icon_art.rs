// Shared pixel-art generator for the app icon: window/taskbar icon
// (src/main.rs), system tray icon (src/tray.rs), and the embedded .ico
// resource (build.rs, via `include!` — build scripts can't depend on the
// main crate, so this file stays free of egui/tray_icon/ico imports).

const BG: [u8; 4] = [0x07, 0x07, 0x0B, 0xFF];
const FG: [u8; 4] = [0xFF, 0x14, 0x93, 0xFF]; // neon pink accent
const BORDER: [u8; 4] = [0x00, 0xFF, 0xDD, 0xFF]; // neon mint — complementary accent

const GLYPH_B: [[u8; 5]; 7] = [
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
];
const GLYPH_C: [[u8; 5]; 7] = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [0, 1, 1, 1, 0],
];

/// Draws the "BC" glyph pair plus a neon-mint top/right border on a
/// `size`x`size` canvas, scaling the 32x32 reference layout (block scale 2,
/// glyph origins at x=3/16, y=9) up or down.
pub fn draw_icon_rgba(size: u32) -> Vec<u8> {
    let size = size as usize;
    let scale = size as f64 / 32.0;
    let mut rgba = vec![0u8; size * size * 4];
    for chunk in rgba.chunks_exact_mut(4) {
        chunk.copy_from_slice(&BG);
    }

    let block = ((2.0 * scale).round() as usize).max(1);
    for (glyph, x0_ref) in [(&GLYPH_B, 3.0), (&GLYPH_C, 16.0)] {
        let x0 = (x0_ref * scale).round() as usize;
        let y0 = (9.0 * scale).round() as usize;
        for (row, bits) in glyph.iter().enumerate() {
            for (col, &on) in bits.iter().enumerate() {
                if on == 0 {
                    continue;
                }
                for dy in 0..block {
                    for dx in 0..block {
                        let px = x0 + col * block + dx;
                        let py = y0 + row * block + dy;
                        if px < size && py < size {
                            let i = (py * size + px) * 4;
                            rgba[i..i + 4].copy_from_slice(&FG);
                        }
                    }
                }
            }
        }
    }

    let thickness = ((size / 16).max(1)).min(size);
    for py in 0..thickness {
        for px in 0..size {
            let i = (py * size + px) * 4;
            rgba[i..i + 4].copy_from_slice(&BORDER);
        }
    }
    for py in 0..size {
        for px in (size - thickness)..size {
            let i = (py * size + px) * 4;
            rgba[i..i + 4].copy_from_slice(&BORDER);
        }
    }

    rgba
}
