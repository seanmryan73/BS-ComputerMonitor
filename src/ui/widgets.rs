//! Reusable low-level widgets: sparkline, card frame, glow card.

use egui::{
    Color32, Frame, Margin, Mesh, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};

use crate::theme::Theme;

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

// ── Mini sparkline ────────────────────────────────────────────────────────────

/// Trend sparkline for compact cards — gradient fill + soft glow + crisp line.
///
/// Drawn behind the value text; alpha is tuned so the big number stays readable.
pub fn mini_sparkline(
    painter: &egui::Painter,
    rect: egui::Rect,
    data: &[f64],
    max_val: f32,
    color: Color32,
) {
    if data.len() < 2 || max_val <= 0.0 { return; }
    let h = rect.height();
    if h < 4.0 { return; }
    let [r, g, b, _] = color.to_array();

    let step = rect.width() / (data.len() - 1).max(1) as f32;
    let raw: Vec<egui::Pos2> = data.iter().enumerate().map(|(i, &v)| {
        let norm = (v as f32 / max_val).clamp(0.0, 1.0);
        egui::pos2(
            rect.min.x + i as f32 * step,
            rect.max.y - norm * h,
        )
    }).collect();

    let curve = cr_smooth(&raw, 4);
    fill_gradient(painter, &curve, rect.max.y, r, g, b, 32);
    // Soft glow halo then crisp core
    painter.add(egui::Shape::line(curve.clone(), egui::Stroke::new(2.5, Color32::from_rgba_unmultiplied(r, g, b, 22))));
    painter.add(egui::Shape::line(curve,          egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 70))));
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
pub fn glow_card<R>(
    ui: &mut Ui,
    theme: &Theme,
    accent: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let resp = compact_card_frame(theme).show(ui, |inner_ui| {
        inner_ui.set_min_width(inner_ui.available_width());
        add_contents(inner_ui)
    });
    let rect = resp.response.rect;
    let hovered = ui.rect_contains_pointer(rect);

    let painter = ui.painter();
    let [r, g, b, _] = accent.to_array();

    // Per-card top border accent stripe
    let top_y = rect.min.y + 3.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 7.0, top_y),
            egui::pos2(rect.max.x - 7.0, top_y + 2.5),
        ),
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(r, g, b, 140),
    );

    // Full-height left spine
    let spine_alpha = if hovered { 155 } else { 105 };
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 1.5, top_y),
            egui::pos2(rect.min.x + 3.5, rect.max.y - 9.0),
        ),
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(r, g, b, spine_alpha),
    );

    // Glow rings
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

    if hovered {
        painter.rect_stroke(
            rect,
            Rounding::same(7.0),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 95)),
        );
    }

    resp.inner
}
