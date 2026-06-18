//! Compact metric cards — fill bar, mini sparkline, peak tick, pulse animation.

use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, Rect, Rounding, Sense, Vec2};

use crate::{
    app::{CardVisibility, MonitorApp},
    models::{fmt_bytes, fmt_bps_parts, FpsSnapshot, PingSnapshot, SystemSnapshot},
};

use super::widgets::{bar_fill_gradient, glow_card, mini_sparkline};

// ── Grid entry point ──────────────────────────────────────────────────────────

pub fn show_grid(
    app: &mut MonitorApp,
    ui: &mut Ui,
    snap: &SystemSnapshot,
    fps: &FpsSnapshot,
    ping: &PingSnapshot,
    vis: &CardVisibility,
) {
    let fs = vis.compact_font_size;
    ui.spacing_mut().item_spacing.y = 2.0;

    {
        let hist = app.hist_cpu.as_vec();
        compact_row(ui, &app.theme, "CPU", app.theme.accent_cpu,
            &compact_cpu_val(snap), compact_cpu_color(app, snap), fs,
            Some(snap.cpu.total_usage), &compact_cpu_sub(snap),
            &hist, 100.0, Some(app.peak_cpu));
    }
    {
        let hist = app.hist_mem.as_vec();
        compact_row(ui, &app.theme, "MEM", app.theme.accent_mem,
            &compact_mem_val(snap), compact_mem_color(app, snap), fs,
            Some(snap.memory.usage_percent()), &compact_mem_sub(snap),
            &hist, 100.0, Some(app.peak_mem));
    }
    draw_card_anim(ui, app, 0, |ui, app| {
        let val  = compact_fps_val(fps);
        let col  = compact_fps_color(app, fps);
        let sub  = compact_fps_sub(fps);
        let hist = app.hist_fps.as_vec();
        compact_row(ui, &app.theme, "FPS", app.theme.accent_net, &val, col, fs,
            None, &sub, &hist, 120.0, None);
    });
    draw_card_anim(ui, app, 1, |ui, app| {
        let val  = compact_gpu_val(snap);
        let col  = compact_gpu_color(app, snap);
        let sub  = compact_gpu_sub(snap);
        let bar  = snap.gpu.utilization_percent.filter(|_| snap.gpu.available);
        let hist = app.hist_gpu.as_vec();
        let peak = if snap.gpu.available { Some(app.peak_gpu) } else { None };
        compact_row(ui, &app.theme, "GPU", app.theme.accent_gpu, &val, col, fs,
            bar, &sub, &hist, 100.0, peak);
    });
    draw_card_anim(ui, app, 2, |ui, app| {
        let val     = compact_net_val(snap);
        let sub     = compact_net_sub(snap);
        let cap_bps = vis.net_cap_mbps * 125_000.0_f32;
        let rx_pct  = (snap.network.total_rx_bps as f32 / cap_bps * 100.0).clamp(0.0, 100.0);
        let col = if rx_pct >= 90.0      { app.theme.crit }
                  else if rx_pct >= 70.0 { app.theme.warn }
                  else                   { app.theme.accent_net };
        let hist     = app.hist_rx.as_vec();
        let hist_max = hist.iter().cloned().fold(cap_bps as f64, f64::max);
        let peak_pct = (app.peak_net_rx / cap_bps * 100.0).clamp(0.0, 100.0);
        compact_row(ui, &app.theme, "NET", app.theme.accent_net, &val, col, fs,
            Some(rx_pct), &sub, &hist, hist_max as f32, Some(peak_pct));
    });
    draw_card_anim(ui, app, 3, |ui, app| {
        let val  = compact_disk_val(snap);
        let col  = compact_disk_color(app, snap);
        let sub  = compact_disk_sub(snap);
        let bar  = snap.disks.first().map(|d| d.usage_percent());
        let hist = app.hist_disk.as_vec();
        compact_row(ui, &app.theme, "DISK", app.theme.accent_disk, &val, col, fs,
            bar, &sub, &hist, 100.0, Some(app.peak_disk));
    });
    draw_card_anim(ui, app, 4, |ui, app| {
        let val  = compact_temp_val(snap);
        let col  = compact_temp_color(app, snap);
        let sub  = compact_temp_sub(snap);
        let bar  = snap.temps.cpu_celsius.map(|t| t.clamp(0.0, 100.0));
        let hist = app.hist_temp_cpu.as_vec();
        let peak = snap.temps.cpu_celsius.map(|_| app.peak_temp);
        compact_row(ui, &app.theme, "TEMP", app.theme.accent_temp, &val, col, fs,
            bar, &sub, &hist, 100.0, peak);
    });
    draw_card_anim(ui, app, 5, |ui, app| {
        let val  = compact_ping_val(ping);
        let col  = compact_ping_color(app, ping);
        let sub  = compact_ping_sub(ping);
        // Scale gauge 0–200 ms; clamp so > 200 ms just pegs the bar.
        let bar  = ping.latency_ms.map(|ms| (ms as f32 / 200.0 * 100.0).clamp(0.0, 100.0));
        let hist = app.hist_ping.as_vec();
        compact_row(ui, &app.theme, "PING", app.theme.accent_net, &val, col, fs,
            bar, &sub, &hist, 200.0, None);
    });
}

/// Render an optional card with a height-collapse animation.
///
/// `slot` indexes [fps=0, gpu=1, net=2, disk=3, temp=4].
fn draw_card_anim<F>(ui: &mut Ui, app: &mut MonitorApp, slot: usize, draw_fn: F)
where
    F: FnOnce(&mut Ui, &mut MonitorApp),
{
    let scale    = app.card_anim.scale[slot];
    let stored_h = app.card_anim.height[slot];

    if scale <= 0.001 { return; }

    if scale >= 0.999 {
        let top = ui.cursor().top();
        draw_fn(ui, app);
        let spacing = ui.spacing().item_spacing.y;
        let h = (ui.cursor().top() - top - spacing).max(0.0);
        if h > 4.0 { app.card_anim.height[slot] = h; }
    } else {
        let alloc_h = (stored_h * scale).max(1.0);
        let avail_w = ui.available_width();
        let (clip_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, alloc_h), Sense::hover());
        let full_rect = Rect::from_min_size(clip_rect.min, Vec2::new(avail_w, stored_h));
        #[allow(deprecated)]
        let mut child = ui.child_ui_with_id_source(
            full_rect,
            Layout::top_down(Align::LEFT),
            slot,
            None,
        );
        child.set_clip_rect(child.clip_rect().intersect(clip_rect));
        draw_fn(&mut child, app);
    }
}

// ── Compact row ───────────────────────────────────────────────────────────────

fn compact_row(
    ui: &mut Ui,
    theme: &crate::theme::Theme,
    label: &str,
    accent: Color32,
    val: &str,
    val_color: Color32,
    font_size: f32,
    fill_pct: Option<f32>,   // 0–100 fill bar; None = no bar
    sub_label: &str,         // secondary info; "" = none
    spark_data: &[f64],      // history for mini sparkline; &[] = skip
    spark_max: f32,          // normalization ceiling for sparkline
    peak_pct: Option<f32>,   // session peak 0-100; tick on fill bar
) {
    const BAR_H:  f32 = 3.0;
    const BAR_GAP: f32 = 4.0;
    const SUB_C: Color32 = Color32::from_rgb(0x68, 0x8E, 0x78);

    let top_y = ui.cursor().top();

    glow_card(ui, theme, accent, |ui| {
        let has_sub = !sub_label.is_empty();
        let has_bar = fill_pct.is_some();

        let bar_reserve = if has_bar { BAR_H + BAR_GAP } else { 0.0 };
        let min_h = if has_sub { 38.0 } else { 28.0 };
        let row_h = (font_size + 8.0 + bar_reserve).max(min_h);

        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
        let p = ui.painter();

        let content_bot = rect.max.y - bar_reserve;
        let cy = (rect.min.y + content_bot) * 0.5;

        // Left accent spine
        let spine_h = (font_size * 0.65).clamp(12.0, 28.0);
        p.rect_filled(
            egui::Rect::from_center_size(egui::pos2(rect.min.x + 1.5, cy), Vec2::new(3.0, spine_h)),
            Rounding::same(1.5),
            accent,
        );

        // Mini sparkline — trend ghost in the right zone, drawn before text so it sits behind
        if spark_data.len() >= 2 && spark_max > 0.0 {
            let spark_x0 = rect.min.x + rect.width() * 0.25;
            let spark_rect = egui::Rect::from_min_max(
                egui::pos2(spark_x0, rect.min.y + 2.0),
                egui::pos2(rect.max.x - 4.0, content_bot - 2.0),
            );
            if spark_rect.width() > 10.0 && spark_rect.height() > 4.0 {
                mini_sparkline(p, spark_rect, spark_data, spark_max, val_color);
            }
        }

        // Label + optional sub-label stacked on the left
        let tx = rect.min.x + 11.0;
        if has_sub {
            p.text(egui::pos2(tx, cy - 7.0), Align2::LEFT_CENTER, label,
                FontId::new(10.5, FontFamily::Monospace), accent);
            p.text(egui::pos2(tx, cy + 7.0), Align2::LEFT_CENTER, sub_label,
                FontId::new(9.5, FontFamily::Proportional), SUB_C);
        } else {
            p.text(egui::pos2(tx, cy), Align2::LEFT_CENTER, label,
                FontId::new(10.5, FontFamily::Monospace), accent);
        }

        // Value — right-aligned, neon glow
        let vp  = egui::pos2(rect.max.x - 4.0, cy);
        let fid = FontId::new(font_size, FontFamily::Monospace);
        let [r, g, b, _] = val_color.to_array();
        let halo = Color32::from_rgba_unmultiplied(r, g, b, 18);
        for (dx, dy) in [(-1.0f32, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            p.text(egui::pos2(vp.x + dx, vp.y + dy), Align2::RIGHT_CENTER, val, fid.clone(), halo);
        }
        p.text(vp, Align2::RIGHT_CENTER, val, fid, val_color);

        // Fill bar
        if let Some(pct) = fill_pct {
            let bar_top = rect.max.y - BAR_H;
            let bar_bot = rect.max.y;
            let full = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, bar_top),
                egui::pos2(rect.max.x, bar_bot),
            );
            p.rect_filled(full, Rounding::same(1.5),
                Color32::from_rgba_unmultiplied(r, g, b, 22));
            let fill_w = rect.width() * (pct / 100.0).clamp(0.0, 1.0);
            if fill_w > 0.5 {
                let fill_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x, bar_top),
                    egui::pos2(rect.min.x + fill_w, bar_bot),
                );
                let left_col  = Color32::from_rgba_unmultiplied(r, g, b, 190);
                let right_col = Color32::from_rgba_unmultiplied(
                    (r as u32 * 50 / 100).min(255) as u8,
                    (g as u32 * 60 / 100).min(255) as u8,
                    (b as u32 * 55 / 100).min(255) as u8,
                    110,
                );
                bar_fill_gradient(p, fill_rect, left_col, right_col);
            }
            // Session peak tick
            if let Some(peak) = peak_pct {
                let peak_x = rect.min.x + rect.width() * (peak / 100.0).clamp(0.0, 1.0);
                if peak_x > rect.min.x + 2.0 {
                    p.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(peak_x - 1.0, bar_top - 1.0),
                            egui::pos2(peak_x + 1.0, bar_bot + 1.0),
                        ),
                        Rounding::ZERO,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                    );
                }
            }
        }
    });

    // Threshold pulse — animated glow rings in warn/crit state
    let is_warn = val_color == theme.warn;
    let is_crit = val_color == theme.crit;
    if is_warn || is_crit {
        let bot_y = (ui.cursor().top() - ui.spacing().item_spacing.y).max(top_y + 1.0);
        let card_rect = egui::Rect::from_min_max(
            egui::pos2(ui.max_rect().left(), top_y),
            egui::pos2(ui.max_rect().right(), bot_y),
        );
        let t = ui.ctx().input(|i| i.time) as f32;
        let freq = if is_crit { 1.8_f32 } else { 1.0 };
        let wave = (t * freq * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let [r, g, b, _] = val_color.to_array();
        let max_a = if is_crit { 80u8 } else { 50u8 };
        let alpha = (wave * max_a as f32) as u8 + if is_crit { 15u8 } else { 8u8 };

        let painter = ui.painter();
        for i in 1u8..=3 {
            let a = (alpha as u32 * 3 / (i as u32 * i as u32 + 2)).min(255) as u8;
            if a > 1 {
                let spread = i as f32 * 1.8;
                painter.rect_stroke(
                    card_rect.expand(spread),
                    Rounding::same(7.0 + spread),
                    egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(r, g, b, a)),
                );
            }
        }
        ui.ctx().request_repaint();
    }
}

// ── Value formatters ──────────────────────────────────────────────────────────

fn compact_cpu_val(snap: &SystemSnapshot) -> String {
    format!("{:.1}%", snap.cpu.total_usage)
}
fn compact_cpu_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    let p = snap.cpu.total_usage;
    if p >= 85.0 { app.theme.crit } else if p >= 60.0 { app.theme.warn } else { app.theme.accent_cpu }
}
fn compact_cpu_sub(snap: &SystemSnapshot) -> String {
    let ghz = snap.cpu.frequency_mhz as f32 / 1000.0;
    format!("{ghz:.1} GHz · {}c", snap.cpu.logical_cores)
}

fn compact_mem_val(snap: &SystemSnapshot) -> String {
    format!("{:.1}%", snap.memory.usage_percent())
}
fn compact_mem_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    let p = snap.memory.usage_percent();
    if p >= 90.0 { app.theme.crit } else if p >= 70.0 { app.theme.warn } else { app.theme.accent_mem }
}
fn compact_mem_sub(snap: &SystemSnapshot) -> String {
    format!("{} / {}", fmt_bytes(snap.memory.used_bytes), fmt_bytes(snap.memory.total_bytes))
}

fn compact_fps_val(fps: &FpsSnapshot) -> String {
    if fps.active { format!("{:.0} fps", fps.fps) } else { "— fps".into() }
}
fn compact_fps_color(app: &MonitorApp, fps: &FpsSnapshot) -> Color32 {
    if !fps.active          { app.theme.text_subtle }
    else if fps.fps >= 60.0 { app.theme.accent_net }
    else if fps.fps >= 30.0 { app.theme.warn }
    else                    { app.theme.crit }
}
fn compact_fps_sub(fps: &FpsSnapshot) -> String {
    if fps.active && !fps.window_title.is_empty() {
        let t = &fps.window_title;
        if t.chars().count() > 18 { format!("{}…", t.chars().take(17).collect::<String>()) } else { t.clone() }
    } else if !fps.active {
        "no game".into()
    } else {
        String::new()
    }
}

fn compact_gpu_val(snap: &SystemSnapshot) -> String {
    if snap.gpu.available {
        snap.gpu.utilization_percent.map(|p| format!("{p:.0}%")).unwrap_or_else(|| "—".into())
    } else { "—".into() }
}
fn compact_gpu_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    match snap.gpu.utilization_percent.filter(|_| snap.gpu.available) {
        Some(p) if p >= 90.0 => app.theme.crit,
        Some(p) if p >= 70.0 => app.theme.warn,
        Some(_)              => app.theme.accent_gpu,
        None                 => app.theme.text_subtle,
    }
}
fn compact_gpu_sub(snap: &SystemSnapshot) -> String {
    if !snap.gpu.available { return "unavailable".into(); }
    if snap.gpu.vram_total_bytes > 0 {
        format!("{} / {}", fmt_bytes(snap.gpu.vram_used_bytes), fmt_bytes(snap.gpu.vram_total_bytes))
    } else if !snap.gpu.name.is_empty() {
        let n = &snap.gpu.name;
        if n.chars().count() > 18 { format!("{}…", n.chars().take(17).collect::<String>()) } else { n.clone() }
    } else {
        String::new()
    }
}

fn compact_net_val(snap: &SystemSnapshot) -> String {
    let (num, unit) = fmt_bps_parts(snap.network.total_rx_bps);
    format!("↓{num} {unit}")
}
fn compact_net_sub(snap: &SystemSnapshot) -> String {
    let (num, unit) = fmt_bps_parts(snap.network.total_tx_bps);
    format!("↑{num} {unit}")
}

fn compact_disk_val(snap: &SystemSnapshot) -> String {
    snap.disks.first().map(|d| format!("{:.0}%", d.usage_percent())).unwrap_or_else(|| "—".into())
}
fn compact_disk_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    snap.disks.first()
        .map(|d| {
            let p = d.usage_percent();
            if p >= 90.0 { app.theme.crit } else if p >= 75.0 { app.theme.warn } else { app.theme.accent_disk }
        })
        .unwrap_or(app.theme.text_subtle)
}
fn compact_disk_sub(snap: &SystemSnapshot) -> String {
    snap.disks.first().map(|d| {
        let free = d.total_bytes.saturating_sub(d.used_bytes);
        format!("{} free", fmt_bytes(free))
    }).unwrap_or_default()
}

fn compact_temp_val(snap: &SystemSnapshot) -> String {
    snap.temps.cpu_celsius.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "—".into())
}
fn compact_temp_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    snap.temps.cpu_celsius
        .map(|t| if t >= 85.0 { app.theme.crit } else if t >= 70.0 { app.theme.warn } else { app.theme.accent_temp })
        .unwrap_or(app.theme.text_subtle)
}
fn compact_temp_sub(snap: &SystemSnapshot) -> String {
    match snap.temps.gpu_celsius {
        Some(t) => format!("GPU  {t:.0}°C"),
        None if snap.temps.cpu_celsius.is_some() => "GPU  —".into(),
        None => "no sensor".into(),
    }
}

fn compact_ping_val(ping: &PingSnapshot) -> String {
    if ping.sample_count == 0 {
        return "— ms".into();
    }
    match ping.latency_ms {
        Some(ms) => format!("{ms} ms"),
        None     => "— ms".into(),
    }
}

fn compact_ping_color(app: &MonitorApp, ping: &PingSnapshot) -> Color32 {
    if ping.sample_count == 0 {
        return app.theme.text_subtle;
    }
    if ping.loss_pct >= 100.0 {
        return app.theme.crit;
    }
    match ping.latency_ms {
        None                          => app.theme.crit,
        Some(ms) if ms > 150         => app.theme.crit,
        Some(ms) if ms > 50          => app.theme.warn,
        _                            => app.theme.accent_net,
    }
}

fn compact_ping_sub(ping: &PingSnapshot) -> String {
    if ping.sample_count == 0 {
        return "measuring…".into();
    }
    if ping.loss_pct >= 100.0 {
        return "OFFLINE".into();
    }
    let quality = match ping.avg_ms as u32 {
        0..=20   => "EXCELLENT",
        21..=50  => "GOOD",
        51..=150 => "FAIR",
        _        => "POOR",
    };
    if ping.loss_pct > 0.0 {
        format!("avg {:.0}ms  loss {:.0}%", ping.avg_ms, ping.loss_pct)
    } else {
        format!("avg {:.0}ms  {quality}", ping.avg_ms)
    }
}

use egui::Ui;
