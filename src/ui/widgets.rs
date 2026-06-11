//! Reusable low-level widgets: spectrum bars, gradient bar, sparkline, dot gauge.

use egui::{
    Color32, Frame, Margin, Mesh, Rect, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};

use crate::theme::Theme;

// ── Spectrum bar panel ────────────────────────────────────────────────────────

/// Renders history as a smooth filled-area curve with a neon glow line on top.
///
/// `max_val`    — value that maps to full height (e.g. 100.0 for percentages).
/// `accent_end` — kept for API compatibility, unused in line mode.
/// `color_fn`   — determines line/fill color from latest value's health (vu or fps).
pub fn spectrum_bars(
    ui: &mut Ui,
    hist: &[f64],
    max_val: f32,
    accent: Color32,
    _accent_end: Option<Color32>,
    height: f32,
    color_fn: fn(f32, Color32) -> Color32,
) {
    let n = hist.len();

    // Always allocate the full zone so card height is static regardless of data state.
    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, height), Sense::hover());

    if n < 2 { return; }

    let painter = ui.painter();

    // top 65% = signal area, bottom 35% = reflection zone
    let split_y    = rect.min.y + rect.height() * 0.65;
    let bar_h_range = split_y - rect.min.y;

    // Faint grid lines at 25 / 50 / 75 %
    for frac in [0.25f32, 0.50, 0.75] {
        let y = split_y - frac * bar_h_range;
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(80, 80, 120, 12)),
        );
    }

    // Map samples evenly across the full width
    let step = avail_w / (n - 1) as f32;
    let raw: Vec<egui::Pos2> = hist.iter().enumerate().map(|(i, &v)| {
        let norm = (v as f32 / max_val).clamp(0.0, 1.0);
        egui::pos2(
            rect.min.x + i as f32 * step,
            (split_y - norm * bar_h_range).clamp(rect.min.y, split_y),
        )
    }).collect();

    // Catmull-Rom smooth curve (6 sub-points per segment)
    let curve: Vec<egui::Pos2> = cr_smooth(&raw, 6)
        .into_iter()
        .map(|p| egui::pos2(p.x, p.y.clamp(rect.min.y, split_y)))
        .collect();

    // Health-aware color — always at full brightness so line is vivid at any load
    let latest_norm = hist.last().map(|&v| (v as f32 / max_val).clamp(0.0, 1.0)).unwrap_or(0.0);
    let hc = color_fn(latest_norm.max(0.18), accent);
    let [r, g, b, _] = hc.to_array();

    // Gradient fill: accent at curve → transparent at split_y
    fill_gradient(&painter, &curve, split_y, r, g, b, 62);

    // Reflection: compressed mirror below split_y, much lower alpha
    let refl: Vec<egui::Pos2> = curve.iter().map(|p| {
        let dist = (split_y - p.y).max(0.0);
        egui::pos2(p.x, (split_y + dist * 0.28).min(rect.max.y))
    }).collect();
    fill_gradient_inv(&painter, &refl, split_y, r, g, b, 18);

    // Neon glow line — wide soft halo → medium → inner → bright core
    for &(w, a) in &[(9.0f32, 9u8), (4.5, 22), (2.2, 58), (1.1, 225)] {
        painter.add(Shape::line(
            curve.clone(),
            Stroke::new(w, Color32::from_rgba_unmultiplied(r, g, b, a)),
        ));
    }

    // Floor separator
    painter.line_segment(
        [egui::pos2(rect.min.x, split_y), egui::pos2(rect.max.x, split_y)],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 40)),
    );
}

// ── Curve helpers ─────────────────────────────────────────────────────────────

/// Catmull-Rom spline: `subdiv` sub-points between every pair of input points.
fn cr_smooth(pts: &[egui::Pos2], subdiv: usize) -> Vec<egui::Pos2> {
    let n = pts.len();
    if n < 2 { return pts.to_vec(); }
    let mut out = Vec::with_capacity((n - 1) * subdiv + 1);
    for i in 0..n - 1 {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 >= n { pts[n - 1] } else { pts[i + 2] };
        for j in 0..subdiv {
            let t = j as f32 / subdiv as f32;
            out.push(cr_pt(p0, p1, p2, p3, t));
        }
    }
    out.push(*pts.last().unwrap());
    out
}

fn cr_pt(p0: egui::Pos2, p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2, t: f32) -> egui::Pos2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let f = |a: f32, b: f32, c: f32, d: f32| {
        0.5 * ((2.0 * b) + (-a + c) * t + (2.0*a - 5.0*b + 4.0*c - d) * t2 + (-a + 3.0*b - 3.0*c + d) * t3)
    };
    egui::pos2(f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y))
}

/// Fills the area between `curve` and `floor_y` with a top→transparent gradient mesh.
fn fill_gradient(painter: &egui::Painter, curve: &[egui::Pos2], floor_y: f32, r: u8, g: u8, b: u8, alpha: u8) {
    if curve.len() < 2 { return; }
    let top_c = Color32::from_rgba_unmultiplied(r, g, b, alpha);
    let bot_c = Color32::from_rgba_unmultiplied(r, g, b, 0);
    let mut mesh = Mesh::default();
    for i in 0..curve.len() - 1 {
        let (p0, p1) = (curve[i], curve[i + 1]);
        let base = mesh.vertices.len() as u32;
        for &(pos, col) in &[
            (egui::pos2(p0.x, p0.y), top_c),
            (egui::pos2(p1.x, p1.y), top_c),
            (egui::pos2(p1.x, floor_y), bot_c),
            (egui::pos2(p0.x, floor_y), bot_c),
        ] {
            mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::pos2(0.0, 0.0), color: col });
        }
        mesh.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }
    painter.add(Shape::Mesh(mesh));
}

/// Fills the area between `ceil_y` and the (reflected) `curve` — transparent at bottom.
fn fill_gradient_inv(painter: &egui::Painter, curve: &[egui::Pos2], ceil_y: f32, r: u8, g: u8, b: u8, alpha: u8) {
    if curve.len() < 2 { return; }
    let top_c = Color32::from_rgba_unmultiplied(r, g, b, alpha);
    let bot_c = Color32::from_rgba_unmultiplied(r, g, b, 0);
    let mut mesh = Mesh::default();
    for i in 0..curve.len() - 1 {
        let (p0, p1) = (curve[i], curve[i + 1]);
        let base = mesh.vertices.len() as u32;
        for &(pos, col) in &[
            (egui::pos2(p0.x, ceil_y), top_c),
            (egui::pos2(p1.x, ceil_y), top_c),
            (egui::pos2(p1.x, p1.y),   bot_c),
            (egui::pos2(p0.x, p0.y),   bot_c),
        ] {
            mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::pos2(0.0, 0.0), color: col });
        }
        mesh.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }
    painter.add(Shape::Mesh(mesh));
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
    let resp = compact_card_frame(theme).show(ui, |inner_ui| {
        // Lock minimum width to the full available width BEFORE the card calls
        // set_max_width to split left content from the right panel.  Without
        // this the frame background only covers the left column and the right
        // panel numbers float outside the card border.
        inner_ui.set_min_width(inner_ui.available_width());
        add_contents(inner_ui)
    });
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

