//! Spectrum-analyser metric cards — glowing bars + reflections for all metrics.

use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, Rounding, Sense, Vec2};

use crate::{
    app::{CardVisibility, MonitorApp},
    models::{fmt_bytes, fmt_bps, fmt_bps_parts, FpsSnapshot, SystemSnapshot},
};

use super::widgets::{fps_color, glow_card, spectrum_bars, vu_color};

// Gradient endpoints: right-edge colour for the spectrum bar left→right fade
const CPU_END: egui::Color32 = egui::Color32::from_rgb( 28, 130, 215); // sapphire → cobalt
const MEM_END: egui::Color32 = egui::Color32::from_rgb(195,  75,  15); // amber → burnt umber

const SPEC_H: f32 = 44.0;

// Left-column pixel threshold below which stat text switches to abbreviated form
const NARROW: f32 = 115.0;

// ── Grid entry point ──────────────────────────────────────────────────────────

pub fn show_grid(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot, fps: &FpsSnapshot, vis: &CardVisibility) {
    if vis.compact_mode {
        let fs = vis.compact_font_size;
        ui.spacing_mut().item_spacing.y = 2.0;
        compact_row(ui, &app.theme, "CPU",  app.theme.accent_cpu,  &compact_cpu_val(app, snap),  compact_cpu_color(app, snap),  fs);
        compact_row(ui, &app.theme, "MEM",  app.theme.accent_mem,  &compact_mem_val(app, snap),  compact_mem_color(app, snap),  fs);
        if vis.show_fps  { compact_row(ui, &app.theme, "FPS",  app.theme.accent_net,  &compact_fps_val(fps),        compact_fps_color(app, fps),  fs); }
        if vis.show_gpu  { compact_row(ui, &app.theme, "GPU",  app.theme.accent_gpu,  &compact_gpu_val(app, snap),  compact_gpu_color(app, snap), fs); }
        if vis.show_net  { compact_row(ui, &app.theme, "NET",  app.theme.accent_net,  &compact_net_val(snap),       app.theme.accent_net,         fs); }
        if vis.show_disk { compact_row(ui, &app.theme, "DISK", app.theme.accent_disk, &compact_disk_val(app, snap), compact_disk_color(app, snap),fs); }
        if vis.show_temp { compact_row(ui, &app.theme, "TEMP", app.theme.accent_temp, &compact_temp_val(snap),      compact_temp_color(app, snap),fs); }
    } else {
        ui.spacing_mut().item_spacing.y = 4.0;
        cpu_card(app, ui, snap);
        memory_card(app, ui, snap);
        if vis.show_fps  { fps_card(app, ui, fps); }
        if vis.show_gpu  { gpu_card(app, ui, snap); }
        if vis.show_net  { network_card(app, ui, snap); }
        if vis.show_disk { disk_card(app, ui, snap); }
        if vis.show_temp { temps_card(app, ui, snap); }
    }
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

    // Neon glow + solid big value — same style as compact mode
    let (num_y, anchor) = if unit.is_empty() {
        (mid_y, Align2::CENTER_CENTER)
    } else {
        (mid_y - 4.0, Align2::CENTER_BOTTOM)
    };
    let pos = egui::pos2(right_cx, num_y);
    let fid = FontId::new(28.0, FontFamily::Monospace);

    // Soft bloom — 4 cardinals at 1 px
    let halo = Color32::from_rgba_unmultiplied(r, g, b, 18);
    for (dx, dy) in [(-1.0f32, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        ui.painter().text(egui::pos2(pos.x + dx, pos.y + dy), anchor, big_text, fid.clone(), halo);
    }
    // Solid core
    ui.painter().text(pos, anchor, big_text, fid, text_color);

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

// ── Compact mode ─────────────────────────────────────────────────────────────

fn compact_row(ui: &mut Ui, theme: &crate::theme::Theme, label: &str, accent: Color32, val: &str, val_color: Color32, font_size: f32) {
    glow_card(ui, theme, accent, |ui| {
        let row_h = (font_size + 12.0).max(24.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
        let p  = ui.painter();
        let cy = rect.center().y;

        // Accent bar
        let bar_h = (font_size * 0.65).clamp(12.0, 26.0);
        p.rect_filled(
            egui::Rect::from_center_size(egui::pos2(rect.min.x + 1.5, cy), Vec2::new(3.0, bar_h)),
            Rounding::same(1.5),
            accent,
        );

        // Label (tag on the left)
        p.text(
            egui::pos2(rect.min.x + 11.0, cy),
            Align2::LEFT_CENTER,
            label,
            FontId::new(10.5, FontFamily::Monospace),
            accent,
        );

        // Value — right-aligned with thick stroke + glow
        let vp  = egui::pos2(rect.max.x - 4.0, cy);
        let fid = FontId::new(font_size, FontFamily::Monospace);
        let [r, g, b, _] = val_color.to_array();

        // Soft glow bloom — 4 cardinals at 1 px, just enough for the neon look
        let halo = Color32::from_rgba_unmultiplied(r, g, b, 18);
        for (dx, dy) in [(-1.0f32, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            p.text(egui::pos2(vp.x+dx, vp.y+dy), Align2::RIGHT_CENTER, val, fid.clone(), halo);
        }

        // Solid core
        p.text(vp, Align2::RIGHT_CENTER, val, fid, val_color);
    });
}

fn compact_cpu_val(_app: &MonitorApp, snap: &SystemSnapshot) -> String {
    format!("{:.1}%", snap.cpu.total_usage)
}
fn compact_cpu_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    app.theme.health_color(snap.cpu.total_usage, 60.0, 85.0)
}

fn compact_mem_val(_: &MonitorApp, snap: &SystemSnapshot) -> String {
    format!("{:.1}%", snap.memory.usage_percent())
}
fn compact_mem_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    app.theme.health_color(snap.memory.usage_percent(), 70.0, 90.0)
}

fn compact_fps_val(fps: &FpsSnapshot) -> String {
    if fps.active { format!("{:.0} fps", fps.fps) } else { "— fps".into() }
}
fn compact_fps_color(app: &MonitorApp, fps: &FpsSnapshot) -> Color32 {
    if !fps.active         { app.theme.text_dim }
    else if fps.fps >= 60.0 { app.theme.ok }
    else if fps.fps >= 30.0 { app.theme.warn }
    else                    { app.theme.crit }
}

fn compact_gpu_val(_: &MonitorApp, snap: &SystemSnapshot) -> String {
    if snap.gpu.available {
        snap.gpu.utilization_percent.map(|p| format!("{p:.0}%")).unwrap_or_else(|| "—".into())
    } else { "—".into() }
}
fn compact_gpu_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    match snap.gpu.utilization_percent.filter(|_| snap.gpu.available) {
        Some(p) => app.theme.health_color(p, 70.0, 90.0),
        None    => app.theme.text_dim,
    }
}

fn compact_net_val(snap: &SystemSnapshot) -> String {
    let (num, unit) = fmt_bps_parts(snap.network.total_rx_bps);
    format!("↓{num} {unit}")
}

fn compact_disk_val(_: &MonitorApp, snap: &SystemSnapshot) -> String {
    snap.disks.first().map(|d| format!("{:.0}%", d.usage_percent())).unwrap_or_else(|| "—".into())
}
fn compact_disk_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    snap.disks.first()
        .map(|d| app.theme.health_color(d.usage_percent(), 75.0, 90.0))
        .unwrap_or(app.theme.text_dim)
}

fn compact_temp_val(snap: &SystemSnapshot) -> String {
    snap.temps.cpu_celsius.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "—".into())
}
fn compact_temp_color(app: &MonitorApp, snap: &SystemSnapshot) -> Color32 {
    snap.temps.cpu_celsius
        .map(|t| app.theme.health_color(t, 70.0, 85.0))
        .unwrap_or(app.theme.text_dim)
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

        let left_w = ui.available_width();
        card_hdr_no_val(ui, "CPU", &snap.cpu.brand, accent, subtle);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_cpu, 100.0, accent, Some(CPU_END), SPEC_H, vu_color);
        zone_sep(ui, accent);
        let cpu_left = if left_w < NARROW {
            format!("{}c · {}M", snap.cpu.logical_cores, snap.cpu.frequency_mhz)
        } else {
            format!("{} cores  ·  {} MHz", snap.cpu.logical_cores, snap.cpu.frequency_mhz)
        };
        stat_line(ui, &cpu_left, &format!("{pct:.1}%"), dim, accent);
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

        let left_w = ui.available_width();
        card_hdr_no_val(ui, "MEM", &fmt_bytes(snap.memory.total_bytes), accent, subtle);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_mem, 100.0, accent, Some(MEM_END), SPEC_H, vu_color);
        zone_sep(ui, accent);
        let (mem_left, mem_right) = if left_w < NARROW {
            (fmt_bytes(snap.memory.used_bytes), fmt_bytes(snap.memory.available_bytes))
        } else {
            (format!("{} used", fmt_bytes(snap.memory.used_bytes)),
             format!("{} free", fmt_bytes(snap.memory.available_bytes)))
        };
        stat_line(ui, &mem_left, &mem_right, dim, accent);
        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &format!("{pct:.0}%"), "", val_color, dim, border);
    });
}

// ── FPS ───────────────────────────────────────────────────────────────────────

fn fps_card(app: &mut MonitorApp, ui: &mut Ui, fps: &FpsSnapshot) {
    let accent    = app.theme.accent_net;
    let subtle    = app.theme.text_subtle;
    let dim       = app.theme.text_dim;
    let border    = app.theme.card_border;

    let val_color = if !fps.active         { dim }
                   else if fps.fps >= 60.0 { app.theme.ok }
                   else if fps.fps >= 30.0 { app.theme.warn }
                   else                    { app.theme.crit };
    let big_num = if fps.active { format!("{:.0}", fps.fps) } else { "—".into() };
    let unit    = if fps.active { "fps" } else { "" };
    let ft      = if fps.active && fps.fps > 0.0 { format!("{:.1} ms", 1000.0 / fps.fps) } else { String::new() };

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));
        let left_w = ui.available_width();

        let subtitle  = if fps.active { truncate(&fps.window_title, 18) } else { "no game".into() };
        let sub_color = if fps.active { subtle } else { dim };
        card_hdr_no_val(ui, "FPS", &subtitle, accent, sub_color);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_fps, 120.0, accent, None, SPEC_H, fps_color);
        zone_sep(ui, accent);
        let (stat_left, stat_right, stat_color) = if fps.active {
            let chars = if left_w < NARROW { 10 } else { 26 };
            (truncate(&fps.window_title, chars), ft, accent)
        } else {
            ("waiting for game".into(), String::new(), dim)
        };
        stat_line(ui, &stat_left, &stat_right, dim, stat_color);

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
    let big_val   = if !snap.gpu.available {
        "—".into()
    } else {
        snap.gpu.utilization_percent
            .map(|p| format!("{p:.0}%"))
            .unwrap_or_else(|| "—".into())
    };
    let big_color = if snap.gpu.available && snap.gpu.utilization_percent.is_some() {
        app.theme.health_color(pct, 70.0, 90.0)
    } else { dim };

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        let subtitle  = if snap.gpu.available { snap.gpu.name.as_str() } else { "unavailable" };
        let sub_color = if snap.gpu.available { subtle } else { dim };
        card_hdr_no_val(ui, "GPU", subtitle, accent, sub_color);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_gpu, 100.0, accent, None, SPEC_H, vu_color);
        zone_sep(ui, accent);
        if snap.gpu.available {
            let vram_str = if snap.gpu.vram_total_bytes > 0 {
                format!("{} / {}", fmt_bytes(snap.gpu.vram_used_bytes), fmt_bytes(snap.gpu.vram_total_bytes))
            } else { String::new() };
            let temp_str = snap.gpu.temperature_celsius.map(|t| format!("{t:.0}°C")).unwrap_or_default();
            let right = [&vram_str, &temp_str].iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("  ·  ");
            stat_line(ui, "VRAM", &right, dim, accent);
        } else {
            stat_line(ui, "no GPU data", "", dim, dim);
        }

        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &big_val, "", big_color, dim, border);
    });
}

// ── Network ───────────────────────────────────────────────────────────────────

fn network_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_net;
    let subtle = app.theme.text_subtle;
    let dim    = app.theme.text_dim;
    let border = app.theme.card_border;

    let rx_max = app.hist_rx.as_vec().iter().cloned().fold(8_000.0f64, f64::max) as f32;
    let (rx_num, rx_unit) = fmt_bps_parts(snap.network.total_rx_bps);

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        card_hdr_no_val(ui, "NET", "↓ download", accent, subtle);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_rx, rx_max, accent, None, SPEC_H, vu_color);
        zone_sep(ui, accent);
        stat_line(ui, "↑ upload", &fmt_bps(snap.network.total_tx_bps), dim, accent);

        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &rx_num, rx_unit, accent, dim, border);
    });
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn disk_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_disk;
    let subtle = app.theme.text_subtle;
    let dim    = app.theme.text_dim;
    let border = app.theme.card_border;

    let (val_color, big_val, subtitle, stat_left, stat_right) =
        if let Some(disk) = snap.disks.first() {
            let pct       = disk.usage_percent();
            let val_color = app.theme.health_color(pct, 75.0, 90.0);
            let free      = disk.total_bytes.saturating_sub(disk.used_bytes);
            let stat_right = if snap.disks.len() > 1 {
                format!("+{} more", snap.disks.len() - 1)
            } else {
                format!("{} free", fmt_bytes(free))
            };
            (val_color, format!("{pct:.0}%"), disk.mount.clone(),
             format!("{} used", fmt_bytes(disk.used_bytes)), stat_right)
        } else {
            (dim, "—".into(), String::new(), "no disks found".into(), String::new())
        };

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        card_hdr_no_val(ui, "DISK", &subtitle, accent, subtle);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_disk, 100.0, accent, None, SPEC_H, vu_color);
        zone_sep(ui, accent);
        stat_line(ui, &stat_left, &stat_right, dim, accent);

        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &big_val, "", val_color, dim, border);
    });
}

// ── Temperatures ──────────────────────────────────────────────────────────────

fn temps_card(app: &mut MonitorApp, ui: &mut Ui, snap: &SystemSnapshot) {
    let accent = app.theme.accent_temp;
    let subtle = app.theme.text_subtle;
    let dim    = app.theme.text_dim;
    let border = app.theme.card_border;

    let has_data  = snap.temps.cpu_celsius.is_some() || snap.temps.gpu_celsius.is_some();
    let cpu_big   = snap.temps.cpu_celsius.map(|t| format!("{t:.0}")).unwrap_or_else(|| "—".into());
    let cpu_unit  = if snap.temps.cpu_celsius.is_some() { "°C" } else { "" };
    let cpu_color = snap.temps.cpu_celsius
        .map(|t| app.theme.health_color(t, 70.0, 85.0))
        .unwrap_or(dim);

    glow_card(ui, &app.theme, accent, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        const RIGHT_W: f32 = 65.0;
        let full_right = ui.max_rect().right();
        ui.set_max_width((ui.available_width() - RIGHT_W - 1.0).max(60.0));

        let subtitle  = if has_data { "CPU & GPU" } else { "no sensor data" };
        let sub_color = if has_data { subtle } else { dim };
        card_hdr_no_val(ui, "TEMP", subtitle, accent, sub_color);
        zone_sep(ui, accent);
        spectrum_bars(ui, &app.disp_temp_cpu, 100.0, accent, None, SPEC_H, vu_color);
        zone_sep(ui, accent);

        let gpu_str   = snap.temps.gpu_celsius.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "—".into());
        let gpu_color = snap.temps.gpu_celsius
            .map(|t| app.theme.health_color(t, 75.0, 90.0))
            .unwrap_or(dim);
        stat_line(ui, "GPU", &gpu_str, dim, gpu_color);

        draw_right_panel(ui, ui.min_rect(), full_right, RIGHT_W, &cpu_big, cpu_unit, cpu_color, dim, border);
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
