//! Settings — separate opaque OS window that pops out to the left of the main window.

use std::sync::{Arc, Mutex};

use egui::{Color32, Frame, Margin, Rect, RichText, Rounding, Stroke, Vec2, ViewportBuilder, ViewportId};

use crate::{app::CardVisibility, theme::{Theme, ThemeId}};

const CLOSE_ID: &str = "about_close_requested";

// Panel background — slightly lighter than the main app bg for visual separation.
const PANEL_BG: Color32 = Color32::from_rgb(0x0C, 0x0C, 0x16);

pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    open: &mut bool,
    card_vis: Arc<Mutex<CardVisibility>>,
    initial_pos: Option<egui::Pos2>,
    is_elevated: bool,
    gpu_names: Vec<String>,
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
        .with_title("BC Monitor — Settings")
        .with_inner_size([360.0, 700.0])
        .with_resizable(false)
        .with_maximize_button(false);

    if let Some(pos) = initial_pos {
        builder = builder.with_position(pos);
    }

    ctx.show_viewport_deferred(
        ViewportId::from_hash_of("about_viewport"),
        builder,
        move |ctx, _class| {
            let gpu_names = gpu_names.clone();
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new(CLOSE_ID), true));
            }

            // Inherit the full theme widget styles — don't wipe them with a bare dark() visuals.
            let mut vis = ctx.style().visuals.clone();
            vis.panel_fill = PANEL_BG;
            vis.selection.bg_fill = theme.accent_cpu;
            ctx.set_visuals(vis);

            // Text levels used in this panel.
            // body  — readable secondary text (~65% white), replaces the near-invisible text_subtle
            // hint  — tertiary / disabled labels, uses text_subtle (not text_dim which is invisible)
            let body = Color32::from_rgb(0xA8, 0xA8, 0xC0);
            let hint = theme.text_subtle;

            egui::CentralPanel::default()
                .frame(Frame::none().fill(PANEL_BG).inner_margin(Margin::same(18.0)))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {

                        // ── Header ────────────────────────────────────────
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("BC COMPUTER MONITOR")
                                .color(theme.accent_cpu)
                                .monospace()
                                .size(18.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("v2026.06.23  ·  System Resource Monitor")
                                .color(body)
                                .monospace()
                                .size(11.0),
                        );
                        ui.add_space(10.0);
                        row(ui, theme, body, "AUTHOR",   "Sean Ryan");
                        row(ui, theme, body, "CONTACT",  "seanmryan@gmail.com");
                        row(ui, theme, body, "COMPANY",  "BagPipes");
                        ui.add_space(2.0);
                        row(ui, theme, body, "RUNTIME",  "Rust · egui 0.29 · eframe 0.29");
                        row(ui, theme, body, "PLATFORM", "Windows · x86_64");
                        ui.add_space(2.0);
                        row_link(ui, theme, body, "DONATE", "Support on Ko-fi", "https://ko-fi.com/bagofpipes");

                        // ── Theme ─────────────────────────────────────────
                        section(ui, theme, "THEME");

                        if let Ok(mut vis) = card_vis.lock() {
                            for &id in ThemeId::ALL {
                                let selected = vis.theme_id == id;
                                let color = if selected { theme.text_primary } else { body };
                                if ui.radio(selected, RichText::new(id.label()).color(color).monospace().size(12.0)).clicked() {
                                    vis.theme_id = id;
                                    vis.save();
                                }
                            }
                        }

                        // ── Permissions ───────────────────────────────────
                        section(ui, theme, "PERMISSIONS");

                        if is_elevated {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").color(theme.ok).size(13.0));
                                ui.label(
                                    RichText::new("Running as Administrator — all sensors active")
                                        .color(theme.ok)
                                        .monospace()
                                        .size(11.5),
                                );
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("▲").color(theme.warn).size(14.0));
                                ui.label(
                                    RichText::new("Not running as Administrator")
                                        .color(theme.warn)
                                        .monospace()
                                        .size(12.0)
                                        .strong(),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Some sensors require admin rights:")
                                    .color(body)
                                    .monospace()
                                    .size(11.0),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("  · GPU utilization %\n  · CPU / GPU temperatures")
                                    .color(theme.text_primary)
                                    .monospace()
                                    .size(11.0),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("TO RUN AS ADMINISTRATOR")
                                    .color(body)
                                    .monospace()
                                    .size(11.0)
                                    .strong(),
                            );
                            ui.add_space(3.0);
                            ui.label(
                                RichText::new("Right-click the .exe → Run as administrator\n\nOr: shortcut → Properties → Advanced\n→ check \"Run as administrator\"")
                                    .color(hint)
                                    .monospace()
                                    .size(10.5),
                            );
                        }

                        // ── Appearance ────────────────────────────────────
                        section(ui, theme, "APPEARANCE");

                        if let Ok(mut vis) = card_vis.lock() {
                            ui.label(RichText::new("VALUE SIZE").color(body).monospace().size(11.0));
                            ui.add_space(2.0);
                            let slider = egui::Slider::new(
                                &mut vis.compact_font_size,
                                11.0_f32..=60.0_f32,
                            )
                            .custom_formatter(|v, _| format!("{:.0} pt", v))
                            .custom_parser(|s| s.trim_end_matches("pt").trim().parse().ok());
                            if ui.add(slider).changed() { vis.save(); }
                        }

                        // ── Visible cards ─────────────────────────────────
                        section(ui, theme, "VISIBLE CARDS");

                        let mut always = true;
                        ui.add_enabled(false, egui::Checkbox::new(
                            &mut always,
                            RichText::new("CPU  —  always visible").color(hint).monospace().size(12.0),
                        ));
                        ui.add_enabled(false, egui::Checkbox::new(
                            &mut always,
                            RichText::new("MEM  —  always visible").color(hint).monospace().size(12.0),
                        ));
                        ui.add_space(2.0);

                        if let Ok(mut vis) = card_vis.lock() {
                            let c1 = toggle(ui, theme, body, &mut vis.show_fps,  "FPS");
                            let c2 = toggle(ui, theme, body, &mut vis.show_gpu,  "GPU");
                            let c3 = toggle(ui, theme, body, &mut vis.show_net,  "NET");
                            let c4 = toggle(ui, theme, body, &mut vis.show_disk, "DISK");
                            let c5 = toggle(ui, theme, body, &mut vis.show_temp, "TEMP");
                            let c6 = toggle(ui, theme, body, &mut vis.show_ping, "PING");
                            if c1 || c2 || c3 || c4 || c5 || c6 { vis.save(); }
                        }

                        // ── GPU Adapter ───────────────────────────────────
                        section(ui, theme, "GPU ADAPTER");

                        if gpu_names.is_empty() {
                            ui.label(
                                RichText::new("Detecting…")
                                    .color(hint)
                                    .monospace()
                                    .size(11.0),
                            );
                        } else if gpu_names.len() == 1 {
                            ui.label(
                                RichText::new(&gpu_names[0])
                                    .color(theme.text_primary)
                                    .monospace()
                                    .size(11.5),
                            );
                            ui.label(
                                RichText::new("Only one GPU detected")
                                    .color(hint)
                                    .monospace()
                                    .size(10.5),
                            );
                        } else if let Ok(mut vis) = card_vis.lock() {
                            let clamped = vis.selected_gpu_index.min(gpu_names.len().saturating_sub(1));
                            let selected_name = gpu_names.get(clamped).cloned().unwrap_or_default();
                            let prev = vis.selected_gpu_index;
                            egui::ComboBox::from_id_salt("gpu_select")
                                .selected_text(
                                    RichText::new(&selected_name)
                                        .monospace()
                                        .size(11.0)
                                        .color(theme.text_primary),
                                )
                                .width(ui.available_width() - 8.0)
                                .show_ui(ui, |ui| {
                                    for (i, name) in gpu_names.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut vis.selected_gpu_index,
                                            i,
                                            RichText::new(name).monospace().size(11.0),
                                        );
                                    }
                                });
                            if vis.selected_gpu_index != prev { vis.save(); }
                        }

                        // ── Network ───────────────────────────────────────
                        section(ui, theme, "NETWORK");

                        ui.label(RichText::new("BANDWIDTH CAP").color(body).monospace().size(11.0));
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new("Sets 100% on the NET fill bar and health colours")
                                .color(hint)
                                .monospace()
                                .size(10.5),
                        );
                        ui.add_space(5.0);

                        if let Ok(mut vis) = card_vis.lock() {
                            const PRESETS: &[(f32, &str)] = &[
                                (10.0,    "10 Mbps"),
                                (25.0,    "25 Mbps"),
                                (50.0,    "50 Mbps"),
                                (100.0,   "100 Mbps  —  Fast Ethernet"),
                                (250.0,   "250 Mbps"),
                                (500.0,   "500 Mbps"),
                                (1000.0,  "1 Gbps  —  Gigabit"),
                                (2500.0,  "2.5 Gbps"),
                                (10000.0, "10 Gbps"),
                            ];
                            let cur_label = PRESETS.iter()
                                .find(|&&(v, _)| (v - vis.net_cap_mbps).abs() < 0.5)
                                .map(|&(_, l)| l)
                                .unwrap_or("Custom");
                            let prev = vis.net_cap_mbps;
                            egui::ComboBox::from_id_salt("net_cap_select")
                                .selected_text(
                                    RichText::new(cur_label)
                                        .monospace()
                                        .size(11.0)
                                        .color(theme.text_primary),
                                )
                                .width(ui.available_width() - 8.0)
                                .show_ui(ui, |ui| {
                                    for &(val, label) in PRESETS {
                                        ui.selectable_value(
                                            &mut vis.net_cap_mbps,
                                            val,
                                            RichText::new(label).monospace().size(11.0),
                                        );
                                    }
                                });
                            if (vis.net_cap_mbps - prev).abs() > 0.1 { vis.save(); }
                        }

                        // ── Ping Target ──────────────────────────────────
                        section(ui, theme, "PING TARGET");

                        ui.label(
                            RichText::new("Hostname or IP to ping (e.g. 8.8.8.8, router)")
                                .color(hint)
                                .monospace()
                                .size(10.5),
                        );
                        ui.add_space(4.0);

                        if let Ok(mut vis) = card_vis.lock() {
                            let prev = vis.ping_target.clone();
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut vis.ping_target)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(ui.available_width() - 8.0),
                            );
                            if resp.lost_focus() && vis.ping_target != prev {
                                vis.save();
                            }
                        }

                        // ── Window ────────────────────────────────────────
                        section(ui, theme, "WINDOW");

                        ui.add_space(2.0);
                        ui.label(RichText::new("OPACITY").color(body).monospace().size(11.0));
                        ui.add_space(4.0);

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

                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::Vec2::splat(30.0), egui::Sense::hover(),
                            );
                            let c = rect.center();
                            let r = 6.0_f32;
                            let gap = 2.2;
                            let stroke = egui::Stroke::new(1.4, theme.accent_cpu);
                            let p = ui.painter();
                            p.circle_stroke(c, r, stroke);
                            p.line_segment([c + egui::vec2(0.0, -(r+gap)), c + egui::vec2(0.0, -(r+gap+4.0))], stroke);
                            p.line_segment([c + egui::vec2(0.0,  r+gap),   c + egui::vec2(0.0,  r+gap+4.0)],   stroke);
                            p.line_segment([c + egui::vec2(-(r+gap), 0.0), c + egui::vec2(-(r+gap+4.0), 0.0)], stroke);
                            p.line_segment([c + egui::vec2( r+gap,   0.0), c + egui::vec2( r+gap+4.0,   0.0)], stroke);
                            p.circle_filled(c, 1.4, theme.accent_cpu);

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Click this button (top-left of title bar)\nto toggle passthrough on or off.")
                                    .color(body)
                                    .monospace()
                                    .size(11.0),
                            );
                        });

                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Mouse clicks pass through to the game/app behind the window so you can play normally while stats float on top.")
                                .color(hint)
                                .monospace()
                                .size(10.5),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("TO INTERACT WHILE PASSTHROUGH IS ON")
                                .color(body)
                                .monospace()
                                .size(11.0)
                                .strong(),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new("Hold Ctrl — clicks go to this app instead of passing through. Use it to drag, resize, open settings, or click the crosshair to turn passthrough off.")
                                .color(hint)
                                .monospace()
                                .size(10.5),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Passthrough and pin-on-top reset to OFF on restart.")
                                .color(hint)
                                .monospace()
                                .size(10.5)
                                .italics(),
                        );

                        // ── Who Made This? ────────────────────────────────
                        section(ui, theme, "WHO MADE THIS?");

                        ui.vertical_centered(|ui| {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("BagPipes")
                                    .color(theme.accent_cpu)
                                    .monospace()
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(3.0);
                            ui.label(
                                RichText::new("\"Because knowing your system is half the battle.\"")
                                    .color(hint)
                                    .monospace()
                                    .size(10.5)
                                    .italics(),
                            );
                            ui.add_space(6.0);
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("P.S.")
                                    .color(theme.warn)
                                    .monospace()
                                    .size(11.0)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "The \"BS\" stands for BagPipes Software. \
                                     We know what you were thinking. We respect it.",
                                )
                                .color(hint)
                                .monospace()
                                .size(10.5),
                            );
                        });
                        ui.add_space(6.0);

                        // ── Buttons ───────────────────────────────────────
                        ui.add_space(16.0);
                        divider(ui, theme.accent_cpu, 0.5);
                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            let btn_w = (ui.available_width() - 12.0) / 2.0;

                            let reset_btn = egui::Button::new(
                                RichText::new("RESET TO DEFAULTS")
                                    .color(theme.crit)
                                    .monospace()
                                    .size(11.0)
                                    .strong(),
                            )
                            .fill(Color32::from_rgba_unmultiplied(40, 10, 10, 240))
                            .stroke(Stroke::new(1.0, theme.crit))
                            .min_size(Vec2::new(btn_w, 30.0));

                            if ui.add(reset_btn)
                                .on_hover_text("Show all cards · opacity 100%")
                                .clicked()
                            {
                                if let Ok(mut vis) = card_vis.lock() {
                                    *vis = CardVisibility::default();
                                    vis.save();
                                }
                            }

                            ui.add_space(12.0);

                            let close_btn = egui::Button::new(
                                RichText::new("CLOSE")
                                    .color(theme.text_primary)
                                    .monospace()
                                    .size(11.0),
                            )
                            .fill(Color32::from_rgba_unmultiplied(20, 20, 40, 240))
                            .stroke(Stroke::new(1.0, theme.accent_cpu))
                            .min_size(Vec2::new(btn_w, 30.0));

                            if ui.add(close_btn).clicked() {
                                ctx.data_mut(|d| d.insert_temp(egui::Id::new(CLOSE_ID), true));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });

                        ui.add_space(14.0);
                    }); // ScrollArea
                });
        },
    );
}

fn toggle(ui: &mut egui::Ui, _theme: Theme, body: Color32, value: &mut bool, label: &str) -> bool {
    let color = if *value { Color32::from_rgb(0xEE, 0xEE, 0xF4) } else { body };
    ui.checkbox(value, RichText::new(label).color(color).monospace().size(12.0))
        .changed()
}

fn row(ui: &mut egui::Ui, theme: Theme, body: Color32, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:<10}", label))
                .color(body)
                .monospace()
                .size(11.5),
        );
        ui.label(
            RichText::new(value)
                .color(theme.text_primary)
                .monospace()
                .size(11.5),
        );
    });
}

fn row_link(ui: &mut egui::Ui, theme: Theme, body: Color32, label: &str, link_text: &str, url: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:<10}", label))
                .color(body)
                .monospace()
                .size(11.5),
        );
        ui.hyperlink_to(
            RichText::new(link_text)
                .color(theme.accent_cpu)
                .monospace()
                .size(11.5),
            url,
        );
    });
}

fn section(ui: &mut egui::Ui, theme: Theme, title: &str) {
    ui.add_space(14.0);

    // Full-width background strip for the section header.
    let avail_w = ui.available_width();
    let (strip_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(
        strip_rect,
        Rounding::same(3.0),
        Color32::from_rgba_unmultiplied(
            theme.accent_cpu.r(),
            theme.accent_cpu.g(),
            theme.accent_cpu.b(),
            22,
        ),
    );
    // Left accent bar
    let bar = Rect::from_min_size(strip_rect.min, Vec2::new(3.0, 22.0));
    ui.painter().rect_filled(bar, Rounding::same(2.0), theme.accent_cpu);
    // Title text centred vertically in the strip
    ui.painter().text(
        strip_rect.min + egui::vec2(10.0, 11.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::new(11.5, egui::FontFamily::Monospace),
        theme.accent_cpu,
    );

    ui.add_space(8.0);
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
