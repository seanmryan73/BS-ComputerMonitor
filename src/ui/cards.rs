//! All metric cards and the responsive grid layout.

use egui::{Grid, ScrollArea, Ui, Vec2};

use crate::{
    app::MonitorApp,
    models::{fmt_bytes, fmt_bps, SystemSnapshot},
    theme::Theme,
};

use super::widgets::{bar, big_value, card_frame, card_title, mini_bar, sparkline, stat_row};

// ── Grid ──────────────────────────────────────────────────────────────────────

pub fn show_grid(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let available_w = ui.available_width();
    let cols: usize = if available_w >= 760.0 { 2 } else { 1 };
    let card_w = (available_w - (cols as f32 - 1.0) * 8.0) / cols as f32;

    ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.allocate_ui(Vec2::new(card_w, 0.0), |ui| cpu_card(app, ui, snap));
            ui.allocate_ui(Vec2::new(card_w, 0.0), |ui| memory_card(app, ui, snap));
            ui.allocate_ui(Vec2::new(card_w, 0.0), |ui| gpu_card(app, ui, snap));
            ui.allocate_ui(Vec2::new(card_w, 0.0), |ui| network_card(app, ui, snap));
            ui.allocate_ui(Vec2::new(card_w, 0.0), |ui| disk_card(app, ui, snap));
            ui.allocate_ui(Vec2::new(card_w, 0.0), |ui| temps_card(app, ui, snap));
        });
    });
}

// ── CPU ───────────────────────────────────────────────────────────────────────

fn cpu_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_cpu;
    let track = Theme::dim(accent);
    let pct = snap.cpu.total_usage;
    let val_color = app.theme.health_color(pct, 60.0, 85.0);
    let text_subtle = app.theme.text_subtle;
    let text_primary = app.theme.text_primary;
    let text_dim = app.theme.text_dim;
    let bar_rounding = app.theme.bar_rounding;

    card_frame(&app.theme).show(ui, |ui| {
        card_title(ui, "CPU", accent);

        ui.horizontal(|ui| {
            big_value(ui, &format!("{pct:.0}%"), val_color);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                ui.label(
                    egui::RichText::new(&snap.cpu.brand)
                        .color(text_subtle)
                        .size(10.0),
                );
            });
        });

        ui.add_space(4.0);
        bar(ui, pct, 8.0, accent, track, bar_rounding);
        ui.add_space(8.0);

        // Per-core mini grid
        let cores = &snap.cpu.per_core;
        let grid_cols = ((cores.len().min(16)) as f32).sqrt().ceil() as usize;
        Grid::new("cpu_cores")
            .num_columns(grid_cols * 2)
            .spacing([4.0, 3.0])
            .show(ui, |ui| {
                for (i, &c) in cores.iter().take(16).enumerate() {
                    ui.label(
                        egui::RichText::new(format!("C{i}"))
                            .color(text_dim)
                            .size(9.0),
                    );
                    ui.allocate_ui(Vec2::new(60.0, 5.0), |ui| {
                        mini_bar(ui, c, accent, track);
                    });
                    if (i + 1) % grid_cols == 0 {
                        ui.end_row();
                    }
                }
            });

        ui.add_space(8.0);
        sparkline(ui, "spark_cpu", &app.hist_cpu.as_vec(), accent, 36.0);

        stat_row(
            ui,
            "Freq",
            &format!("{} MHz", snap.cpu.frequency_mhz),
            text_subtle,
            text_primary,
        );
        stat_row(
            ui,
            "Cores",
            &snap.cpu.logical_cores.to_string(),
            text_subtle,
            text_primary,
        );
    });
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn memory_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_mem;
    let track = Theme::dim(accent);
    let pct = snap.memory.usage_percent();
    let val_color = app.theme.health_color(pct, 70.0, 90.0);
    let text_subtle = app.theme.text_subtle;
    let text_primary = app.theme.text_primary;
    let bar_rounding = app.theme.bar_rounding;

    card_frame(&app.theme).show(ui, |ui| {
        card_title(ui, "MEMORY", accent);

        ui.horizontal(|ui| {
            big_value(ui, &format!("{pct:.0}%"), val_color);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                ui.label(
                    egui::RichText::new(fmt_bytes(snap.memory.total_bytes))
                        .color(text_subtle)
                        .size(10.0),
                );
            });
        });

        ui.add_space(4.0);
        bar(ui, pct, 8.0, accent, track, bar_rounding);
        ui.add_space(8.0);
        sparkline(ui, "spark_mem", &app.hist_mem.as_vec(), accent, 36.0);

        stat_row(
            ui,
            "Used",
            &fmt_bytes(snap.memory.used_bytes),
            text_subtle,
            text_primary,
        );
        stat_row(
            ui,
            "Available",
            &fmt_bytes(snap.memory.available_bytes),
            text_subtle,
            text_primary,
        );
        if snap.memory.swap_total_bytes > 0 {
            let swap_pct =
                snap.memory.swap_used_bytes as f32 / snap.memory.swap_total_bytes as f32 * 100.0;
            stat_row(
                ui,
                "Swap",
                &format!(
                    "{} / {} ({:.0}%)",
                    fmt_bytes(snap.memory.swap_used_bytes),
                    fmt_bytes(snap.memory.swap_total_bytes),
                    swap_pct
                ),
                text_subtle,
                text_primary,
            );
        }
    });
}

// ── GPU ───────────────────────────────────────────────────────────────────────

fn gpu_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_gpu;
    let track = Theme::dim(accent);
    let text_subtle = app.theme.text_subtle;
    let text_primary = app.theme.text_primary;
    let text_dim = app.theme.text_dim;
    let bar_rounding = app.theme.bar_rounding;

    card_frame(&app.theme).show(ui, |ui| {
        card_title(ui, "GPU", accent);

        if !snap.gpu.available {
            ui.label(
                egui::RichText::new("No GPU data available")
                    .color(text_dim)
                    .size(11.0),
            );
            return;
        }

        let pct = snap.gpu.utilization_percent.unwrap_or(0.0);
        let val_color = app.theme.health_color(pct, 70.0, 90.0);
        let label = if snap.gpu.utilization_percent.is_some() {
            format!("{pct:.0}%")
        } else {
            "N/A".into()
        };

        big_value(ui, &label, val_color);
        ui.add_space(4.0);
        bar(ui, pct, 8.0, accent, track, bar_rounding);
        ui.add_space(8.0);

        if !app.hist_gpu.data.is_empty() {
            sparkline(ui, "spark_gpu", &app.hist_gpu.as_vec(), accent, 36.0);
        }

        ui.label(
            egui::RichText::new(&snap.gpu.name)
                .color(text_subtle)
                .size(10.0),
        );

        if snap.gpu.vram_total_bytes > 0 {
            let vram_pct = snap.gpu.vram_usage_percent();
            stat_row(
                ui,
                "VRAM",
                &format!(
                    "{} / {}",
                    fmt_bytes(snap.gpu.vram_used_bytes),
                    fmt_bytes(snap.gpu.vram_total_bytes)
                ),
                text_subtle,
                text_primary,
            );
            bar(ui, vram_pct, 5.0, accent, track, bar_rounding);
        }

        if let Some(t) = snap.gpu.temperature_celsius {
            stat_row(
                ui,
                "Temp",
                &format!("{t:.0}°C"),
                text_subtle,
                app.theme.health_color(t, 75.0, 90.0),
            );
        }
    });
}

// ── Network ───────────────────────────────────────────────────────────────────

fn network_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_net;
    let accent_rx = app.theme.accent_cpu;
    let text_subtle = app.theme.text_subtle;

    card_frame(&app.theme).show(ui, |ui| {
        card_title(ui, "NETWORK", accent);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("▲ TX").color(text_subtle).size(10.0));
                ui.label(
                    egui::RichText::new(fmt_bps(snap.network.total_tx_bps))
                        .color(accent)
                        .monospace()
                        .size(18.0)
                        .strong(),
                );
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("▼ RX").color(text_subtle).size(10.0));
                ui.label(
                    egui::RichText::new(fmt_bps(snap.network.total_rx_bps))
                        .color(accent_rx)
                        .monospace()
                        .size(18.0)
                        .strong(),
                );
            });
        });

        ui.add_space(8.0);
        sparkline(ui, "spark_tx", &app.hist_tx.as_vec(), accent, 28.0);
        sparkline(ui, "spark_rx", &app.hist_rx.as_vec(), accent_rx, 28.0);
        ui.add_space(4.0);

        for iface in snap.network.interfaces.iter().take(5) {
            if iface.rx_bps == 0 && iface.tx_bps == 0 {
                continue;
            }
            stat_row(
                ui,
                &truncate(&iface.name, 22),
                &format!("↑{} ↓{}", fmt_bps(iface.tx_bps), fmt_bps(iface.rx_bps)),
                text_subtle,
                app.theme.text_primary,
            );
        }
    });
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn disk_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_disk;
    let track = Theme::dim(accent);
    let text_subtle = app.theme.text_subtle;
    let text_dim = app.theme.text_dim;
    let bar_rounding = app.theme.bar_rounding;

    card_frame(&app.theme).show(ui, |ui| {
        card_title(ui, "DISKS", accent);

        if snap.disks.is_empty() {
            ui.label(
                egui::RichText::new("No disks found")
                    .color(text_dim)
                    .size(11.0),
            );
            return;
        }

        for disk in &snap.disks {
            let pct = disk.usage_percent();
            let val_color = app.theme.health_color(pct, 75.0, 90.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&disk.mount)
                        .color(accent)
                        .size(11.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {} ({:.0}%)",
                            fmt_bytes(disk.used_bytes),
                            fmt_bytes(disk.total_bytes),
                            pct
                        ))
                        .color(val_color)
                        .monospace()
                        .size(10.0),
                    );
                });
            });

            bar(ui, pct, 5.0, accent, track, bar_rounding);
            ui.add_space(2.0);

            if disk.read_bps > 0 || disk.write_bps > 0 {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "R:{}  W:{}",
                            fmt_bps(disk.read_bps),
                            fmt_bps(disk.write_bps)
                        ))
                        .color(text_subtle)
                        .monospace()
                        .size(10.0),
                    );
                });
            }
            ui.add_space(6.0);
        }
    });
}

// ── Temps ─────────────────────────────────────────────────────────────────────

fn temps_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_temp;
    let text_dim = app.theme.text_dim;

    card_frame(&app.theme).show(ui, |ui| {
        card_title(ui, "TEMPERATURES", accent);

        if snap.temps.cpu_celsius.is_none() && snap.temps.gpu_celsius.is_none() {
            ui.label(
                egui::RichText::new(
                    "Temperature data unavailable.\nRun as Administrator for full sensor access.",
                )
                .color(text_dim)
                .size(11.0),
            );
            return;
        }

        ui.horizontal(|ui| {
            temp_gauge(ui, "CPU", snap.temps.cpu_celsius, &app.theme);
            ui.add_space(24.0);
            temp_gauge(ui, "GPU", snap.temps.gpu_celsius, &app.theme);
        });
    });
}

fn temp_gauge(ui: &mut Ui, label: &str, celsius: Option<f32>, theme: &Theme) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).color(theme.text_subtle).size(10.0));
        match celsius {
            Some(t) => {
                ui.label(
                    egui::RichText::new(format!("{t:.0}°C"))
                        .color(theme.health_color(t, 70.0, 90.0))
                        .monospace()
                        .size(26.0)
                        .strong(),
                );
            }
            None => {
                ui.label(egui::RichText::new("—").color(theme.text_dim).size(26.0));
            }
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
