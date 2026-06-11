//! Custom frameless title bar with drag, minimise, and close controls.

use std::sync::{Arc, Mutex};
use egui::{Align, Context, Layout, Response, RichText, Sense, Ui, Vec2, ViewportCommand};

use crate::{app::CardVisibility, theme::Theme};

pub fn show(
    ui: &mut Ui,
    ctx: &Context,
    theme: &Theme,
    always_on_top: &mut bool,
    show_about: &mut bool,
    card_vis: Arc<Mutex<CardVisibility>>,
) {
    // Capture total width before any child allocations so we can budget the title.
    let panel_w = ui.available_width();

    // Time-driven breathe animation — period ≈ 3 s, radius 3.5 → 5.5
    let t = ctx.input(|i| i.time) as f32;
    let pulse = (t * 2.094_f32).sin(); // 2π/3 rad/s
    let dot_r = 4.5 + pulse * 1.0;
    let [dr, dg, db, _] = theme.accent_cpu.to_array();
    let glow_a = ((pulse * 0.5 + 0.5) * 45.0 + 8.0) as u8;

    ui.horizontal(|ui| {
        ui.set_height(36.0);
        ui.add_space(12.0);

        // Accent dot — pulsing glow halo + solid core
        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        let p = ui.painter();
        p.circle_filled(
            dot_rect.center(), dot_r + 3.5,
            egui::Color32::from_rgba_unmultiplied(dr, dg, db, glow_a / 3),
        );
        p.circle_filled(
            dot_rect.center(), dot_r + 1.5,
            egui::Color32::from_rgba_unmultiplied(dr, dg, db, glow_a),
        );
        p.circle_filled(dot_rect.center(), dot_r, theme.accent_cpu);
        ui.add_space(8.0);

        // Title — shrinks at narrow widths so the drag zone stays usable.
        // Budget = panel - (left_pad + dot + gap) - btn_area(5 btns) - min_drag
        //        = panel - 30 - 172 - 30 = panel - 232
        let title_budget = panel_w - 232.0;
        let title = if title_budget >= 148.0 {
            "BS Computer Monitor"
        } else if title_budget >= 72.0 {
            "BS Monitor"
        } else {
            ""
        };
        if !title.is_empty() {
            ui.label(RichText::new(title).color(theme.text_primary).size(13.0).strong());
        }

        // Invisible drag region — guaranteed ≥ 30 px wide regardless of window size
        let avail_w = ui.available_size_before_wrap().x - 172.0;
        let (_rect, drag_resp) =
            ui.allocate_exact_size(Vec2::new(avail_w.max(30.0), 36.0), Sense::click_and_drag());

        if drag_resp.drag_started() || drag_resp.dragged() {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }

        let passthrough_on = card_vis.lock().map(|v| v.passthrough_mode).unwrap_or(false);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(8.0);
            if close_btn(ui, theme).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if min_btn(ui, theme).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }
            if pin_btn(ui, theme, *always_on_top).on_hover_text("Pin on top").clicked() {
                *always_on_top = !*always_on_top;
                let level = if *always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
            }
            if about_btn(ui, theme).on_hover_text("Help / About / Config").clicked() {
                *show_about = !*show_about;
            }
            let pt_tip = if passthrough_on {
                "Passthrough ON — hold CTRL to interact\nClick to disable"
            } else {
                "Game overlay / passthrough mode\nClicks pass through to app behind"
            };
            if passthrough_btn(ui, theme, passthrough_on).on_hover_text(pt_tip).clicked() {
                if let Ok(mut vis) = card_vis.lock() {
                    vis.passthrough_mode = !vis.passthrough_mode;
                    vis.save();
                }
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

fn pin_btn(ui: &mut Ui, theme: &Theme, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let color = if active {
        theme.accent_cpu
    } else if resp.hovered() {
        theme.text_primary
    } else {
        theme.text_dim
    };
    let bg = theme.titlebar_bg;
    let c = rect.center();
    let p = ui.painter();

    // Jellyfish bell — same dome as the old skull
    let dome = c + egui::vec2(0.0, -2.0);
    p.circle_filled(dome, 6.5, color);

    // Bioluminescent spots — same positions as the skull eye sockets
    p.circle_filled(dome + egui::vec2(-3.0, -1.5), 1.8, bg);
    p.circle_filled(dome + egui::vec2( 3.0, -1.5), 1.8, bg);

    // Bell skirt edge — same line as the skull jaw, now defines bell bottom
    let jaw_y = dome.y + 4.0;
    p.line_segment(
        [egui::pos2(c.x - 5.5, jaw_y), egui::pos2(c.x + 5.5, jaw_y)],
        egui::Stroke::new(2.0, bg),
    );

    // Oral arms — the skull's 3 teeth, now trailing below the bell.
    // The skull had bg-coloured gaps at ±1.9 px creating 3 solid teeth;
    // those same tooth centres become the arm roots here, flaring outward.
    let arm_top = jaw_y + 0.5;
    let arm_len = 6.5_f32;
    for (x_top, x_bot) in [
        (c.x - 3.2, c.x - 4.0),
        (c.x,       c.x      ),
        (c.x + 3.2, c.x + 4.0),
    ] {
        p.line_segment(
            [egui::pos2(x_top, arm_top), egui::pos2(x_bot, arm_top + arm_len)],
            egui::Stroke::new(1.6, color),
        );
    }

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
