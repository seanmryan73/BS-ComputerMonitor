//! Colour palette and egui style configuration.

use egui::{Color32, Margin, Rounding, Stroke, Visuals};

pub struct Theme {
    // Surfaces
    pub bg: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub titlebar_bg: Color32,
    pub hover_bg: Color32,

    // Text
    pub text_primary: Color32,
    pub text_subtle: Color32,
    pub text_dim: Color32,

    // Metric accents
    pub accent_cpu: Color32,
    pub accent_mem: Color32,
    pub accent_gpu: Color32,
    pub accent_net: Color32,
    pub accent_disk: Color32,
    pub accent_temp: Color32,

    // Health
    pub ok: Color32,
    pub warn: Color32,
    pub crit: Color32,

    // Geometry
    pub card_rounding: Rounding,
    pub bar_rounding: Rounding,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // BagPipes Black — same base as BS-VChanger-Rust default theme
            bg: Color32::from_rgb(0x08, 0x08, 0x08),
            card_bg: Color32::from_rgb(0x13, 0x13, 0x13),
            card_border: Color32::from_rgb(0x22, 0x22, 0x22),
            titlebar_bg: Color32::from_rgb(0x05, 0x05, 0x05),
            hover_bg: Color32::from_rgb(0x1c, 0x1c, 0x1c),

            text_primary: Color32::from_rgb(0xf0, 0xf0, 0xf0),
            text_subtle: Color32::from_rgb(0x60, 0x60, 0x60),
            text_dim: Color32::from_rgb(0x30, 0x30, 0x30),

            accent_cpu: Color32::from_rgb(85, 222, 255),
            accent_mem: Color32::from_rgb(240, 160, 80),
            accent_gpu: Color32::from_rgb(192, 132, 252),
            accent_net: Color32::from_rgb(77, 232, 142),
            accent_disk: Color32::from_rgb(96, 165, 250),
            accent_temp: Color32::from_rgb(251, 146, 60),

            ok: Color32::from_rgb(77, 232, 142),
            warn: Color32::from_rgb(251, 191, 36),
            crit: Color32::from_rgb(248, 113, 113),

            card_rounding: Rounding::same(8.0),
            bar_rounding: Rounding::same(4.0),
        }
    }
}

impl Theme {
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        let mut visuals = Visuals::dark();

        visuals.panel_fill = self.bg;
        visuals.window_fill = self.bg;
        visuals.faint_bg_color = self.card_bg;
        visuals.extreme_bg_color = Color32::from_rgb(4, 8, 12);
        visuals.window_shadow = egui::Shadow::NONE;
        visuals.popup_shadow = egui::Shadow::NONE;

        visuals.widgets.noninteractive.bg_fill = self.card_bg;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_subtle);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.card_border);
        visuals.widgets.noninteractive.rounding = Rounding::same(4.0);

        visuals.widgets.inactive.bg_fill = self.card_bg;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_subtle);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.card_border);
        visuals.widgets.inactive.rounding = Rounding::same(4.0);

        visuals.widgets.hovered.bg_fill = self.hover_bg;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.hovered.rounding = Rounding::same(4.0);

        visuals.widgets.active.bg_fill = Color32::from_rgb(25, 55, 70);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent_cpu);

        visuals.override_text_color = Some(self.text_primary);

        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.window_margin = Margin::same(0.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);

        ctx.set_style(style);
    }

    pub fn health_color(&self, pct: f32, warn_at: f32, crit_at: f32) -> Color32 {
        if pct >= crit_at {
            self.crit
        } else if pct >= warn_at {
            self.warn
        } else {
            self.ok
        }
    }

    /// Dim version of a colour for empty bar track
    pub fn dim(color: Color32) -> Color32 {
        Color32::from_rgba_premultiplied(
            (color.r() as u16 * 25 / 100) as u8,
            (color.g() as u16 * 25 / 100) as u8,
            (color.b() as u16 * 25 / 100) as u8,
            200,
        )
    }
}
