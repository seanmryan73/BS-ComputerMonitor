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
            // Coral Storm — standard palette: teal-black grounds, neon coral + turquoise accents
            bg:          Color32::from_rgb(0x00, 0x12, 0x12), // #001212
            card_bg:     Color32::from_rgb(0x00, 0x1e, 0x1e), // #001e1e
            card_border: Color32::from_rgb(0x00, 0x38, 0x38), // #003838
            titlebar_bg: Color32::from_rgb(0x00, 0x0a, 0x0a), // slightly darker than bg
            hover_bg:    Color32::from_rgb(0x00, 0x2c, 0x2c), // #002c2c slider track

            text_primary: Color32::from_rgb(0xff, 0xf4, 0xee), // #fff4ee
            text_subtle:  Color32::from_rgb(0x22, 0x99, 0x88), // #229988
            text_dim:     Color32::from_rgb(0x00, 0x2c, 0x2c), // dark teal

            // Coral Storm metric accents — coral primary, turquoise secondary, variants in family
            accent_cpu:  Color32::from_rgb(0xff, 0x55, 0x33), // #ff5533 standard accent (coral)
            accent_mem:  Color32::from_rgb(0x00, 0xff, 0xdd), // #00ffdd standard accent_alt (turquoise)
            accent_gpu:  Color32::from_rgb(0xff, 0x88, 0x66), // lighter coral
            accent_net:  Color32::from_rgb(0x00, 0xdd, 0xbb), // muted turquoise
            accent_disk: Color32::from_rgb(0xff, 0x44, 0x22), // #ff4422 widget border coral
            accent_temp: Color32::from_rgb(0xff, 0x22, 0x00), // hot red-coral

            ok:   Color32::from_rgb(0x00, 0xdd, 0xbb), // teal — healthy
            warn: Color32::from_rgb(0xff, 0xaa, 0x22), // amber
            crit: Color32::from_rgb(0xff, 0x22, 0x11), // red-coral
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
        visuals.extreme_bg_color = Color32::from_rgb(3, 4, 8);
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

        visuals.widgets.active.bg_fill = Color32::from_rgb(12, 18, 28);
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
