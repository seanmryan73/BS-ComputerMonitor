//! Colour palette and egui style configuration.

use egui::{Color32, Margin, Rounding, Stroke, Visuals};

#[derive(Clone, Copy)]
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

}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Coral Storm — dark teal-black grounds, hot coral accent
            bg:          Color32::from_rgb(0x04, 0x0C, 0x09),
            card_bg:     Color32::from_rgb(0x0B, 0x18, 0x12),
            card_border: Color32::from_rgb(0xA0, 0x3C, 0x06),
            titlebar_bg: Color32::from_rgb(0x02, 0x07, 0x05),
            hover_bg:    Color32::from_rgb(0x15, 0x26, 0x1C),

            text_primary: Color32::from_rgb(0xE8, 0xDF, 0xD4),
            text_subtle:  Color32::from_rgb(0x58, 0x7A, 0x66),
            text_dim:     Color32::from_rgb(0x1C, 0x30, 0x24),

            // Coral Storm accents
            accent_cpu:  Color32::from_rgb(0xE8, 0x60, 0x0A), // coral orange — primary
            accent_mem:  Color32::from_rgb(0xE8, 0xA0, 0x1C), // amber gold
            accent_gpu:  Color32::from_rgb(0xB0, 0x5C, 0xE8), // amethyst
            accent_net:  Color32::from_rgb( 42, 200, 118),     // jade emerald
            accent_disk: Color32::from_rgb(0xE8, 0x70, 0x28),  // warm coral
            accent_temp: Color32::from_rgb(0xFF, 0x48, 0x08),  // hot orange-red

            ok:   Color32::from_rgb( 42, 200, 118), // emerald
            warn: Color32::from_rgb(238, 178,   8), // amber gold
            crit: Color32::from_rgb(225,  68,  68), // ruby

        }
    }
}

impl Theme {
    pub fn apply(&self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "JetBrainsMono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")),
        );
        fonts.font_data.insert(
            "CascadiaMono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/CascadiaMono.ttf")),
        );
        // JetBrains Mono as primary monospace (numbers/values), Cascadia as fallback.
        fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "JetBrainsMono".to_owned());
        // Proportional labels use CascadiaMono for a uniform mono aesthetic.
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "CascadiaMono".to_owned());
        ctx.set_fonts(fonts);

        let mut style = (*ctx.style()).clone();
        let mut visuals = Visuals::dark();

        visuals.panel_fill = self.bg;
        visuals.window_fill = self.bg;
        visuals.faint_bg_color = self.card_bg;
        visuals.extreme_bg_color = Color32::from_rgb(2, 5, 3);
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

        visuals.widgets.active.bg_fill = Color32::from_rgb(15, 28, 20);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent_cpu);

        visuals.override_text_color = Some(self.text_primary);

        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.window_margin = Margin::same(0.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);

        ctx.set_style(style);
    }

}
