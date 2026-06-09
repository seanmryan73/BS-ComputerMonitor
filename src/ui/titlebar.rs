//! Custom frameless title bar with drag, minimise, and close controls.

use egui::{Align, Context, Layout, Response, RichText, Sense, Ui, Vec2, ViewportCommand};

use crate::theme::Theme;

pub fn show(ui: &mut Ui, ctx: &Context, theme: &Theme) {
    ui.horizontal(|ui| {
        ui.set_height(36.0);
        ui.add_space(12.0);

        // Accent dot + title
        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter()
            .circle_filled(dot_rect.center(), 5.0, theme.accent_cpu);

        ui.add_space(8.0);
        ui.label(
            RichText::new("BS Computer Monitor")
                .color(theme.text_primary)
                .size(13.0)
                .strong(),
        );

        // Invisible drag region
        let avail_w = ui.available_size_before_wrap().x - 80.0;
        let (_rect, drag_resp) =
            ui.allocate_exact_size(Vec2::new(avail_w.max(0.0), 36.0), Sense::click_and_drag());

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
