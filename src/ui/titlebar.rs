//! Custom frameless title bar with drag, minimise, and close controls.

use egui::{Align, Context, Layout, Response, RichText, Sense, Ui, Vec2, ViewportCommand};

use crate::theme::Theme;

pub fn show(ui: &mut Ui, ctx: &Context, theme: &Theme, always_on_top: &mut bool) {
    // Capture total width before any child allocations so we can budget the title.
    let panel_w = ui.available_width();

    ui.horizontal(|ui| {
        ui.set_height(36.0);
        ui.add_space(12.0);

        // Accent dot
        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter()
            .circle_filled(dot_rect.center(), 5.0, theme.accent_cpu);
        ui.add_space(8.0);

        // Title — shrinks at narrow widths so the drag zone stays usable.
        // Budget = panel - (left_pad + dot + gap) - btn_area - min_drag
        //        = panel - 30 - 108 - 30 = panel - 168
        let title_budget = panel_w - 168.0;
        let title = if title_budget >= 155.0 {
            "BS Computer Monitor"
        } else if title_budget >= 75.0 {
            "BS Monitor"
        } else {
            ""
        };
        if !title.is_empty() {
            ui.label(RichText::new(title).color(theme.text_primary).size(13.0).strong());
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
            if pin_btn(ui, theme, *always_on_top).clicked() {
                *always_on_top = !*always_on_top;
                let level = if *always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
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
    let center = rect.center();
    let p = ui.painter();
    // Pin icon: vertical line + horizontal crossbar
    p.line_segment(
        [center + egui::vec2(0.0, -6.0), center + egui::vec2(0.0, 4.0)],
        egui::Stroke::new(1.5, color),
    );
    p.line_segment(
        [center + egui::vec2(-4.0, -3.0), center + egui::vec2(4.0, -3.0)],
        egui::Stroke::new(1.5, color),
    );
    p.circle_filled(center + egui::vec2(0.0, -6.0), 2.0, color);
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
