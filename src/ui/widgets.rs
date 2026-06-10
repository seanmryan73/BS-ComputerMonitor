//! Reusable low-level widgets: spectrum bars, gradient bar, sparkline, dot gauge.

use egui::{
    Color32, Frame, Margin, Rect, Rounding, Sense, Stroke, Ui, Vec2,
};

use crate::theme::Theme;

// ── Spectrum bar panel ────────────────────────────────────────────────────────

/// Renders the history as a mini spectrum analyser panel:
/// glowing bars + bright cap + two-tier reflection + peak-hold dots at local maxima.
///
/// `max_val`    — value that maps to full bar height (e.g. 100.0 for percentages).
/// `accent_end` — when `Some`, bars get a left→right colour gradient from `accent` to `accent_end`.
/// `color_fn`   — either [`vu_color`] (high = warn) or [`fps_color`] (high = good).
pub fn spectrum_bars(
    ui: &mut Ui,
    hist: &[f64],
    max_val: f32,
    accent: Color32,
    accent_end: Option<Color32>,
    height: f32,
    color_fn: fn(f32, Color32) -> Color32,
) {
    let n = hist.len();
    if n == 0 {
        return;
    }

    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, height), Sense::hover());
    let painter = ui.painter();

    let gap = 1.5f32;
    let bar_w = ((avail_w + gap) / n as f32 - gap).max(1.0);

    // Floor splits the rect: top 65 % = bars, bottom 35 % = reflection
    let split_y = rect.min.y + rect.height() * 0.65;
    let bar_h_range = split_y - rect.min.y;
    let refl_h_range = rect.max.y - split_y;

    // Faint grid lines at 25 / 50 / 75 %
    for frac in [0.25f32, 0.50, 0.75] {
        let y = split_y - frac * bar_h_range;
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(80, 80, 120, 12)),
        );
    }

    let n_frac = n.saturating_sub(1).max(1) as f32;

    for (i, &v) in hist.iter().enumerate() {
        let norm = (v as f32 / max_val).clamp(0.0, 1.0);
        if norm < 0.005 {
            continue;
        }

        // Left→right colour gradient when accent_end is supplied
        let bar_accent = match accent_end {
            Some(end) => sp_lerp_color(accent, end, i as f32 / n_frac),
            None => accent,
        };

        let bar_h = norm * bar_h_range;
        let x = rect.min.x + i as f32 * (bar_w + gap);
        let bar_top = split_y - bar_h;
        let color = color_fn(norm, bar_accent);

        // Glow halos — 3 expanding layers
        for g in 0..3usize {
            let spread = (g + 1) as f32 * 1.8;
            let alpha = ((color.a() as f32) * 0.15 / (g + 1) as f32) as u8;
            let gc = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
            painter.rect_filled(
                Rect::from_min_size(
                    egui::pos2(x - spread, bar_top - spread * 0.4),
                    Vec2::new(bar_w + spread * 2.0, bar_h + spread * 0.4),
                ),
                3.0,
                gc,
            );
        }

        // Main bar body
        painter.rect_filled(
            Rect::from_min_size(egui::pos2(x, bar_top), Vec2::new(bar_w, bar_h)),
            2.0,
            color,
        );

        // Bright cap at the top of each bar
        let cap_h = (bar_h * 0.05).clamp(1.5, 3.5);
        painter.rect_filled(
            Rect::from_min_size(egui::pos2(x, bar_top), Vec2::new(bar_w, cap_h)),
            1.0,
            sp_lighten(color, 0.75),
        );

        // Reflection — two-tier fade (28 % + 9 % opacity)
        let rh = (norm * refl_h_range * 0.82).min(refl_h_range);
        if rh > 0.5 {
            let half = rh * 0.5;
            painter.rect_filled(
                Rect::from_min_size(egui::pos2(x, split_y), Vec2::new(bar_w, half)),
                1.0,
                Color32::from_rgba_unmultiplied(
                    color.r(), color.g(), color.b(),
                    (color.a() as f32 * 0.28) as u8,
                ),
            );
            painter.rect_filled(
                Rect::from_min_size(egui::pos2(x, split_y + half), Vec2::new(bar_w, rh - half)),
                1.0,
                Color32::from_rgba_unmultiplied(
                    color.r(), color.g(), color.b(),
                    (color.a() as f32 * 0.09) as u8,
                ),
            );
        }
    }

    // Peak-hold dots — white-ish 2 px rect above each local maximum
    for i in 1..n.saturating_sub(1) {
        if hist[i] > hist[i - 1] && hist[i] >= hist[i + 1] {
            let norm = (hist[i] as f32 / max_val).clamp(0.0, 1.0);
            if norm < 0.06 {
                continue;
            }
            let x = rect.min.x + i as f32 * (bar_w + gap);
            let py = split_y - norm * bar_h_range - 3.5;
            painter.rect_filled(
                Rect::from_min_size(egui::pos2(x, py), Vec2::new(bar_w, 2.0)),
                0.0,
                sp_lighten(color_fn(norm, accent), 0.88),
            );
        }
    }

    // Live value line — horizontal rule at the level of the most recent sample.
    // Animates smoothly because hist is a lerped display buffer.
    if let Some(&last) = hist.last() {
        let norm = (last as f32 / max_val).clamp(0.0, 1.0);
        if norm > 0.02 {
            let y = split_y - norm * bar_h_range;
            let lc = color_fn(norm, accent);
            // Dim rule across full width
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                Stroke::new(0.75, Color32::from_rgba_unmultiplied(lc.r(), lc.g(), lc.b(), 55)),
            );
            // Bright notch on the right edge — "needle" indicator
            painter.rect_filled(
                Rect::from_min_size(
                    egui::pos2(rect.max.x - 4.0, y - 1.5),
                    Vec2::new(4.0, 3.0),
                ),
                1.0,
                Color32::from_rgba_unmultiplied(lc.r(), lc.g(), lc.b(), 200),
            );
        }
    }

    // Floor separator line (accent-tinted)
    painter.line_segment(
        [egui::pos2(rect.min.x, split_y), egui::pos2(rect.max.x, split_y)],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 40)),
    );
}

// ── VU colour functions ───────────────────────────────────────────────────────

/// Standard VU gradient: dim-accent → accent → yellow → red.
/// Use for metrics where HIGH = BAD (CPU %, temperature, etc.).
pub fn vu_color(norm: f32, accent: Color32) -> Color32 {
    let yellow = Color32::from_rgb(0xe8, 0xb8, 0x00);
    let red    = Color32::from_rgb(0xe8, 0x30, 0x30);
    let base = if norm < 0.65 {
        let dim = Color32::from_rgba_unmultiplied(
            accent.r() / 3, accent.g() / 3, accent.b() / 3, 100,
        );
        sp_lerp_color(dim, accent, norm / 0.65)
    } else if norm < 0.82 {
        sp_lerp_color(accent, yellow, (norm - 0.65) / 0.17)
    } else {
        sp_lerp_color(yellow, red, (norm - 0.82) / 0.18)
    };
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), sp_lerp_u8(70, 230, norm))
}

/// Inverted VU gradient for FPS: red → yellow → accent (green).
/// Use for metrics where HIGH = GOOD and LOW = BAD.
pub fn fps_color(norm: f32, accent: Color32) -> Color32 {
    let red    = Color32::from_rgb(0xe8, 0x30, 0x30);
    let yellow = Color32::from_rgb(0xe8, 0xb8, 0x00);
    let base = if norm < 0.25 {
        sp_lerp_color(red, yellow, norm / 0.25)
    } else if norm < 0.50 {
        sp_lerp_color(yellow, accent, (norm - 0.25) / 0.25)
    } else {
        accent
    };
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), sp_lerp_u8(70, 230, norm))
}

// ── Private colour helpers ────────────────────────────────────────────────────

fn sp_lighten(c: Color32, t: f32) -> Color32 {
    sp_lerp_color(c, Color32::WHITE, t)
}

fn sp_lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        sp_lerp_u8(a.r(), b.r(), t),
        sp_lerp_u8(a.g(), b.g(), t),
        sp_lerp_u8(a.b(), b.b(), t),
    )
}

fn sp_lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)) as u8
}

// ── Compact card frame ────────────────────────────────────────────────────────

pub fn compact_card_frame(theme: &Theme) -> Frame {
    Frame::none()
        .fill(theme.card_bg)
        .stroke(Stroke::new(1.0, theme.card_border))
        .rounding(Rounding::same(7.0))
        .inner_margin(Margin { left: 10.0, right: 10.0, top: 5.0, bottom: 5.0 })
        .outer_margin(Margin { left: 0.0, right: 0.0, top: 2.0, bottom: 2.0 })
}

/// Card wrapper that adds a per-accent glow border + hover highlight.
/// Drop-in replacement for `compact_card_frame(theme).show(ui, ...)`.
pub fn glow_card<R>(
    ui: &mut Ui,
    theme: &Theme,
    accent: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let resp = compact_card_frame(theme).show(ui, add_contents);
    let rect = resp.response.rect;
    let hovered = ui.rect_contains_pointer(rect);

    let painter = ui.painter();
    let [r, g, b, _] = accent.to_array();

    // Per-card top border accent stripe (2.5 px, inset by rounding)
    let top_y = rect.min.y + 3.0; // past outer_margin(2) + stroke(1)
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 7.0, top_y),
            egui::pos2(rect.max.x - 7.0, top_y + 2.5),
        ),
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(r, g, b, 140),
    );

    // Full-height left spine — structural column, like a cathedral rib
    let spine_alpha = if hovered { 155 } else { 105 };
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 1.5, top_y),          // flush with top stripe
            egui::pos2(rect.min.x + 3.5, rect.max.y - 9.0), // stops before bottom rounding
        ),
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(r, g, b, spine_alpha),
    );

    // Glow rings — 3 layers expanding outward at decreasing alpha
    let base_alpha: u8 = if hovered { 40 } else { 14 };
    for i in 1u8..=3 {
        let spread = i as f32 * 1.8;
        let alpha = base_alpha / i;
        painter.rect_stroke(
            rect.expand(spread),
            Rounding::same(7.0 + spread),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, alpha)),
        );
    }

    // Brighter border on hover
    if hovered {
        painter.rect_stroke(
            rect,
            Rounding::same(7.0),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 95)),
        );
    }

    resp.inner
}

// ── Gradient progress bar ─────────────────────────────────────────────────────

/// Gradient-filled progress bar: bright on the left, dims 60 % toward the right.
pub fn gradient_bar(ui: &mut Ui, pct: f32, height: f32, color: Color32, track: Color32) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();
    let rv = Rounding::same(height * 0.5);

    painter.rect_filled(rect, rv, track);

    let fill_w = rect.width() * (pct / 100.0).clamp(0.0, 1.0);
    if fill_w < 1.0 {
        return;
    }

    let n = 16usize;
    let [r, g, b, a] = color.to_array();

    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let dim_f = 1.0 - t * 0.60;
        let strip_color = Color32::from_rgba_premultiplied(
            (r as f32 * dim_f) as u8,
            (g as f32 * dim_f) as u8,
            (b as f32 * dim_f) as u8,
            a,
        );
        let x0 = rect.min.x + (i as f32 / n as f32) * fill_w;
        let x1 = (rect.min.x + ((i + 1) as f32 / n as f32) * fill_w + 0.5)
            .min(rect.min.x + fill_w);

        let strip_round = if i == 0 {
            Rounding { nw: rv.nw, sw: rv.sw, ne: 0.0, se: 0.0 }
        } else if i == n - 1 {
            Rounding { nw: 0.0, sw: 0.0, ne: rv.ne, se: rv.se }
        } else {
            Rounding::ZERO
        };

        painter.rect_filled(
            Rect::from_min_max(egui::pos2(x0, rect.min.y), egui::pos2(x1, rect.max.y)),
            strip_round,
            strip_color,
        );
    }

    // Subtle top-edge highlight
    painter.line_segment(
        [
            egui::pos2(rect.min.x + height * 0.5, rect.min.y + 0.5),
            egui::pos2(
                (rect.min.x + fill_w - height * 0.3).max(rect.min.x + height * 0.5),
                rect.min.y + 0.5,
            ),
        ],
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(
                r.saturating_add(30),
                g.saturating_add(30),
                b.saturating_add(30),
                70,
            ),
        ),
    );
}

