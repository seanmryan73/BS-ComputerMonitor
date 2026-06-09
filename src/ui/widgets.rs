//! Reusable low-level widgets: gradient bar, sparkline, stat row, layered dots.

use egui::{
    Color32, Frame, Margin, Painter, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints};

use crate::theme::Theme;

// ── Compact card frame ────────────────────────────────────────────────────────

pub fn compact_card_frame(theme: &Theme) -> Frame {
    Frame::none()
        .fill(theme.card_bg)
        .stroke(Stroke::new(1.0, theme.card_border))
        .rounding(Rounding::same(7.0))
        .inner_margin(Margin { left: 10.0, right: 10.0, top: 5.0, bottom: 5.0 })
        .outer_margin(Margin { left: 0.0, right: 0.0, top: 2.0, bottom: 2.0 })
}

// ── Layered-glow dot gauge ────────────────────────────────────────────────────

/// Draws a row of glowing LED-style dots. Filled dots have a 3-layer glow
/// (outer halo → inner bloom → solid core + specular highlight).
/// `dot_r` controls dot radius: 4.0 = normal, 3.0 = small for two-row layouts.
pub fn layered_dots(ui: &mut Ui, pct: f32, color: Color32, dot_r: f32) {
    let gap = 3.5f32;
    let row_h = dot_r * 2.0 + 6.0; // 3 px glow bleed on each side

    let avail = ui.available_width();
    let n = ((avail + gap) / (dot_r * 2.0 + gap)).floor() as usize;
    if n == 0 {
        return;
    }
    let total_w = n as f32 * (dot_r * 2.0 + gap) - gap;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, row_h), Sense::hover());
    let painter = ui.painter();

    let filled = ((pct / 100.0).clamp(0.0, 1.0) * n as f32).round() as usize;
    let [r, g, b, _] = color.to_array();
    let dim = Color32::from_rgba_unmultiplied(r / 5, g / 5, b / 5, 210);

    for i in 0..n {
        let cx = rect.min.x + i as f32 * (dot_r * 2.0 + gap) + dot_r;
        let cy = rect.center().y;
        let center = egui::pos2(cx, cy);

        if i < filled {
            // Layer 1 — outer halo
            painter.circle_filled(
                center,
                dot_r + 2.5,
                Color32::from_rgba_unmultiplied(r, g, b, 18),
            );
            // Layer 2 — inner bloom
            painter.circle_filled(
                center,
                dot_r + 1.2,
                Color32::from_rgba_unmultiplied(r, g, b, 55),
            );
            // Layer 3 — solid core
            painter.circle_filled(center, dot_r, color);
            // Specular highlight (upper-left of dot)
            painter.circle_filled(
                center + egui::vec2(-dot_r * 0.28, -dot_r * 0.28),
                dot_r * 0.32,
                Color32::from_rgba_unmultiplied(
                    r.saturating_add(60),
                    g.saturating_add(60),
                    b.saturating_add(60),
                    160,
                ),
            );
        } else {
            painter.circle_filled(center, dot_r, dim);
        }
    }
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

// ── Painter-based sparkline ───────────────────────────────────────────────────

/// Lightweight sparkline drawn directly with the Painter (no egui_plot overhead).
/// Draws an area fill below the curve and a line on top.
pub fn mini_sparkline_raw(painter: &Painter, rect: Rect, data: &[f64], color: Color32) {
    if data.len() < 2 {
        return;
    }
    let max_v = data.iter().cloned().fold(1.0f64, f64::max);
    let n = data.len();
    let [r, g, b, _] = color.to_array();

    let pts: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let t = i as f32 / (n - 1).max(1) as f32;
            egui::pos2(
                rect.min.x + t * rect.width(),
                (rect.max.y - (v as f32 / max_v as f32) * rect.height())
                    .clamp(rect.min.y, rect.max.y),
            )
        })
        .collect();

    // Area fill — convex quad per segment (baseline to curve)
    for pair in pts.windows(2) {
        let a = pair[0];
        let b_pt = pair[1];
        painter.add(egui::Shape::convex_polygon(
            vec![
                a,
                b_pt,
                egui::pos2(b_pt.x, rect.max.y),
                egui::pos2(a.x, rect.max.y),
            ],
            Color32::from_rgba_unmultiplied(r, g, b, 22),
            Stroke::NONE,
        ));
    }

    // Line on top
    for pair in pts.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(1.5, color));
    }
}

// ── Filled progress bar (legacy) ──────────────────────────────────────────────

pub fn bar(
    ui: &mut Ui,
    pct: f32,
    height: f32,
    fill: Color32,
    track: Color32,
    rounding: Rounding,
) -> Response {
    let desired = Vec2::new(ui.available_width(), height);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, rounding, track);

    let fill_w = (rect.width() * (pct / 100.0).clamp(0.0, 1.0)).max(0.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
        painter.rect_filled(fill_rect, rounding, fill);
    }

    resp
}

pub fn mini_bar(ui: &mut Ui, pct: f32, fill: Color32, track: Color32) {
    bar(ui, pct, 5.0, fill, track, Rounding::same(2.5));
}

// ── Sparkline (egui_plot, kept for compatibility) ─────────────────────────────

pub fn sparkline(ui: &mut Ui, id: &str, data: &[f64], color: Color32, height: f32) {
    let points: PlotPoints = data
        .iter()
        .enumerate()
        .map(|(i, &v)| [i as f64, v])
        .collect();

    Plot::new(id)
        .height(height)
        .width(ui.available_width())
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_drag(false)
        .allow_boxed_zoom(false)
        .show_axes([false, false])
        .show_grid(false)
        .include_y(0.0)
        .include_y(100.0)
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(points).color(color).width(1.5));
        });
}

// ── Stat row ──────────────────────────────────────────────────────────────────

pub fn stat_row(ui: &mut Ui, label: &str, value: &str, label_color: Color32, value_color: Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(label_color).size(11.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .color(value_color)
                    .monospace()
                    .size(11.0),
            );
        });
    });
}

// ── Card frame (legacy) ───────────────────────────────────────────────────────

pub fn card_frame(theme: &Theme) -> Frame {
    Frame::none()
        .fill(theme.card_bg)
        .stroke(Stroke::new(1.0, theme.card_border))
        .rounding(theme.card_rounding)
        .inner_margin(Margin::same(12.0))
        .outer_margin(Margin::same(4.0))
}

// ── Card title (legacy) ───────────────────────────────────────────────────────

pub fn card_title(ui: &mut Ui, label: &str, accent: Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(1.5), accent);
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(label)
                .color(accent)
                .size(11.0)
                .strong(),
        );
    });
    ui.add_space(6.0);
}

// ── Big value (legacy) ────────────────────────────────────────────────────────

pub fn big_value(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(
        egui::RichText::new(text)
            .color(color)
            .monospace()
            .size(28.0)
            .strong(),
    );
}
