//! Settings — separate opaque OS window that pops out to the left of the main window.

use std::sync::{Arc, Mutex};

use egui::{Color32, Frame, Margin, RichText, Stroke, Vec2, ViewportBuilder, ViewportId};

use crate::{app::CardVisibility, theme::Theme};

const CLOSE_ID: &str = "about_close_requested";

pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    open: &mut bool,
    card_vis: Arc<Mutex<CardVisibility>>,
    initial_pos: Option<egui::Pos2>,
    is_elevated: bool,
) {
    let close_id = egui::Id::new(CLOSE_ID);
    if ctx.data(|d| d.get_temp::<bool>(close_id).unwrap_or(false)) {
        *open = false;
        ctx.data_mut(|d| d.remove::<bool>(close_id));
    }

    if !*open {
        return;
    }

    let theme = *theme;

    let mut builder = ViewportBuilder::default()
        .with_title("BS Monitor — Settings")
        .with_inner_size([360.0, 720.0])
        .with_resizable(false)
        .with_maximize_button(false);

    if let Some(pos) = initial_pos {
        builder = builder.with_position(pos);
    }

    ctx.show_viewport_deferred(
        ViewportId::from_hash_of("about_viewport"),
        builder,
        move |ctx, _class| {
            let is_elevated = is_elevated;
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new(CLOSE_ID), true));
            }

            let mut vis = egui::Visuals::dark();
            vis.override_text_color = Some(theme.text_primary);
            vis.panel_fill = Color32::from_rgb(0x07, 0x06, 0x12);
            vis.selection.bg_fill = theme.accent_cpu;
            ctx.set_visuals(vis);

            egui::CentralPanel::default()
                .frame(
                    Frame::none()
                        .fill(Color32::from_rgb(0x07, 0x06, 0x12))
                        .inner_margin(Margin::same(20.0)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // ── Header ────────────────────────────────────────
                        ui.label(
                            RichText::new("BS COMPUTER MONITOR")
                                .color(theme.accent_cpu)
                                .monospace()
                                .size(17.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("v1.96.9  ·  System Resource Monitor")
                                .color(theme.text_subtle)
                                .monospace()
                                .size(10.0),
                        );
                        ui.add_space(8.0);
                        row(ui, theme, "AUTHOR",   "seanmryan@gmail.com");
                        row(ui, theme, "COMPANY",  "BagPipes — BS Solutions");
                        ui.add_space(4.0);
                        row(ui, theme, "RUNTIME",  "Rust  ·  egui 0.29  ·  eframe 0.29");
                        row(ui, theme, "PLATFORM", "Windows  ·  x86_64");

                        // ── Permissions ───────────────────────────────────
                        section(ui, theme, "PERMISSIONS");

                        if is_elevated {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").color(theme.ok).monospace().size(11.0));
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Running as Administrator — all sensors active")
                                        .color(theme.text_subtle)
                                        .monospace()
                                        .size(10.0),
                                );
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("▲").color(theme.warn).monospace().size(11.0));
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Not running as Administrator")
                                        .color(theme.warn)
                                        .monospace()
                                        .size(10.5)
                                        .strong(),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Some sensors require admin rights:")
                                    .color(theme.text_dim)
                                    .monospace()
                                    .size(10.0),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("  GPU utilization %\n  CPU / GPU temperatures")
                                    .color(theme.text_subtle)
                                    .monospace()
                                    .size(10.0),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("TO RUN AS ADMINISTRATOR")
                                    .color(theme.text_subtle)
                                    .monospace()
                                    .size(10.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("Right-click the .exe → Run as administrator\n\nOr create a shortcut → Properties → Advanced\n→ check \"Run as administrator\"")
                                    .color(theme.text_dim)
                                    .monospace()
                                    .size(10.0),
                            );
                        }

                        // ── Display mode ──────────────────────────────────
                        section(ui, theme, "DISPLAY MODE");

                        if let Ok(mut vis) = card_vis.lock() {
                            let c1 = toggle(ui, theme, &mut vis.compact_mode,
                                "COMPACT  —  numbers only, narrower window");
                            let is_compact = vis.compact_mode;
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let col = if is_compact { theme.text_subtle } else { theme.text_dim };
                                ui.label(RichText::new("VALUE SIZE").color(col).monospace().size(10.0));
                                let slider = egui::Slider::new(
                                    &mut vis.compact_font_size,
                                    11.0_f32..=60.0_f32,
                                )
                                .custom_formatter(|v, _| format!("{:.0} pt", v))
                                .custom_parser(|s| {
                                    s.trim_end_matches("pt").trim().parse().ok()
                                });
                                let c2 = ui.add_enabled(is_compact, slider).changed();
                                if c1 || c2 { vis.save(); }
                            });
                        }

                        // ── Visible cards ─────────────────────────────────
                        section(ui, theme, "VISIBLE CARDS");

                        let mut always = true;
                        ui.add_enabled(false, egui::Checkbox::new(
                            &mut always,
                            RichText::new("CPU  —  always visible").color(theme.text_dim).monospace().size(10.5),
                        ));
                        ui.add_enabled(false, egui::Checkbox::new(
                            &mut always,
                            RichText::new("MEM  —  always visible").color(theme.text_dim).monospace().size(10.5),
                        ));
                        ui.add_space(2.0);

                        if let Ok(mut vis) = card_vis.lock() {
                            let c1 = toggle(ui, theme, &mut vis.show_fps,  "FPS");
                            let c2 = toggle(ui, theme, &mut vis.show_gpu,  "GPU");
                            let c3 = toggle(ui, theme, &mut vis.show_net,  "NET");
                            let c4 = toggle(ui, theme, &mut vis.show_disk, "DISK");
                            let c5 = toggle(ui, theme, &mut vis.show_temp, "TEMP");
                            if c1 || c2 || c3 || c4 || c5 { vis.save(); }
                        }

                        // ── Window ────────────────────────────────────────
                        section(ui, theme, "WINDOW");

                        ui.add_space(6.0);
                        ui.label(RichText::new("OPACITY").color(theme.text_subtle).monospace().size(10.0));
                        ui.add_space(2.0);

                        if let Ok(mut vis) = card_vis.lock() {
                            let resp = ui.add(
                                egui::Slider::new(&mut vis.opacity, 0.15_f32..=1.0_f32)
                                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                    .custom_parser(|s| {
                                        s.trim_end_matches('%').parse().ok().map(|v: f64| v / 100.0)
                                    }),
                            );
                            if resp.changed() { vis.save(); }
                        }

                        // ── Game overlay / Passthrough ────────────────────
                        section(ui, theme, "GAME OVERLAY / PASSTHROUGH");

                        // Crosshair icon callout — shows exactly what the button looks like
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::Vec2::splat(28.0), egui::Sense::hover(),
                            );
                            let c = rect.center();
                            let r = 5.5_f32;
                            let gap = 2.0;
                            let stroke = egui::Stroke::new(1.2, theme.accent_cpu);
                            let p = ui.painter();
                            p.circle_stroke(c, r, stroke);
                            p.line_segment([c + egui::vec2(0.0, -(r+gap)), c + egui::vec2(0.0, -(r+gap+3.5))], stroke);
                            p.line_segment([c + egui::vec2(0.0,  r+gap),   c + egui::vec2(0.0,  r+gap+3.5)],   stroke);
                            p.line_segment([c + egui::vec2(-(r+gap), 0.0), c + egui::vec2(-(r+gap+3.5), 0.0)], stroke);
                            p.line_segment([c + egui::vec2( r+gap,   0.0), c + egui::vec2( r+gap+3.5,   0.0)], stroke);
                            p.circle_filled(c, 1.2, theme.accent_cpu);

                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Click this button (top-left of title bar)\nto toggle passthrough on or off.")
                                    .color(theme.text_subtle)
                                    .monospace()
                                    .size(9.5),
                            );
                        });

                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Mouse clicks pass through to whatever is behind the window so you can play normally while stats float on top.")
                                .color(theme.text_dim)
                                .monospace()
                                .size(9.5),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("TO INTERACT WHILE PASSTHROUGH IS ON")
                                .color(theme.text_subtle)
                                .monospace()
                                .size(9.5)
                                .strong(),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("Hold Ctrl — clicks go to this app instead of passing through.  Use it to drag, resize, open settings, or click the crosshair button to turn passthrough off.")
                                .color(theme.text_dim)
                                .monospace()
                                .size(9.5),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Both passthrough and pin-on-top reset to OFF on restart.")
                                .color(theme.text_subtle)
                                .monospace()
                                .size(9.5)
                                .italics(),
                        );

                        // ── Reset ─────────────────────────────────────────
                        ui.add_space(12.0);
                        divider(ui, theme.accent_cpu, 0.35);
                        ui.add_space(8.0);

                        ui.vertical_centered(|ui| {
                            let reset_btn = egui::Button::new(
                                RichText::new("RESET TO DEFAULTS")
                                    .color(theme.crit)
                                    .monospace()
                                    .size(10.5),
                            )
                            .fill(Color32::from_rgba_unmultiplied(30, 10, 10, 220))
                            .stroke(Stroke::new(1.0, theme.crit))
                            .min_size(Vec2::new(160.0, 24.0));

                            if ui.add(reset_btn)
                                .on_hover_text("Show all cards · opacity 100%")
                                .clicked()
                            {
                                if let Ok(mut vis) = card_vis.lock() {
                                    *vis = CardVisibility::default();
                                    vis.save();
                                }
                            }
                        });

                        // ── Close ─────────────────────────────────────────
                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            let btn = egui::Button::new(
                                RichText::new("CLOSE")
                                    .color(theme.text_primary)
                                    .monospace()
                                    .size(11.0),
                            )
                            .fill(Color32::from_rgba_unmultiplied(30, 30, 50, 220))
                            .stroke(Stroke::new(1.0, theme.card_border))
                            .min_size(Vec2::new(100.0, 26.0));

                            if ui.add(btn).clicked() {
                                ctx.data_mut(|d| d.insert_temp(egui::Id::new(CLOSE_ID), true));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });

                        ui.add_space(12.0);
                    }); // ScrollArea
                });
        },
    );
}

fn toggle(ui: &mut egui::Ui, theme: Theme, value: &mut bool, label: &str) -> bool {
    let color = if *value { theme.text_primary } else { theme.text_dim };
    ui.checkbox(value, RichText::new(label).color(color).monospace().size(10.5))
        .changed()
}

fn row(ui: &mut egui::Ui, theme: Theme, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:<10}", label))
                .color(theme.text_subtle)
                .monospace()
                .size(10.5),
        );
        ui.label(RichText::new("·").color(theme.text_dim).monospace().size(10.5));
        ui.add_space(4.0);
        ui.label(RichText::new(value).color(theme.text_primary).monospace().size(10.5));
    });
}

fn section(ui: &mut egui::Ui, theme: Theme, title: &str) {
    ui.add_space(10.0);
    divider(ui, theme.accent_cpu, 0.35);
    ui.add_space(6.0);
    ui.label(
        RichText::new(title)
            .color(theme.accent_cpu)
            .monospace()
            .size(10.5)
            .strong(),
    );
    ui.add_space(4.0);
}

fn divider(ui: &mut egui::Ui, color: Color32, alpha_frac: f32) {
    let alpha = (255.0 * alpha_frac) as u8;
    let [r, g, b, _] = color.to_array();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 1.0),
        egui::Sense::hover(),
    );
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, alpha)),
    );
}
