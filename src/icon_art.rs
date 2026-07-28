// Shared pixel-art generator for the app icon: window/taskbar icon
// (src/main.rs), system tray icon (src/tray.rs), and the embedded .ico
// resource (build.rs, via `include!` — build scripts can't depend on the
// main crate, so this file stays free of egui/tray_icon/ico imports).

const BG: [u8; 4] = [0x07, 0x07, 0x0B, 0xFF];
const BORDER: [u8; 4] = [0x00, 0xFF, 0xDD, 0xFF]; // neon mint — complementary accent

const BAR_COLORS: [[u8; 4]; 5] = [
    [0xFF, 0x2E, 0x9F, 0xFF], // neon pink
    [0x00, 0xFF, 0xDD, 0xFF], // neon mint
    [0x8A, 0x2E, 0xFF, 0xFF], // electric purple
    [0xFF, 0xD2, 0x3F, 0xFF], // gold
    [0x2E, 0x8B, 0xFF, 0xFF], // electric blue
];
const BAR_FRACTIONS: [f64; 5] = [0.9, 0.55, 1.0, 0.4, 0.7];

/// Draws a stack of horizontal bars, each a different color and length
/// (like a horizontal equalizer/bar-chart), plus a neon-mint top/right
/// border, on a `size`x`size` canvas.
pub fn draw_icon_rgba(size: u32) -> Vec<u8> {
    let size = size as usize;
    let mut rgba = vec![0u8; size * size * 4];
    for chunk in rgba.chunks_exact_mut(4) {
        chunk.copy_from_slice(&BG);
    }

    let thickness = ((size / 16).max(1)).min(size);
    let margin = thickness;
    let content_x0 = margin;
    let content_x1 = size.saturating_sub(thickness).max(content_x0);
    let content_y0 = thickness;
    let content_y1 = size.saturating_sub(margin).max(content_y0);
    let content_w = content_x1 - content_x0;
    let content_h = content_y1 - content_y0;

    let bars = BAR_COLORS.len();
    let slot_h = (content_h / bars).max(1);
    let bar_h = (slot_h.saturating_sub(1)).max(1);
    for (i, (color, frac)) in BAR_COLORS.iter().zip(BAR_FRACTIONS.iter()).enumerate() {
        let y0 = content_y0 + i * slot_h;
        let y1 = (y0 + bar_h).min(content_y1);
        let w = ((content_w as f64) * frac).round() as usize;
        let x1 = (content_x0 + w).min(content_x1);
        for py in y0..y1 {
            for px in content_x0..x1 {
                let idx = (py * size + px) * 4;
                rgba[idx..idx + 4].copy_from_slice(color);
            }
        }
    }

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
