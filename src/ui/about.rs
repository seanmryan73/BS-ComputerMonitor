//! Settings / Help / About — separate opaque OS window that pops out to the left of the main window.
//!
//! Three tabs, each a stack of collapsible sections. The Reset / Close row is pinned below the
//! scroll area so it never scrolls away. Every colour comes from the active `Theme`, so picking a
//! theme re-skins this window too.

use std::path::Path;
use std::sync::{Arc, Mutex};

use egui::{
    collapsing_header::CollapsingState, emath::Rot2, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Frame, Id,
    Layout, Margin, Rect, Response, RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2, ViewportBuilder, ViewportId,
};

use crate::{
    app::CardVisibility,
    theme::{contrast, Theme, ThemeId},
};

const CLOSE_ID: &str = "about_close_requested";
const TAB_ID: &str = "about_tab";
const RESET_CONFIRM_ID: &str = "about_reset_confirm";

/// Vault About-Screen-Standards § Per-app mottos.
const MOTTO: &str = "Because knowing your system is half the battle.";

/// Display version derived from Cargo.toml so the two can't drift.
fn version_string() -> String {
    format!("v{}", pad_version(env!("CARGO_PKG_VERSION")))
}

/// `YYYY.MMDD.N`: Cargo rejects a leading zero, so January–September store `2026.109.0`; pad the
/// middle field back to four digits (vault Author-Version-Standards § Software). Anything that
/// isn't three numeric fields is returned unchanged. Replaced the `YYYY.MM.DD[-a]` helper
/// 2026-10-20, which would have shown `2026.1020.0` as `2026.1020.00`.
fn pad_version(cargo: &str) -> String {
    let parts: Vec<&str> = cargo.split('.').collect();
    match parts.as_slice() {
        [y, md, n] if [y, md, n].iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())) => {
            format!("{y}.{md:0>4}.{n}")
        }
        _ => cargo.to_string(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    Help,
    About,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Settings, Tab::Help, Tab::About];

    fn label(self) -> &'static str {
        match self {
            Tab::Settings => "SETTINGS",
            Tab::Help => "HELP",
            Tab::About => "ABOUT",
        }
    }
}

/// The active theme plus the two secondary text levels this window uses. They are blended from
/// `text_primary` toward `bg` rather than taken from `text_subtle`, which is near-invisible on
/// some themes; `secondary_text_stays_readable_on_every_theme` holds them to 4.5:1.
#[derive(Clone, Copy)]
struct Look {
    t: Theme,
    /// Readable secondary text: labels, descriptions.
    body: Color32,
    /// Tertiary text: hints, disabled labels.
    hint: Color32,
}

impl Look {
    fn new(t: Theme) -> Self {
        Self {
            t,
            body: readable_mix(&t, 0.25),
            hint: readable_mix(&t, 0.40),
        }
    }
}

/// `text_primary` blended `frac` of the way toward `bg`, backed off in 5% steps until it holds
/// 4.5:1 on both `bg` and `card_bg`. Only themes whose card is close to the text in lightness
/// (Coral Storm, Jizo) need the back-off; the rest keep the full blend.
fn readable_mix(t: &Theme, frac: f32) -> Color32 {
    let mut f = frac;
    loop {
        let c = mix(t.text_primary, t.bg, f);
        if f <= 0.0 || (contrast(c, t.bg) >= 4.5 && contrast(c, t.card_bg) >= 4.5) {
            return c;
        }
        f -= 0.05;
    }
}

pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    open: &mut bool,
    card_vis: Arc<Mutex<CardVisibility>>,
    initial_pos: Option<egui::Pos2>,
    is_elevated: bool,
    gpu_names: Vec<String>,
) {
    let close_id = Id::new(CLOSE_ID);
    if ctx.data(|d| d.get_temp::<bool>(close_id).unwrap_or(false)) {
        *open = false;
        ctx.data_mut(|d| d.remove::<bool>(close_id));
    }

    if !*open {
        return;
    }

    // `initial_pos` is only set on the frame the window opens: a fresh open never starts
    // half-way through a reset confirmation.
    if initial_pos.is_some() {
        ctx.data_mut(|d| d.remove::<bool>(Id::new(RESET_CONFIRM_ID)));
    }

    let look = Look::new(*theme);

    let mut builder = ViewportBuilder::default()
        .with_title("BS Monitor — Settings")
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
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.data_mut(|d| d.insert_temp(Id::new(CLOSE_ID), true));
            }

            let t = look.t;
            let tab_id = Id::new(TAB_ID);
            let mut tab = ctx.data(|d| d.get_temp::<Tab>(tab_id)).unwrap_or(Tab::Settings);

            egui::TopBottomPanel::top("about_header")
                .show_separator_line(false)
                .frame(Frame::none().fill(t.bg).inner_margin(Margin {
                    left: 18.0,
                    right: 18.0,
                    top: 16.0,
                    bottom: 0.0,
                }))
                .show(ctx, |ui| {
                    header(ui, &look, is_elevated);
                    ui.add_space(12.0);
                    tab_strip(ui, &look, &mut tab);
                });
            ctx.data_mut(|d| d.insert_temp(tab_id, tab));

            egui::TopBottomPanel::bottom("about_buttons")
                .show_separator_line(false)
                .frame(Frame::none().fill(t.bg).inner_margin(Margin::symmetric(18.0, 12.0)))
                .show(ctx, |ui| button_row(ui, &look, &card_vis));

            egui::CentralPanel::default()
                .frame(Frame::none().fill(t.bg).inner_margin(Margin::symmetric(18.0, 0.0)))
                .show(ctx, |ui| {
                    // Scoped to this panel — ctx.set_visuals would leak into the
                    // main window's shared Context and linger after close.
                    ui.style_mut().visuals.selection.bg_fill = t.accent_cpu;
                    egui::ScrollArea::vertical()
                        .id_salt(tab.label())
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            match tab {
                                Tab::Settings => settings_tab(ui, &look, &card_vis, &gpu_names),
                                Tab::Help => help_tab(ui, &look, is_elevated),
                                Tab::About => about_tab(ui, &look),
                            }
                            ui.add_space(12.0);
                        });
                });
        },
    );
}

// ── Frame: header, tabs, buttons ──────────────────────────────────────────────

fn header(ui: &mut Ui, look: &Look, is_elevated: bool) {
    let t = look.t;
    ui.horizontal(|ui| {
        ui.label(mono("BS COMPUTER MONITOR", t.accent_cpu, 17.0).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (glyph, word, color, tip) = if is_elevated {
                ("●", "ADMIN", t.ok, "Running as Administrator — all sensors active")
            } else {
                (
                    "▲",
                    "LIMITED",
                    t.warn,
                    "Not running as Administrator — see HELP › PERMISSIONS",
                )
            };
            // Right-to-left: the word is added first so the glyph sits before it.
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(mono(word, color, 10.5)).on_hover_text(tip);
            ui.label(RichText::new(glyph).color(color).size(11.0))
                .on_hover_text(tip);
        });
    });
    ui.label(mono(
        format!("{}  ·  System Resource Monitor", version_string()),
        look.body,
        11.0,
    ));
}

fn tab_strip(ui: &mut Ui, look: &Look, tab: &mut Tab) {
    let t = look.t;
    let height = 28.0;
    let (strip, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let tab_w = strip.width() / Tab::ALL.len() as f32;

    let mut selected_rect = None;
    for (i, &this) in Tab::ALL.iter().enumerate() {
        let rect = Rect::from_min_size(strip.min + Vec2::new(i as f32 * tab_w, 0.0), Vec2::new(tab_w, height));
        let resp = ui
            .interact(rect, ui.id().with(("about_tab", i)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            *tab = this;
        }

        let selected = *tab == this;
        let top_only = Rounding {
            nw: 3.0,
            ne: 3.0,
            sw: 0.0,
            se: 0.0,
        };
        if selected {
            ui.painter().rect_filled(rect, top_only, tint(t.accent_cpu, 26));
            selected_rect = Some(rect);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, top_only, tint(t.accent_cpu, 12));
        }
        let color = if selected {
            t.accent_cpu
        } else if resp.hovered() {
            t.text_primary
        } else {
            look.body
        };
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            this.label(),
            FontId::new(11.5, FontFamily::Monospace),
            color,
        );
    }

    // Baseline under all tabs, then the selected tab's underline on top of it.
    ui.painter().line_segment(
        [strip.left_bottom(), strip.right_bottom()],
        Stroke::new(1.0, tint(t.card_border, 120)),
    );
    if let Some(r) = selected_rect {
        ui.painter().rect_filled(
            Rect::from_min_max(r.left_bottom() - Vec2::new(0.0, 2.0), r.right_bottom()),
            Rounding::ZERO,
            t.accent_cpu,
        );
    }
}

fn button_row(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>) {
    let t = look.t;
    let confirm_id = Id::new(RESET_CONFIRM_ID);
    let confirming = ui.ctx().data(|d| d.get_temp::<bool>(confirm_id)).unwrap_or(false);
    let gap = 10.0;

    divider(ui, t.accent_cpu, 0.35);
    ui.add_space(12.0);

    if confirming {
        ui.label(mono("Reset every setting to its default?", t.warn, 11.0).strong());
        ui.add_space(2.0);
        ui.label(mono("Theme, cards, sizes, opacity, network and ping.", look.hint, 10.5));
        ui.add_space(8.0);
    }

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let btn_w = (ui.available_width() - gap) / 2.0;
        let size = Vec2::new(btn_w, 30.0);

        if confirming {
            let confirm = egui::Button::new(mono("CONFIRM RESET", t.crit, 11.0).strong())
                .fill(tint(t.crit, 36))
                .stroke(Stroke::new(1.0, t.crit))
                .min_size(size);
            if ui.add(confirm).clicked() {
                // Build the fresh value and drop the lock before saving: save() does file I/O.
                let fresh = CardVisibility::default();
                match card_vis.lock() {
                    Ok(mut vis) => {
                        *vis = fresh.clone();
                        drop(vis);
                        fresh.save();
                    }
                    Err(e) => log::error!("card visibility lock poisoned, reset skipped: {e}"),
                }
                ui.ctx().data_mut(|d| d.remove::<bool>(confirm_id));
            }

            let cancel = egui::Button::new(mono("CANCEL", t.text_primary, 11.0))
                .fill(t.card_bg)
                .stroke(Stroke::new(1.0, tint(t.accent_cpu, 200)))
                .min_size(size);
            if ui.add(cancel).clicked() {
                ui.ctx().data_mut(|d| d.remove::<bool>(confirm_id));
            }
        } else {
            let reset = egui::Button::new(mono("RESET TO DEFAULTS", t.crit, 11.0).strong())
                .fill(tint(t.crit, 20))
                .stroke(Stroke::new(1.0, t.crit))
                .min_size(size);
            if ui
                .add(reset)
                .on_hover_text("Restores every saved setting — asks first")
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(confirm_id, true));
            }

            let close = egui::Button::new(mono("CLOSE", t.text_primary, 11.0))
                .fill(t.card_bg)
                .stroke(Stroke::new(1.0, tint(t.accent_cpu, 200)))
                .min_size(size);
            if ui.add(close).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(Id::new(CLOSE_ID), true));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    });
}

// ── SETTINGS tab ──────────────────────────────────────────────────────────────

fn settings_tab(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>, gpu_names: &[String]) {
    fold(ui, look, "theme", "THEME", true, |ui| theme_grid(ui, look, card_vis));
    fold(ui, look, "cards", "VISIBLE CARDS", true, |ui| {
        visible_cards(ui, look, card_vis)
    });
    fold(ui, look, "appearance", "APPEARANCE", false, |ui| {
        appearance(ui, look, card_vis)
    });
    fold(ui, look, "gpu", "GPU ADAPTER", false, |ui| {
        gpu_adapter(ui, look, card_vis, gpu_names)
    });
    fold(ui, look, "network", "NETWORK", false, |ui| network(ui, look, card_vis));
}

fn theme_grid(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>) {
    let Ok(mut vis) = card_vis.lock() else { return };
    let gap = 8.0;
    let cell_w = (ui.available_width() - gap) / 2.0;
    for pair in ThemeId::ALL.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &id in pair {
                if swatch(ui, look, id, vis.theme_id == id, cell_w).clicked() && vis.theme_id != id {
                    vis.theme_id = id;
                    vis.save();
                }
            }
        });
    }
}

/// One theme-picker cell. The chip shows that theme's own surfaces (`bg` | `hover_bg`, framed in
/// `card_border`): the dark themes share the same metric accents, so an accent chip would look the
/// same for most of them.
fn swatch(ui: &mut Ui, look: &Look, id: ThemeId, selected: bool, width: f32) -> Response {
    let t = look.t;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hovered = resp.hovered();

    let (fill, outline) = if selected {
        (tint(t.accent_cpu, 28), t.accent_cpu)
    } else if hovered {
        (tint(t.accent_cpu, 14), tint(t.accent_cpu, 160))
    } else {
        (Color32::TRANSPARENT, tint(t.card_border, 80))
    };
    let p = ui.painter();
    p.rect(rect, Rounding::same(3.0), fill, Stroke::new(1.0, outline));

    let preview = Theme::from_id(id);
    let chip = Rect::from_min_size(rect.left_center() + Vec2::new(7.0, -8.0), Vec2::splat(16.0));
    p.rect_filled(chip, Rounding::same(2.0), preview.hover_bg);
    p.rect_filled(
        Rect::from_min_max(chip.min, chip.center_bottom()),
        Rounding {
            nw: 2.0,
            sw: 2.0,
            ne: 0.0,
            se: 0.0,
        },
        preview.bg,
    );
    p.rect_stroke(chip, Rounding::same(2.0), Stroke::new(1.0, preview.card_border));

    let color = if selected || hovered { t.text_primary } else { look.body };
    p.text(
        chip.right_center() + Vec2::new(8.0, 0.0),
        Align2::LEFT_CENTER,
        id.label(),
        FontId::new(11.5, FontFamily::Monospace),
        color,
    );
    resp
}

fn visible_cards(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>) {
    let mut always = true;
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Checkbox::new(&mut always, mono("CPU", look.hint, 12.0)));
        ui.add_enabled(false, egui::Checkbox::new(&mut always, mono("MEM", look.hint, 12.0)));
        ui.label(mono("always shown", look.hint, 10.5));
    });
    ui.add_space(4.0);

    let Ok(mut vis) = card_vis.lock() else { return };
    let mut changed = false;
    egui::Grid::new("about_cards")
        .num_columns(3)
        .spacing([18.0, 6.0])
        .show(ui, |ui| {
            changed |= toggle(ui, look, &mut vis.show_fps, "FPS");
            changed |= toggle(ui, look, &mut vis.show_gpu, "GPU");
            changed |= toggle(ui, look, &mut vis.show_net, "NET");
            ui.end_row();
            changed |= toggle(ui, look, &mut vis.show_disk, "DISK");
            changed |= toggle(ui, look, &mut vis.show_temp, "TEMP");
            changed |= toggle(ui, look, &mut vis.show_ping, "PING");
            ui.end_row();
            changed |= toggle(ui, look, &mut vis.show_top_cpu, "TOP CPU");
            changed |= toggle(ui, look, &mut vis.show_top_mem, "TOP MEM");
            ui.end_row();
        });
    if changed {
        vis.save();
    }
}

fn appearance(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>) {
    let Ok(mut vis) = card_vis.lock() else { return };
    ui.spacing_mut().slider_width = (ui.available_width() - 70.0).max(80.0);

    ui.label(mono("VALUE SIZE", look.body, 11.0));
    ui.add_space(2.0);
    let slider = egui::Slider::new(&mut vis.compact_font_size, 11.0_f32..=60.0_f32)
        .custom_formatter(|v, _| format!("{:.0} pt", v))
        .custom_parser(|s| s.trim_end_matches("pt").trim().parse().ok());
    // Save on release, not per drag-frame — avoids hammering config.json.
    let resp = ui.add(slider);
    let mut save = resp.drag_stopped() || resp.lost_focus();

    ui.add_space(8.0);
    ui.label(mono("WINDOW OPACITY", look.body, 11.0));
    ui.add_space(2.0);
    let resp = ui.add(
        egui::Slider::new(&mut vis.opacity, 0.15_f32..=1.0_f32)
            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
            .custom_parser(|s| s.trim_end_matches('%').parse().ok().map(|v: f64| v / 100.0)),
    );
    save |= resp.drag_stopped() || resp.lost_focus();

    if save {
        vis.save();
    }
}

fn gpu_adapter(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>, gpu_names: &[String]) {
    let t = look.t;
    match gpu_names {
        [] => {
            ui.label(mono("Detecting…", look.hint, 11.0));
        }
        [only] => {
            ui.label(mono(only.as_str(), t.text_primary, 11.5));
            ui.label(mono("Only one GPU detected", look.hint, 10.5));
        }
        _ => {
            let Ok(mut vis) = card_vis.lock() else { return };
            let clamped = vis.selected_gpu_index.min(gpu_names.len().saturating_sub(1));
            let selected_name = gpu_names.get(clamped).cloned().unwrap_or_default();
            let prev = vis.selected_gpu_index;
            egui::ComboBox::from_id_salt("gpu_select")
                .selected_text(mono(selected_name, t.text_primary, 11.0))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (i, name) in gpu_names.iter().enumerate() {
                        ui.selectable_value(
                            &mut vis.selected_gpu_index,
                            i,
                            RichText::new(name).monospace().size(11.0),
                        );
                    }
                });
            if vis.selected_gpu_index != prev {
                vis.save();
            }
        }
    }
}

fn network(ui: &mut Ui, look: &Look, card_vis: &Mutex<CardVisibility>) {
    let t = look.t;
    let Ok(mut vis) = card_vis.lock() else { return };

    ui.label(mono("BANDWIDTH CAP", look.body, 11.0));
    ui.label(mono(
        "Sets 100% on the NET fill bar and health colours",
        look.hint,
        10.5,
    ));
    ui.add_space(4.0);

    const PRESETS: &[(f32, &str)] = &[
        (10.0, "10 Mbps"),
        (25.0, "25 Mbps"),
        (50.0, "50 Mbps"),
        (100.0, "100 Mbps  —  Fast Ethernet"),
        (250.0, "250 Mbps"),
        (500.0, "500 Mbps"),
        (1000.0, "1 Gbps  —  Gigabit"),
        (2500.0, "2.5 Gbps"),
        (10000.0, "10 Gbps"),
    ];
    let cur_label = PRESETS
        .iter()
        .find(|&&(v, _)| (v - vis.net_cap_mbps).abs() < 0.5)
        .map(|&(_, l)| l)
        .unwrap_or("Custom");
    let prev = vis.net_cap_mbps;
    egui::ComboBox::from_id_salt("net_cap_select")
        .selected_text(mono(cur_label, t.text_primary, 11.0))
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for &(val, label) in PRESETS {
                ui.selectable_value(&mut vis.net_cap_mbps, val, RichText::new(label).monospace().size(11.0));
            }
        });
    if (vis.net_cap_mbps - prev).abs() > 0.1 {
        vis.save();
    }

    ui.add_space(10.0);
    ui.label(mono("PING TARGET", look.body, 11.0));
    ui.label(mono("Hostname or IP to ping (e.g. 8.8.8.8, router)", look.hint, 10.5));
    ui.add_space(4.0);
    let prev = vis.ping_target.clone();
    let resp = ui.add(
        egui::TextEdit::singleline(&mut vis.ping_target)
            .font(egui::TextStyle::Monospace)
            .desired_width(ui.available_width()),
    );
    if resp.lost_focus() && vis.ping_target != prev {
        vis.save();
    }
}

// ── HELP tab ──────────────────────────────────────────────────────────────────

fn help_tab(ui: &mut Ui, look: &Look, is_elevated: bool) {
    fold(ui, look, "permissions", "PERMISSIONS", true, |ui| {
        permissions(ui, look, is_elevated)
    });
    fold(ui, look, "passthrough", "GAME OVERLAY / PASSTHROUGH", true, |ui| {
        passthrough(ui, look)
    });
    fold(ui, look, "files", "FILES", false, |ui| files(ui, look));
}

fn permissions(ui: &mut Ui, look: &Look, is_elevated: bool) {
    let t = look.t;
    if is_elevated {
        ui.horizontal(|ui| {
            ui.label(RichText::new("●").color(t.ok).size(13.0));
            ui.label(mono("Running as Administrator — all sensors active", t.ok, 11.5));
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("▲").color(t.warn).size(14.0));
        ui.label(mono("Not running as Administrator", t.warn, 12.0).strong());
    });
    ui.add_space(4.0);
    ui.label(mono("These need admin rights:", look.body, 11.0));
    ui.add_space(2.0);
    ui.label(mono(
        "  · GPU utilization %\n  · CPU / GPU temperatures",
        t.text_primary,
        11.0,
    ));
    ui.add_space(8.0);
    ui.add(
        egui::Label::new(mono(
            "The app asks for admin rights when it starts. If you said No to the prompt, \
             close it and start it again — or right-click the .exe → Run as administrator.",
            look.hint,
            10.5,
        ))
        .wrap(),
    );
}

fn passthrough(ui: &mut Ui, look: &Look) {
    let t = look.t;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
        let c = rect.center();
        let r = 6.0_f32;
        let gap = 2.2;
        let stroke = Stroke::new(1.4, t.accent_cpu);
        let p = ui.painter();
        p.circle_stroke(c, r, stroke);
        p.line_segment(
            [c + egui::vec2(0.0, -(r + gap)), c + egui::vec2(0.0, -(r + gap + 4.0))],
            stroke,
        );
        p.line_segment(
            [c + egui::vec2(0.0, r + gap), c + egui::vec2(0.0, r + gap + 4.0)],
            stroke,
        );
        p.line_segment(
            [c + egui::vec2(-(r + gap), 0.0), c + egui::vec2(-(r + gap + 4.0), 0.0)],
            stroke,
        );
        p.line_segment(
            [c + egui::vec2(r + gap, 0.0), c + egui::vec2(r + gap + 4.0, 0.0)],
            stroke,
        );
        p.circle_filled(c, 1.4, t.accent_cpu);

        ui.add_space(4.0);
        ui.label(mono(
            "Click this button (top-left of title bar)\nto toggle passthrough on or off.",
            look.body,
            11.0,
        ));
    });

    ui.add_space(6.0);
    ui.add(
        egui::Label::new(mono(
            "Mouse clicks pass through to the game/app behind the window so you can play normally \
             while stats float on top.",
            look.hint,
            10.5,
        ))
        .wrap(),
    );
    ui.add_space(8.0);
    ui.label(mono("TO INTERACT WHILE PASSTHROUGH IS ON", look.body, 11.0).strong());
    ui.add_space(3.0);
    ui.add(
        egui::Label::new(mono(
            "Hold Ctrl — clicks go to this app instead of passing through. Use it to drag, resize, \
             open settings, or click the crosshair to turn passthrough off.",
            look.hint,
            10.5,
        ))
        .wrap(),
    );
    ui.add_space(4.0);
    ui.label(mono("Passthrough and pin-on-top reset to OFF on restart.", look.hint, 10.5).italics());
}

fn files(ui: &mut Ui, look: &Look) {
    let config = crate::app::config_path();
    path_row(ui, look, "SETTINGS", config.as_deref(), config.as_deref().and_then(Path::parent));
    ui.add_space(8.0);
    let logs = crate::logging::log_dir();
    path_row(ui, look, "LOGS", logs.as_deref(), logs.as_deref());
}

/// A labelled path with an OPEN button for its folder. The button is disabled until the folder
/// exists (nothing saved yet, or logging fell back to stderr).
fn path_row(ui: &mut Ui, look: &Look, label: &str, path: Option<&Path>, folder: Option<&Path>) {
    let t = look.t;
    ui.horizontal(|ui| {
        ui.label(mono(label, look.body, 11.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(dir) = folder {
                let open = egui::Button::new(mono("OPEN", t.text_primary, 10.5)).small();
                if ui
                    .add_enabled(dir.is_dir(), open)
                    .on_hover_text("Open in Explorer")
                    .clicked()
                {
                    open_folder(dir);
                }
            }
        });
    });
    let text = path.map_or_else(
        || "unavailable — LOCALAPPDATA is not set".to_owned(),
        |p| p.display().to_string(),
    );
    ui.add(egui::Label::new(mono(text, t.text_primary, 10.5)).wrap());
}

fn open_folder(dir: &Path) {
    // Explorer hands the window to the running shell and its exit code means nothing, so the
    // child is not waited on.
    if let Err(e) = std::process::Command::new("explorer").arg(dir).spawn() {
        log::warn!("could not open {} in Explorer: {e}", dir.display());
    }
}

// ── ABOUT tab ─────────────────────────────────────────────────────────────────

fn about_tab(ui: &mut Ui, look: &Look) {
    fold(ui, look, "details", "DETAILS", true, |ui| {
        row(ui, look, "AUTHOR", "Sean Ryan");
        row(ui, look, "CONTACT", "seanmryan@gmail.com");
        row(ui, look, "COMPANY", "BagPipes");
        ui.add_space(4.0);
        row(ui, look, "RUNTIME", "Rust · egui 0.29 · eframe 0.29");
        row(ui, look, "PLATFORM", "Windows · x86_64");
        ui.add_space(4.0);
        row_link(ui, look, "DONATE", "Support on Ko-fi", "https://ko-fi.com/bagofpipes");
    });
    fold(ui, look, "who", "WHO MADE THIS?", true, |ui| who_made_this(ui, look));
}

fn who_made_this(ui: &mut Ui, look: &Look) {
    let t = look.t;
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        ui.label(mono("BagPipes", t.accent_cpu, 16.0).strong());
        ui.add_space(3.0);
        ui.label(mono(format!("\"{MOTTO}\""), look.hint, 10.5).italics());
        ui.add_space(8.0);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(mono("P.S.", t.warn, 11.0).strong());
        ui.add_space(4.0);
        ui.label(mono(
            "The \"BS\" stands for BagPipes Software. \
             We know what you were thinking. We respect it.",
            look.hint,
            10.5,
        ));
    });
}

// ── Building blocks ───────────────────────────────────────────────────────────

/// A collapsible section: a clickable header strip (tint, accent bar, turning chevron, title)
/// over a card holding the body. Open state lives in egui memory for the session.
fn fold(ui: &mut Ui, look: &Look, key: &str, title: &str, default_open: bool, add_body: impl FnOnce(&mut Ui)) {
    let t = look.t;
    let id = ui.make_persistent_id(("about_fold", key));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);

    ui.add_space(10.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    if resp.clicked() {
        state.toggle(ui);
    }
    let openness = state.openness(ui.ctx());

    let p = ui.painter();
    p.rect_filled(
        rect,
        Rounding::same(3.0),
        tint(t.accent_cpu, if resp.hovered() { 38 } else { 22 }),
    );
    p.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
        Rounding::same(2.0),
        t.accent_cpu,
    );
    // Chevron: points right when closed and turns to point down as the section opens.
    let c = rect.left_center() + Vec2::new(16.0, 0.0);
    let rot = Rot2::from_angle(egui::lerp(-std::f32::consts::FRAC_PI_2..=0.0, openness));
    let points = [Vec2::new(-4.0, -2.5), Vec2::new(4.0, -2.5), Vec2::new(0.0, 3.0)]
        .map(|v| c + rot * v)
        .to_vec();
    p.add(Shape::convex_polygon(points, t.accent_cpu, Stroke::NONE));
    p.text(
        rect.left_center() + Vec2::new(28.0, 0.0),
        Align2::LEFT_CENTER,
        title,
        FontId::new(11.5, FontFamily::Monospace),
        t.accent_cpu,
    );

    state.show_body_unindented(ui, |ui| {
        ui.add_space(6.0);
        Frame::none()
            .fill(t.card_bg)
            .stroke(Stroke::new(1.0, tint(t.card_border, 110)))
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add_body(ui);
            });
    });
}

fn toggle(ui: &mut Ui, look: &Look, value: &mut bool, label: &str) -> bool {
    let color = if *value { look.t.text_primary } else { look.body };
    ui.checkbox(value, mono(label, color, 12.0)).changed()
}

fn row(ui: &mut Ui, look: &Look, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(mono(format!("{:<10}", label), look.body, 11.5));
        ui.label(mono(value, look.t.text_primary, 11.5));
    });
}

fn row_link(ui: &mut Ui, look: &Look, label: &str, link_text: &str, url: &str) {
    ui.horizontal(|ui| {
        ui.label(mono(format!("{:<10}", label), look.body, 11.5));
        ui.hyperlink_to(mono(link_text, look.t.accent_cpu, 11.5), url);
    });
}

fn divider(ui: &mut Ui, color: Color32, alpha_frac: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, tint(color, (255.0 * alpha_frac) as u8)),
    );
}

fn mono(text: impl Into<String>, color: Color32, size: f32) -> RichText {
    RichText::new(text).color(color).monospace().size(size)
}

/// The colour at a fixed alpha (a row or badge tint).
fn tint(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Straight per-channel blend from `a` (t = 0) to `b` (t = 1).
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let ch = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

#[cfg(test)]
mod version_tests {
    use super::pad_version;

    #[test]
    fn the_middle_field_pads_to_four_digits_and_anything_else_is_left_alone() {
        assert_eq!(pad_version("2026.109.0"), "2026.0109.0");
        assert_eq!(pad_version("2026.1009.1"), "2026.1009.1");
        assert_eq!(pad_version("2026.1020.0"), "2026.1020.0");
        assert_eq!(pad_version("2026.10.9-a"), "2026.10.9-a");
        assert_eq!(pad_version("dev"), "dev");
    }
}

#[cfg(test)]
mod look_tests {
    use super::Look;
    use crate::theme::{contrast, Theme, ThemeId};

    /// The window paints body and hint text on both `bg` and `card_bg`; both levels must clear
    /// WCAG 1.4.3 (4.5:1) on both, for every theme, Terminal included.
    #[test]
    fn secondary_text_stays_readable_on_every_theme() {
        let mut failures = Vec::new();
        for &id in ThemeId::ALL {
            let look = Look::new(Theme::from_id(id));
            for (level, fg) in [("body", look.body), ("hint", look.hint)] {
                for (surface, bg) in [("bg", look.t.bg), ("card_bg", look.t.card_bg)] {
                    let ratio = contrast(fg, bg);
                    if ratio < 4.5 {
                        failures.push(format!("{}: {level} on {surface} = {ratio:.2}", id.label()));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "contrast failures:\n{}", failures.join("\n"));
    }
}
