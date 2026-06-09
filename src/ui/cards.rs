//! Spectrum-analyser metric cards — glowing bars + reflections for all metrics.

use egui::{Align, Color32, Layout, Rounding, Sense, Vec2};

use crate::{
    app::MonitorApp,
    models::{fmt_bytes, fmt_bps, FpsSnapshot, SystemSnapshot},
    theme::Theme,
};

use super::widgets::{compact_card_frame, fps_color, gradient_bar, spectrum_bars, vu_color};

// Bar heights
const SPEC_H: f32 = 60.0;       // single-column cards
const SPEC_H_2COL: f32 = 50.0;  // two-column cards (NET, TEMPS)

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
            ui.label(egui::RichText::new(truncate(subtitle, 22)).color(text_subtle).size(9.5));
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
        spectrum_bars(ui, &app.disp_cpu, 100.0, accent, SPEC_H, vu_color);
        stat_line(
            ui,
            &format!("{} cores  ·  {} MHz", snap.cpu.logical_cores, snap.cpu.frequency_mhz),
            &format!("{pct:.1}%"),
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
    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        card_hdr(
            ui,
            "MEM",
            &fmt_bytes(snap.memory.total_bytes),
            &format!("{pct:.0}%"),
            accent,
            val_color,
            subtle,
        );
        spectrum_bars(ui, &app.disp_mem, 100.0, accent, SPEC_H, vu_color);
        stat_line(
            ui,
            &format!("{} used", fmt_bytes(snap.memory.used_bytes)),
            &format!("{} free", fmt_bytes(snap.memory.available_bytes)),
            dim,
            subtle,
        );
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

        let value_str = format!("{:.0} fps", fps.fps);
        let ft = if fps.fps > 0.0 {
            format!("{:.1} ms", 1000.0 / fps.fps)
        } else {
            String::new()
        };

        card_hdr(ui, "FPS", &truncate(&fps.window_title, 22), &value_str, accent, val_color, subtle);
        // fps_color: tall bar = high fps = green (inverted VU); 120 fps = full height
        spectrum_bars(ui, &app.disp_fps, 120.0, accent, SPEC_H, fps_color);
        stat_line(ui, &truncate(&fps.window_title, 28), &ft, dim, subtle);
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
        let value_str = snap
            .gpu
            .utilization_percent
            .map(|p| format!("{p:.0}%"))
            .unwrap_or_else(|| "N/A".into());
        card_hdr(ui, "GPU", &snap.gpu.name, &value_str, accent, val_color, subtle);
        spectrum_bars(ui, &app.disp_gpu, 100.0, accent, SPEC_H, vu_color);

        let vram_str = if snap.gpu.vram_total_bytes > 0 {
            format!(
                "{} / {}",
                fmt_bytes(snap.gpu.vram_used_bytes),
                fmt_bytes(snap.gpu.vram_total_bytes),
            )
        } else {
            String::new()
        };
        let temp_str = snap
            .gpu
            .temperature_celsius
            .map(|t| format!("{t:.0}°C"))
            .unwrap_or_default();
        let right = [&vram_str, &temp_str]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("  ·  ");

        stat_line(ui, "VRAM", &right, dim, subtle);
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

    compact_card_frame(&app.theme).show(ui, |ui| {
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
            spectrum_bars(&mut cols[0], &disp_tx, tx_max, acc_tx, SPEC_H_2COL, vu_color);
            spectrum_bars(&mut cols[1], &disp_rx, rx_max, acc_rx, SPEC_H_2COL, vu_color);
        });

        stat_line(ui, "peak TX", &fmt_bps(tx_max as u64), dim, subtle);
    });
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn disk_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_disk;
    let dim = app.theme.text_dim;

    compact_card_frame(&app.theme).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

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

    compact_card_frame(&app.theme).show(ui, |ui| {
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
                spectrum_bars(&mut cols[0], &cpu_disp, 100.0, cpu_col, SPEC_H_2COL, vu_color);
            }
            if !gpu_disp.is_empty() {
                spectrum_bars(&mut cols[1], &gpu_disp, 100.0, gpu_col, SPEC_H_2COL, vu_color);
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
