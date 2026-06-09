//! UI rendering — called once per frame from [`crate::app::MonitorApp::update`].

mod cards;
mod titlebar;
mod widgets;

use egui::{CentralPanel, Context, Frame, TopBottomPanel};

use crate::{app::MonitorApp, models::SystemSnapshot};

pub fn draw(
    app: &mut MonitorApp,
    ctx: &Context,
    _frame: &mut eframe::Frame,
    snap: &SystemSnapshot,
) {
    let tb_bg = app.theme.titlebar_bg;
    let bg = app.theme.bg;

    TopBottomPanel::top("titlebar")
        .exact_height(36.0)
        .frame(Frame::none().fill(tb_bg))
        .show(ctx, |ui| {
            titlebar::show(ui, ctx, &app.theme);
        });

    CentralPanel::default()
        .frame(Frame::none().fill(bg).inner_margin(egui::Margin::same(12.0)))
        .show(ctx, |ui| {
            cards::show_grid(app, ui, snap);
        });
}
