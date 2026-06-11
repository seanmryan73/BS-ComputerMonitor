//! UI rendering — called once per frame from [`crate::app::MonitorApp::update`].

mod about;
mod cards;
mod titlebar;
mod widgets;

use egui::{
    Area, CentralPanel, Color32, Context, Frame, Id, Order, ResizeDirection, Sense, Stroke,
    TopBottomPanel, ViewportCommand,
};

use std::sync::Arc;
use crate::{app::MonitorApp, models::{FpsSnapshot, SystemSnapshot}};

pub fn draw(
    app: &mut MonitorApp,
    ctx: &Context,
    _frame: &mut eframe::Frame,
    snap: &SystemSnapshot,
    fps: &FpsSnapshot,
) {
    let tb_bg = app.theme.titlebar_bg;
    let bg = app.theme.bg;

    let show_about = &mut app.show_about;
    TopBottomPanel::top("titlebar")
        .exact_height(36.0)
        .frame(Frame::none().fill(tb_bg))
        .show(ctx, |ui| {
            titlebar::show(ui, ctx, &app.theme, show_about, Arc::clone(&app.card_vis));
        });

    // Calculate where to place the settings window on the frame it first opens.
    // Only set once — user can freely reposition it after that.
    let initial_pos = if app.show_about && !app.prev_show_about {
        ctx.input(|i| i.viewport().outer_rect)
            .map(|r| egui::pos2((r.min.x - 360.0 - 8.0).max(0.0), r.min.y))
    } else {
        None
    };
    app.prev_show_about = app.show_about;

    let vis = app.card_vis.lock().map(|g| g.clone()).unwrap_or_default();
    CentralPanel::default()
        .frame(Frame::none().fill(bg).inner_margin(egui::Margin::same(12.0)))
        .show(ctx, |ui| {
            cards::show_grid(app, ui, snap, fps, &vis);
        });

    // Settings window — separate opaque OS window to the left
    about::show(ctx, &app.theme, &mut app.show_about, Arc::clone(&app.card_vis), initial_pos, app.is_elevated);

    // Resize handles — invisible edge/corner hit-zones around the window.
    // The title bar (top 36 px) is excluded; N/NE/NW are skipped to avoid
    // conflicting with the title-bar drag region.
    resize_handles(ctx, &app.theme);

    // CRT scanline overlay — subtle horizontal dark rules every 2.5 px.
    scanline_overlay(ctx);
}

fn scanline_overlay(ctx: &Context) {
    Area::new(Id::new("scanlines"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            let rect = ctx.screen_rect();
            let painter = ui.painter();
            let line = Color32::from_rgba_unmultiplied(0, 0, 0, 18);
            let mut y = rect.min.y;
            while y < rect.max.y {
                painter.line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    Stroke::new(0.5, line),
                );
                y += 2.5;
            }
        });
}

fn resize_handles(ctx: &Context, theme: &crate::theme::Theme) {
    let sr = ctx.screen_rect();
    let edge = 5.0f32;
    let corner = 14.0f32;

    // (id_suffix, ResizeDirection, cursor, rect)
    let handles: &[(&str, ResizeDirection, egui::CursorIcon, egui::Rect)] = &[
        // Bottom edge (excluding corners)
        (
            "rs",
            ResizeDirection::South,
            egui::CursorIcon::ResizeSouth,
            egui::Rect::from_min_size(
                egui::pos2(sr.min.x + corner, sr.max.y - edge),
                egui::vec2(sr.width() - corner * 2.0, edge),
            ),
        ),
        // Right edge
        (
            "re",
            ResizeDirection::East,
            egui::CursorIcon::ResizeEast,
            egui::Rect::from_min_size(
                egui::pos2(sr.max.x - edge, sr.min.y + 36.0),
                egui::vec2(edge, (sr.height() - 36.0 - corner).max(0.0)),
            ),
        ),
        // Left edge
        (
            "rw",
            ResizeDirection::West,
            egui::CursorIcon::ResizeWest,
            egui::Rect::from_min_size(
                egui::pos2(sr.min.x, sr.min.y + 36.0),
                egui::vec2(edge, (sr.height() - 36.0 - corner).max(0.0)),
            ),
        ),
        // SE corner
        (
            "rse",
            ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
            egui::Rect::from_min_size(
                egui::pos2(sr.max.x - corner, sr.max.y - corner),
                egui::vec2(corner, corner),
            ),
        ),
        // SW corner
        (
            "rsw",
            ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeSouthWest,
            egui::Rect::from_min_size(
                egui::pos2(sr.min.x, sr.max.y - corner),
                egui::vec2(corner, corner),
            ),
        ),
    ];

    for (id_str, dir, cursor, hit_rect) in handles {
        let dir = *dir;
        let cursor = *cursor;
        let rect = *hit_rect;
        if rect.width() <= 0.0 || rect.height() <= 0.0 { continue; }
        let grip = dir == ResizeDirection::SouthEast;

        Area::new(Id::new(*id_str))
            .fixed_pos(rect.min)
            .order(Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                let (_, resp) = ui.allocate_exact_size(rect.size(), Sense::drag());

                if resp.hovered() || resp.dragged() {
                    ctx.set_cursor_icon(cursor);
                }
                if resp.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }

                // Visible SE grip indicator
                if grip {
                    let p = ui.painter();
                    let c = Color32::from_rgba_unmultiplied(
                        theme.text_subtle.r(),
                        theme.text_subtle.g(),
                        theme.text_subtle.b(),
                        if resp.hovered() { 160 } else { 70 },
                    );
                    for i in 1..=3 {
                        let d = i as f32 * 4.0;
                        p.line_segment(
                            [
                                egui::pos2(rect.max.x - d, rect.max.y - 1.0),
                                egui::pos2(rect.max.x - 1.0, rect.max.y - d),
                            ],
                            Stroke::new(1.2, c),
                        );
                    }
                }
            });
    }
}
