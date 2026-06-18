//! Custom frameless title bar with drag, minimise, and close controls.

use std::sync::{Arc, Mutex};
use egui::{Align, Context, Layout, Response, RichText, Sense, Ui, Vec2, ViewportCommand};

use crate::{app::CardVisibility, theme::Theme};

pub fn show(
    ui: &mut Ui,
    ctx: &Context,
    theme: &Theme,
    show_about: &mut bool,
    card_vis: Arc<Mutex<CardVisibility>>,
) {
    let panel_w = ui.available_width();
    let passthrough_on = card_vis.lock().map(|v| v.passthrough_mode).unwrap_or(false);

    ui.horizontal(|ui| {
        ui.set_height(36.0);
        ui.add_space(8.0);

        // Passthrough crosshair — left side, replaces the pulsing dot
        let pt_tip = if passthrough_on {
            "Passthrough ON — hold CTRL to interact\nClick to disable"
        } else {
            "Game overlay / passthrough mode\nClicks pass through to app behind"
        };
        if passthrough_btn(ui, theme, passthrough_on).on_hover_text(pt_tip).clicked() {
            if let Ok(mut vis) = card_vis.lock() {
                vis.passthrough_mode = !vis.passthrough_mode;
            }
        }
        ui.add_space(8.0);

        // Title — shrinks at narrow widths so the drag zone stays usable.
        // Budget = panel - (left: 8 + 28 + 8) - (right: 3 btns ≈ 108) - min_drag 30
        //        = panel - 174
        let title_budget = panel_w - 174.0;
        let title = if title_budget >= 148.0 {
            "BS Computer Monitor"
        } else if title_budget >= 72.0 {
            "BS Monitor"
        } else {
            ""
        };
        if !title.is_empty() {
            ui.label(RichText::new(title).color(theme.accent_cpu).size(14.0).strong());
        }

        // Invisible drag region — guaranteed ≥ 30 px wide regardless of window size
        let avail_w = ui.available_size_before_wrap().x - 108.0;
        let (_rect, drag_resp) =
            ui.allocate_exact_size(Vec2::new(avail_w.max(30.0), 36.0), Sense::click_and_drag());

        if drag_resp.drag_started() || drag_resp.dragged() {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(8.0);
            if close_btn(ui, theme).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if min_btn(ui, theme).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }
            if about_btn(ui, theme).on_hover_text("Help / About / Config").clicked() {
                *show_about = !*show_about;
            }
        });
    });
}

fn close_btn(ui: &mut Ui, theme: &Theme) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let color = if resp.hovered() {
        egui::Color32::from_rgb(220, 60, 60)
    } else {
        theme.text_dim
    };
    let center = rect.center();
    let p = ui.painter();
    p.line_segment(
        [center + egui::vec2(-5.0, -5.0), center + egui::vec2(5.0, 5.0)],
        egui::Stroke::new(1.5, color),
    );
    p.line_segment(
        [center + egui::vec2(5.0, -5.0), center + egui::vec2(-5.0, 5.0)],
        egui::Stroke::new(1.5, color),
    );
    resp
}


fn min_btn(ui: &mut Ui, theme: &Theme) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let color = if resp.hovered() {
        theme.text_primary
    } else {
        theme.text_dim
    };
    let center = rect.center();
    ui.painter().line_segment(
        [center + egui::vec2(-5.0, 2.0), center + egui::vec2(5.0, 2.0)],
        egui::Stroke::new(1.5, color),
    );
    resp
}

fn about_btn(ui: &mut Ui, theme: &Theme) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let color = if resp.hovered() { theme.accent_gpu } else { theme.text_dim };
    let c = rect.center();
    let p = ui.painter();

    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        "?",
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
        color,
    );

    resp
}

fn passthrough_btn(ui: &mut Ui, theme: &Theme, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let color = if active {
        theme.accent_cpu
    } else if resp.hovered() {
        theme.text_primary
    } else {
        theme.text_dim
    };
    let c = rect.center();
    let p = ui.painter();
    let r = 5.5_f32;

    // Crosshair / target reticle icon
    p.circle_stroke(c, r, egui::Stroke::new(1.2, color));
    // 4 axis lines, gap of 2 px from circle edge
    let gap = 2.0;
    p.line_segment([c + egui::vec2(0.0, -(r + gap)), c + egui::vec2(0.0, -(r + gap + 3.5))], egui::Stroke::new(1.2, color));
    p.line_segment([c + egui::vec2(0.0,  r + gap),   c + egui::vec2(0.0,  r + gap + 3.5)],   egui::Stroke::new(1.2, color));
    p.line_segment([c + egui::vec2(-(r + gap), 0.0), c + egui::vec2(-(r + gap + 3.5), 0.0)], egui::Stroke::new(1.2, color));
    p.line_segment([c + egui::vec2( r + gap,   0.0), c + egui::vec2( r + gap + 3.5,   0.0)], egui::Stroke::new(1.2, color));
    // Center dot
    p.circle_filled(c, 1.2, color);

    resp
}
