//! Spectrum-analyser metric cards — glowing bars + reflections for all metrics.

use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, Rounding, Sense, Vec2};

use crate::{
    app::MonitorApp,
    models::{fmt_bytes, fmt_bps, FpsSnapshot, SystemSnapshot},
    theme::Theme,
};

use super::widgets::{fps_color, glow_card, gradient_bar, spectrum_bars, vu_color};

// Gradient endpoints: right-edge colour for the spectrum bar left→right fade
const CPU_END: egui::Color32 = egui::Color32::from_rgb( 28, 130, 215); // sapphire → cobalt
const MEM_END: egui::Color32 = egui::Color32::from_rgb(195,  75,  15); // amber → burnt umber

// Bar heights
const SPEC_H: f32 = 44.0;       // single-column cards
const SPEC_H_2COL: f32 = 36.0;  // two-column cards (NET, TEMPS)

// ── Grid entry point ──────────────────────────────────────────────────────────

pub fn show_grid(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot, fps: &FpsSnapshot) {
    ui.spacing_mut().item_spacing.y = 4.0;
    cpu_card(app, ui, snap);
    memory_card(app, ui, snap);
    fps_card(app, ui, fps);
    gpu_card(app, ui, snap);
    network_card(app, ui, snap);
    disk_card(app, ui, snap);
    temps_card(app, ui, snap);
}

// ── Shared helpers ────────────────────────────────────────────────────────────

// Header without a right-aligned value — used by split-layout cards where
// the value is painted by `draw_right_panel` instead.
fn card_hdr_no_val(ui: &mut Ui, label: &str, subtitle: &str, accent: Color32, text_subtle: Color32) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
        ui.painter().rect_filled(r, Rounding::same(1.5), accent);
        ui.add_space(5.0);
        ui.label(egui::RichText::new(label).color(accent).size(11.5).strong());
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(truncate(subtitle, 18)).color(text_subtle).size(9.5));
        }
    });
}

// Paint tinted right panel + health stripe + big glowing centred value.
// Call AFTER rendering left content so `content_rect` (ui.min_rect()) is accurate.
fn draw_right_panel(
    ui: &mut Ui,
    content_rect: egui::Rect,
    full_right: f32,
    right_w: f32,
    big_text: &str,
    unit: &str,       // optional unit below (e.g. "fps"), "" to skip
    text_color: Color32,
    dim: Color32,
    _card_border: Color32,
) {
    let div_x    = full_right - right_w;
    let mid_y    = content_rect.center().y;
    let right_cx = full_right - right_w * 0.5;
    let [r, g, b, _] = text_color.to_array();

    // Subtle dark tint behind the right panel
    ui.painter().rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(div_x, content_rect.min.y),
            egui::pos2(full_right, content_rect.max.y),
        ),
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 38),
    );

    // Health-colored left edge stripe (replaces plain divider)
    ui.painter().rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(div_x, content_rect.min.y),
            egui::pos2(div_x + 2.0, content_rect.max.y),
        ),
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(r, g, b, 180),
    );

    // Neon glow + solid big value
    let (num_y, anchor) = if unit.is_empty() {
        (mid_y, Align2::CENTER_CENTER)
    } else {
        (mid_y - 4.0, Align2::CENTER_BOTTOM)
    };
    let pos = egui::pos2(right_cx, num_y);

    // Outer glow — 4 diagonal offsets at 2 px
    let outer = Color32::from_rgba_unmultiplied(r, g, b, 28);
    for (dx, dy) in [(-2.0f32, -2.0), (2.0, -2.0), (-2.0, 2.0), (2.0, 2.0)] {
        ui.painter().text(egui::pos2(pos.x + dx, pos.y + dy), anchor, big_text, FontId::new(28.0, FontFamily::Monospace), outer);
    }
    // Inner glow — 4 cardinal offsets at 1 px
    let inner = Color32::from_rgba_unmultiplied(r, g, b, 50);
    for (dx, dy) in [(-1.0f32, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        ui.painter().text(egui::pos2(pos.x + dx, pos.y + dy), anchor, big_text, FontId::new(28.0, FontFamily::Monospace), inner);
    }
    // Solid core
    ui.painter().text(pos, anchor, big_text, FontId::new(28.0, FontFamily::Monospace), text_color);

    if !unit.is_empty() {
        ui.painter().text(
            egui::pos2(right_cx, mid_y + 2.0),
            Align2::CENTER_TOP,
            unit,
            FontId::new(10.0, FontFamily::Proportional),
            dim,
        );
    }
}

fn stat_line(ui: &mut Ui, left: &str, right: &str, dim: Color32, accent: Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(left).color(dim).size(10.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(egui::RichText::new(right).color(accent).monospace().size(10.0));
        });
    });
}

// Draws a subtle horizontal zone separator — 0.5 px line in the item-spacing gap.
// Spans ui.max_rect() so it naturally respects set_max_width in split-layout cards.
fn zone_sep(ui: &mut Ui, accent: Color32) {
    let y = ui.cursor().top() - ui.spacing().item_spacing.y * 0.5;
    let [r, g, b, _] = accent.to_array();
    ui.painter().line_segment(
        [
            egui::pos2(ui.max_rect().left() + 3.0, y),
            egui::pos2(ui.max_rect().right() - 3.0, y),
        ],
        egui::Stroke::new(0.5, Color32::from_rgba_unmultiplied(r, g, b, 28)),
    );
}

// ── CPU ───────────────────────────────────────────────────────────────────────

fn cpu_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent    = app.theme.accent_cpu;
    let pct       = snap.cpu.total_usage;
    let val_color = app.theme.health_color(pct, 60.0, 85.0);
    let subtle    = app.theme.text_subtle;
    let dim       = app.theme.text_dim;
    let border    = app.theme.card_border;

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        card_hdr_no_val(ui, "CPU", &snap.cpu.brand, accent, subtle);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_cpu, 100.0, accent, Some(CPU_END), SPEC_H, vu_color);
        zone_sep(ui, accent);
        stat_line(
            ui,
            &format!("{} cores  ·  {} MHz", snap.cpu.logical_cores, snap.cpu.frequency_mhz),
            &format!("{pct:.1}%"),
            dim, subtle,
        );
        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &format!("{pct:.0}%"), "", val_color, dim, border);
    });
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn memory_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent    = app.theme.accent_mem;
    let pct       = snap.memory.usage_percent();
    let val_color = app.theme.health_color(pct, 70.0, 90.0);
    let subtle    = app.theme.text_subtle;
    let dim       = app.theme.text_dim;
    let border    = app.theme.card_border;

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        card_hdr_no_val(ui, "MEM", &fmt_bytes(snap.memory.total_bytes), accent, subtle);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_mem, 100.0, accent, Some(MEM_END), SPEC_H, vu_color);
        zone_sep(ui, accent);
        stat_line(
            ui,
            &format!("{} used", fmt_bytes(snap.memory.used_bytes)),
            &format!("{} free", fmt_bytes(snap.memory.available_bytes)),
            dim, subtle,
        );
        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &format!("{pct:.0}%"), "", val_color, dim, border);
    });
}

// ── FPS ───────────────────────────────────────────────────────────────────────

fn fps_card(app: &mut MonitorApp, ui: &mut Ui, fps: &FpsSnapshot) {
    let accent    = app.theme.accent_net;
    let subtle    = app.theme.text_subtle;
    let dim       = app.theme.text_dim;
    let border    = app.theme.card_border;

    let val_color = if !fps.active   { dim }
                   else if fps.fps >= 60.0 { app.theme.ok }
                   else if fps.fps >= 30.0 { app.theme.warn }
                   else                    { app.theme.crit };
    let big_num   = if fps.active { format!("{:.0}", fps.fps) } else { "—".into() };
    let unit      = if fps.active { "fps" } else { "" };
    let ft        = if fps.active && fps.fps > 0.0 { format!("{:.1} ms", 1000.0 / fps.fps) } else { String::new() };

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        if !fps.active {
            card_hdr_no_val(ui, "FPS", "no game detected", accent, dim);
            zone_sep(ui, accent);
            ui.label(egui::RichText::new("Switch to a DirectX / Vulkan window").color(dim).size(10.0));
        } else {
            card_hdr_no_val(ui, "FPS", &truncate(&fps.window_title, 18), accent, subtle);
            zone_sep(ui, accent);
            spectrum_bars(ui, &app.disp_fps, 120.0, accent, None, SPEC_H, fps_color);
            zone_sep(ui, accent);
            stat_line(ui, &truncate(&fps.window_title, 26), &ft, dim, subtle);
        }

        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &big_num, unit, val_color, dim, border);
    });
}

// ── GPU ───────────────────────────────────────────────────────────────────────

fn gpu_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent    = app.theme.accent_gpu;
    let subtle    = app.theme.text_subtle;
    let dim       = app.theme.text_dim;
    let border    = app.theme.card_border;

    let pct       = snap.gpu.utilization_percent.unwrap_or(0.0);
    let val_color = app.theme.health_color(pct, 70.0, 90.0);
    let big_val   = if !snap.gpu.available {
        "—".into()
    } else {
        snap.gpu.utilization_percent
            .map(|p| format!("{p:.0}%"))
            .unwrap_or_else(|| "N/A".into())
    };
    let big_color = if snap.gpu.available && snap.gpu.utilization_percent.is_some() { val_color } else { dim };

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        if !snap.gpu.available {
            card_hdr_no_val(ui, "GPU", "unavailable", accent, dim);
            zone_sep(ui, accent);
            ui.label(egui::RichText::new("WMI data not available").color(dim).size(10.0));
        } else {
            card_hdr_no_val(ui, "GPU", &snap.gpu.name, accent, subtle);
            zone_sep(ui, accent);
            spectrum_bars(ui, &app.disp_gpu, 100.0, accent, None, SPEC_H, vu_color);
            let vram_str = if snap.gpu.vram_total_bytes > 0 {
                format!("{} / {}", fmt_bytes(snap.gpu.vram_used_bytes), fmt_bytes(snap.gpu.vram_total_bytes))
            } else { String::new() };
            let temp_str = snap.gpu.temperature_celsius.map(|t| format!("{t:.0}°C")).unwrap_or_default();
            let right = [&vram_str, &temp_str].iter().filter(|s| !s.is_empty()).map(|s| s.as_str()).collect::<Vec<_>>().join("  ·  ");
            zone_sep(ui, accent);
            stat_line(ui, "VRAM", &right, dim, subtle);
        }

        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &big_val, "", big_color, dim, border);
    });
}

// ── Network ───────────────────────────────────────────────────────────────────

fn network_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let acc_tx = app.theme.accent_net;   // green — upload
    let acc_rx = app.theme.accent_cpu;   // cyan  — download
    let subtle = app.theme.text_subtle;
    let dim = app.theme.text_dim;

    // Use raw history for peak scaling (not lerped values) so scale is accurate
    let tx_max = app.hist_tx.as_vec().iter().cloned().fold(8_000.0f64, f64::max) as f32;
    let rx_max = app.hist_rx.as_vec().iter().cloned().fold(8_000.0f64, f64::max) as f32;

    glow_card(ui, &app.theme, acc_tx, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        // Header — TX↑ and RX↓ values
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
            ui.painter().rect_filled(r, Rounding::same(1.5), acc_tx);
            ui.add_space(5.0);
            ui.label(egui::RichText::new("NET").color(acc_tx).size(11.5).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(fmt_bps(snap.network.total_rx_bps))
                        .color(acc_rx)
                        .monospace()
                        .size(11.0),
                );
                ui.label(egui::RichText::new("↓").color(subtle).size(10.0));
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(fmt_bps(snap.network.total_tx_bps))
                        .color(acc_tx)
                        .monospace()
                        .size(11.0),
                );
                ui.label(egui::RichText::new("↑").color(subtle).size(10.0));
            });
        });
        zone_sep(ui, acc_tx);

        // Sub-labels
        ui.horizontal(|ui| {
            let half = ui.available_width() / 2.0;
            ui.label(egui::RichText::new("▲ TX").color(acc_tx).size(9.5));
            ui.add_space(half - 32.0);
            ui.label(egui::RichText::new("▼ RX").color(acc_rx).size(9.5));
        });

        // Two spectrum panels side by side
        let disp_tx = app.disp_tx.clone();
        let disp_rx = app.disp_rx.clone();
        ui.columns(2, |cols| {
            spectrum_bars(&mut cols[0], &disp_tx, tx_max, acc_tx, None, SPEC_H_2COL, vu_color);
            spectrum_bars(&mut cols[1], &disp_rx, rx_max, acc_rx, None, SPEC_H_2COL, vu_color);
        });

        stat_line(ui, "peak TX", &fmt_bps(tx_max as u64), dim, subtle);
    });
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn disk_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_disk;
    let dim = app.theme.text_dim;

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
            ui.painter().rect_filled(r, Rounding::same(1.5), accent);
            ui.add_space(5.0);
            ui.label(egui::RichText::new("DISKS").color(accent).size(11.5).strong());
        });
        zone_sep(ui, accent);

        if snap.disks.is_empty() {
            ui.label(egui::RichText::new("No disks found").color(dim).size(10.0));
            return;
        }

        for disk in snap.disks.iter().take(3) {
            let pct = disk.usage_percent();
            let val_color = app.theme.health_color(pct, 75.0, 90.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&disk.mount)
                        .color(accent)
                        .monospace()
                        .size(10.0)
                        .strong(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.0}%  {} / {}",
                            pct,
                            fmt_bytes(disk.used_bytes),
                            fmt_bytes(disk.total_bytes),
                        ))
                        .color(val_color)
                        .monospace()
                        .size(9.5),
                    );
                });
            });
            gradient_bar(ui, pct, 5.0, accent, Theme::dim(accent));
        }

        if snap.disks.len() > 3 {
            ui.label(
                egui::RichText::new(format!("+ {} more", snap.disks.len() - 3))
                    .color(dim)
                    .size(9.5),
            );
        }
    });
}

// ── Temperatures ──────────────────────────────────────────────────────────────

fn temps_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_temp;
    let subtle = app.theme.text_subtle;
    let dim = app.theme.text_dim;

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
            ui.painter().rect_filled(r, Rounding::same(1.5), accent);
            ui.add_space(5.0);
            ui.label(egui::RichText::new("TEMPS").color(accent).size(11.5).strong());

            if let (Some(tc), Some(tg)) = (snap.temps.cpu_celsius, snap.temps.gpu_celsius) {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{tg:.0}°"))
                            .color(app.theme.health_color(tg, 75.0, 90.0))
                            .monospace()
                            .size(13.5)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("GPU").color(subtle).size(10.0));
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{tc:.0}°"))
                            .color(app.theme.health_color(tc, 70.0, 85.0))
                            .monospace()
                            .size(13.5)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("CPU").color(subtle).size(10.0));
                });
            } else if let Some(tc) = snap.temps.cpu_celsius {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{tc:.0}°"))
                            .color(app.theme.health_color(tc, 70.0, 85.0))
                            .monospace()
                            .size(13.5)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("CPU").color(subtle).size(10.0));
                });
            }
        });
        zone_sep(ui, accent);

        if snap.temps.cpu_celsius.is_none() && snap.temps.gpu_celsius.is_none() {
            ui.label(
                egui::RichText::new("Run as Administrator for sensor access")
                    .color(dim)
                    .size(10.0),
            );
            return;
        }

        // Sub-labels
        ui.horizontal(|ui| {
            let half = ui.available_width() / 2.0;
            ui.label(egui::RichText::new("CPU °C").color(app.theme.accent_cpu).size(9.5));
            ui.add_space(half - 40.0);
            ui.label(egui::RichText::new("GPU °C").color(app.theme.accent_gpu).size(9.5));
        });

        let cpu_disp = app.disp_temp_cpu.clone();
        let gpu_disp = app.disp_temp_gpu.clone();
        let cpu_col = app.theme.accent_cpu;
        let gpu_col = app.theme.accent_gpu;
        ui.columns(2, |cols| {
            if !cpu_disp.is_empty() {
                spectrum_bars(&mut cols[0], &cpu_disp, 100.0, cpu_col, None, SPEC_H_2COL, vu_color);
            }
            if !gpu_disp.is_empty() {
                spectrum_bars(&mut cols[1], &gpu_disp, 100.0, gpu_col, None, SPEC_H_2COL, vu_color);
            }
        });
    });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

use egui::Ui;
