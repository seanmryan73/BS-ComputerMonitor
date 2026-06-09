//! Compact single-column metric cards for the 500×600 window.
//!
//! Visual anatomy of each card:
//!   ─ header row  (accent pip · metric name · subtitle · value)
//!   ─ dot gauge + inline sparkline
//!   ─ gradient bar
//!   ─ stats line
//!
//! All 7 cards fit on screen at the 500×600 minimum without scrolling.

use egui::{Align, Color32, Layout, Rounding, Sense, Vec2};

use crate::{
    app::MonitorApp,
    models::{fmt_bytes, fmt_bps, FpsSnapshot, SystemSnapshot},
    theme::Theme,
};

use super::widgets::{
    compact_card_frame, gradient_bar, layered_dots, mini_sparkline_raw,
};

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

/// Compact card header: [pip] LABEL  subtitle  ···  VALUE
fn card_hdr(
    ui: &mut Ui,
    label: &str,
    subtitle: &str,
    value: &str,
    accent: Color32,
    val_color: Color32,
    text_subtle: Color32,
) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
        ui.painter().rect_filled(r, Rounding::same(1.5), accent);
        ui.add_space(5.0);
        ui.label(egui::RichText::new(label).color(accent).size(11.5).strong());
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(truncate(subtitle, 20)).color(text_subtle).size(9.5));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .color(val_color)
                    .monospace()
                    .size(13.5)
                    .strong(),
            );
        });
    });
}

/// Dot gauge on the left + painter sparkline on the right, in a single row.
fn dots_spark(ui: &mut Ui, pct: f32, color: Color32, hist: &[f64]) {
    const SPARK_W: f32 = 65.0;
    const ROW_H: f32 = 14.0;
    let bg = Color32::from_rgba_unmultiplied(0, 0, 0, 55);

    ui.horizontal(|ui| {
        let dots_w = (ui.available_width() - SPARK_W - 5.0).max(20.0);
        ui.allocate_ui(Vec2::new(dots_w, ROW_H), |ui| {
            layered_dots(ui, pct, color, 4.0);
        });
        ui.add_space(5.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(SPARK_W, ROW_H), Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(2.0), bg);
        mini_sparkline_raw(ui.painter(), rect, hist, color);
    });
}

/// Stat text line — muted label on the left, monospace value on the right.
fn stat_line(ui: &mut Ui, left: &str, right: &str, dim: Color32, accent: Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(left).color(dim).size(10.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(egui::RichText::new(right).color(accent).monospace().size(10.0));
        });
    });
}

// ── CPU ───────────────────────────────────────────────────────────────────────

fn cpu_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_cpu;
    let pct = snap.cpu.total_usage;
    let val_color = app.theme.health_color(pct, 60.0, 85.0);
    let subtle = app.theme.text_subtle;
    let dim = app.theme.text_dim;

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        card_hdr(ui, "CPU", &snap.cpu.brand, &format!("{pct:.0}%"), accent, val_color, subtle);
        dots_spark(ui, pct, accent, &app.hist_cpu.as_vec());
        gradient_bar(ui, pct, 5.0, accent, Theme::dim(accent));

        stat_line(
            ui,
            &format!("{} MHz", snap.cpu.frequency_mhz),
            &format!("{} cores", snap.cpu.logical_cores),
            dim,
            subtle,
        );
    });
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn memory_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_mem;
    let pct = snap.memory.usage_percent();
    let val_color = app.theme.health_color(pct, 70.0, 90.0);
    let subtle = app.theme.text_subtle;
    let dim = app.theme.text_dim;
    let total_label = fmt_bytes(snap.memory.total_bytes);

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        card_hdr(ui, "MEM", &total_label, &format!("{pct:.0}%"), accent, val_color, subtle);
        dots_spark(ui, pct, accent, &app.hist_mem.as_vec());
        gradient_bar(ui, pct, 5.0, accent, Theme::dim(accent));

        stat_line(
            ui,
            &format!("{} used", fmt_bytes(snap.memory.used_bytes)),
            &format!("{} free", fmt_bytes(snap.memory.available_bytes)),
            dim,
            subtle,
        );
    });
}

// ── GPU ───────────────────────────────────────────────────────────────────────

fn gpu_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_gpu;
    let subtle = app.theme.text_subtle;
    let dim = app.theme.text_dim;

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        if !snap.gpu.available {
            card_hdr(ui, "GPU", "unavailable", "—", accent, dim, subtle);
            ui.label(egui::RichText::new("WMI data not available").color(dim).size(10.0));
            return;
        }

        let pct = snap.gpu.utilization_percent.unwrap_or(0.0);
        let val_color = app.theme.health_color(pct, 70.0, 90.0);
        let value_str = if snap.gpu.utilization_percent.is_some() {
            format!("{pct:.0}%")
        } else {
            "N/A".into()
        };

        card_hdr(ui, "GPU", &snap.gpu.name, &value_str, accent, val_color, subtle);
        dots_spark(ui, pct, accent, &app.hist_gpu.as_vec());
        gradient_bar(ui, pct, 5.0, accent, Theme::dim(accent));

        // VRAM + temp in one line
        let vram_str = if snap.gpu.vram_total_bytes > 0 {
            format!(
                "VRAM {}/{}",
                fmt_bytes(snap.gpu.vram_used_bytes),
                fmt_bytes(snap.gpu.vram_total_bytes)
            )
        } else {
            String::new()
        };
        let temp_str = snap
            .gpu
            .temperature_celsius
            .map(|t| format!("{t:.0}°C"))
            .unwrap_or_default();

        let right = match (vram_str.is_empty(), temp_str.is_empty()) {
            (false, false) => format!("{vram_str}  ·  {temp_str}"),
            (false, true) => vram_str,
            (true, false) => temp_str,
            _ => "—".into(),
        };

        if !right.is_empty() {
            stat_line(ui, "", &right, dim, subtle);
        }
    });
}

// ── FPS ───────────────────────────────────────────────────────────────────────

fn fps_card(app: &mut MonitorApp, ui: &mut Ui, fps: &FpsSnapshot) {
    let accent = app.theme.accent_net;
    let subtle = app.theme.text_subtle;
    let dim = app.theme.text_dim;

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        if !fps.active {
            card_hdr(ui, "FPS", "no game detected", "—", accent, dim, subtle);
            ui.label(
                egui::RichText::new("Switch to a DirectX / Vulkan window")
                    .color(dim)
                    .size(10.0),
            );
            return;
        }

        let val_color = if fps.fps >= 60.0 {
            app.theme.ok
        } else if fps.fps >= 30.0 {
            app.theme.warn
        } else {
            app.theme.crit
        };

        let fps_pct = (fps.fps / 120.0 * 100.0).min(100.0); // 120 fps = full gauge
        let value_str = format!("{:.0} fps", fps.fps);

        card_hdr(ui, "FPS", &fps.window_title, &value_str, accent, val_color, subtle);
        dots_spark(ui, fps_pct, val_color, &app.hist_fps.as_vec());
        gradient_bar(ui, fps_pct, 5.0, val_color, Theme::dim(val_color));

        let ft = if fps.fps > 0.0 {
            format!("{:.1} ms frame", 1000.0 / fps.fps)
        } else {
            String::new()
        };
        stat_line(ui, &truncate(&fps.window_title, 26), &ft, dim, subtle);
    });
}

// ── Network ───────────────────────────────────────────────────────────────────

fn network_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let acc_tx = app.theme.accent_net;  // green — upload
    let acc_rx = app.theme.accent_cpu;  // cyan  — download
    let subtle = app.theme.text_subtle;

    let tx_hist = app.hist_tx.as_vec();
    let rx_hist = app.hist_rx.as_vec();
    let tx_max = tx_hist.iter().cloned().fold(1.0f64, f64::max) as f32;
    let rx_max = rx_hist.iter().cloned().fold(1.0f64, f64::max) as f32;
    let tx_pct = (snap.network.total_tx_bps as f32 / tx_max * 100.0).clamp(0.0, 100.0);
    let rx_pct = (snap.network.total_rx_bps as f32 / rx_max * 100.0).clamp(0.0, 100.0);

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        // Header — TX and RX values right-aligned
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
                        .size(11.5),
                );
                ui.label(egui::RichText::new("↓").color(subtle).size(10.0));
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(fmt_bps(snap.network.total_tx_bps))
                        .color(acc_tx)
                        .monospace()
                        .size(11.5),
                );
                ui.label(egui::RichText::new("↑").color(subtle).size(10.0));
            });
        });

        // TX — arrow label + dots
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("▲").color(acc_tx).size(9.5));
            layered_dots(ui, tx_pct, acc_tx, 3.0);
        });
        gradient_bar(ui, tx_pct, 4.0, acc_tx, Theme::dim(acc_tx));

        // RX — arrow label + dots
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("▼").color(acc_rx).size(9.5));
            layered_dots(ui, rx_pct, acc_rx, 3.0);
        });
        gradient_bar(ui, rx_pct, 4.0, acc_rx, Theme::dim(acc_rx));
    });
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn disk_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_disk;
    let dim = app.theme.text_dim;

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        // Title
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
            ui.painter().rect_filled(r, Rounding::same(1.5), accent);
            ui.add_space(5.0);
            ui.label(egui::RichText::new("DISKS").color(accent).size(11.5).strong());
        });

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
                            fmt_bytes(disk.total_bytes)
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

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        // Title
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
            ui.painter().rect_filled(r, Rounding::same(1.5), accent);
            ui.add_space(5.0);
            ui.label(egui::RichText::new("TEMPS").color(accent).size(11.5).strong());
        });

        if snap.temps.cpu_celsius.is_none() && snap.temps.gpu_celsius.is_none() {
            ui.label(
                egui::RichText::new("Run as Administrator for sensor access")
                    .color(dim)
                    .size(10.0),
            );
            return;
        }

        // CPU
        if let Some(t) = snap.temps.cpu_celsius {
            let color = app.theme.health_color(t, 70.0, 85.0);
            let pct = (t / 100.0 * 100.0).clamp(0.0, 100.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("CPU").color(subtle).size(10.0));
                ui.label(
                    egui::RichText::new(format!("{t:.0}°"))
                        .color(color)
                        .monospace()
                        .size(12.0)
                        .strong(),
                );
                layered_dots(ui, pct, color, 3.0);
            });
        }

        // GPU
        if let Some(t) = snap.temps.gpu_celsius {
            let color = app.theme.health_color(t, 75.0, 90.0);
            let pct = (t / 100.0 * 100.0).clamp(0.0, 100.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("GPU").color(subtle).size(10.0));
                ui.label(
                    egui::RichText::new(format!("{t:.0}°"))
                        .color(color)
                        .monospace()
                        .size(12.0)
                        .strong(),
                );
                layered_dots(ui, pct, color, 3.0);
            });
        }
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

// Bring egui::Ui into scope (it's re-exported from egui's prelude but needs
// an explicit import when egui is a direct dep without the prelude glob).
use egui::Ui;
