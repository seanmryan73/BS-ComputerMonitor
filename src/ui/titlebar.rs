//! Custom frameless title bar with drag, minimise, and close controls.

use egui::{Align, Context, Layout, Response, RichText, Sense, Ui, Vec2, ViewportCommand};

use crate::theme::Theme;

pub fn show(ui: &mut Ui, ctx: &Context, theme: &Theme, always_on_top: &mut bool, show_about: &mut bool) {
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
        // Budget = panel - (left_pad + dot + gap) - btn_area(4 btns) - min_drag
        //        = panel - 30 - 144 - 30 = panel - 204
        let title_budget = panel_w - 204.0;
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
        let avail_w = ui.available_size_before_wrap().x - 144.0;
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
            if pin_btn(ui, theme, *always_on_top).clicked() {
                *always_on_top = !*always_on_top;
                let level = if *always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
            }
            if about_btn(ui, theme).clicked() {
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

    // Skull dome — solid filled circle
    let dome = c + egui::vec2(0.0, -2.0);
    p.circle_filled(dome, 6.5, color);

    // Eye sockets — bg-colored holes punched through the solid dome
    let eye_l = dome + egui::vec2(-3.0, -1.5);
    let eye_r = dome + egui::vec2( 3.0, -1.5);
    p.circle_filled(eye_l, 2.0, bg);
    p.circle_filled(eye_r, 2.0, bg);

    // Jaw separation — bg line cuts dome into face + teeth area
    let jaw_y = dome.y + 4.0;
    p.line_segment(
        [egui::pos2(c.x - 5.0, jaw_y), egui::pos2(c.x + 5.0, jaw_y)],
        egui::Stroke::new(1.8, bg),
    );

    // Two gaps creating 3 teeth (bg vertical cuts in lower dome)
    for dx in [-1.9f32, 1.9] {
        p.line_segment(
            [egui::pos2(c.x + dx, jaw_y), egui::pos2(c.x + dx, dome.y + 7.2)],
            egui::Stroke::new(1.8, bg),
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
    let color = if resp.hovered() {
        theme.accent_gpu
    } else {
        theme.text_dim
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "\u{00BF}",
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
        color,
    );
    resp
}
