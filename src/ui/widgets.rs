//! Reusable low-level widgets: gradient bar, sparkline, stat row.

use egui::{Color32, Frame, Margin, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};
use egui_plot::{Line, Plot, PlotPoints};

use crate::theme::Theme;

// ── Filled progress bar ───────────────────────────────────────────────────────

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

// ── Sparkline ─────────────────────────────────────────────────────────────────

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

// ── Card frame ────────────────────────────────────────────────────────────────

pub fn card_frame(theme: &Theme) -> Frame {
    Frame::none()
        .fill(theme.card_bg)
        .stroke(Stroke::new(1.0, theme.card_border))
        .rounding(theme.card_rounding)
        .inner_margin(Margin::same(12.0))
        .outer_margin(Margin::same(4.0))
}

// ── Card title ────────────────────────────────────────────────────────────────

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

// ── Big value ─────────────────────────────────────────────────────────────────

pub fn big_value(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(
        egui::RichText::new(text)
            .color(color)
            .monospace()
            .size(28.0)
            .strong(),
    );
}
