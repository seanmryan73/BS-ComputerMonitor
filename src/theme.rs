//! Colour palette and egui style configuration.

use egui::{Color32, Margin, Rounding, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ThemeId {
    CoralStorm,
    CandyPop,
    GlitchMode,
    ColdSteel,
    #[default]
    Lucky,
}

impl ThemeId {
    pub fn label(self) -> &'static str {
        match self {
            ThemeId::CoralStorm => "Coral Storm",
            ThemeId::CandyPop   => "Candy Pop",
            ThemeId::GlitchMode => "Glitch Mode",
            ThemeId::ColdSteel  => "Cold Steel",
            ThemeId::Lucky      => "Lucky",
        }
    }

    pub const ALL: &'static [ThemeId] = &[
        ThemeId::CoralStorm,
        ThemeId::CandyPop,
        ThemeId::GlitchMode,
        ThemeId::ColdSteel,
        ThemeId::Lucky,
    ];
}

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

// Shared accent colours — fixed across all themes.
const ACCENT_CPU:  Color32 = Color32::from_rgb(0xff, 0x55, 0x33);
const ACCENT_MEM:  Color32 = Color32::from_rgb(0x00, 0xff, 0xdd);
const ACCENT_GPU:  Color32 = Color32::from_rgb(0xff, 0x88, 0x66);
const ACCENT_NET:  Color32 = Color32::from_rgb(0x00, 0xdd, 0xbb);
const ACCENT_DISK: Color32 = Color32::from_rgb(0xff, 0x44, 0x22);
const ACCENT_TEMP: Color32 = Color32::from_rgb(0xff, 0x22, 0x00);
const OK:          Color32 = Color32::from_rgb(0x00, 0xdd, 0xbb);
const WARN:        Color32 = Color32::from_rgb(0xff, 0xaa, 0x22);
const CRIT:        Color32 = Color32::from_rgb(0xff, 0x22, 0x11);

impl Default for Theme {
    fn default() -> Self { Self::from_id(ThemeId::Lucky) }
}

impl Theme {
    pub fn from_id(id: ThemeId) -> Self {
        match id {
            ThemeId::CoralStorm => Self {
                bg:           Color32::from_rgb(0x00, 0x12, 0x12),
                card_bg:      Color32::from_rgb(0x00, 0x1e, 0x1e),
                card_border:  Color32::from_rgb(0x00, 0x38, 0x38),
                titlebar_bg:  Color32::from_rgb(0x00, 0x0a, 0x0a),
                hover_bg:     Color32::from_rgb(0x00, 0x2c, 0x2c),
                text_primary: Color32::from_rgb(0xff, 0xf4, 0xee),
                text_subtle:  Color32::from_rgb(0x22, 0x99, 0x88),
                text_dim:     Color32::from_rgb(0x00, 0x2c, 0x2c),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::CandyPop => Self {
                bg:           Color32::from_rgb(0x10, 0x00, 0x08),
                card_bg:      Color32::from_rgb(0x1e, 0x00, 0x12),
                card_border:  Color32::from_rgb(0x30, 0x00, 0x1e),
                titlebar_bg:  Color32::from_rgb(0x08, 0x00, 0x04),
                hover_bg:     Color32::from_rgb(0x28, 0x00, 0x18),
                text_primary: Color32::from_rgb(0xff, 0xdd, 0xee),
                text_subtle:  Color32::from_rgb(0xaa, 0x00, 0x55),
                text_dim:     Color32::from_rgb(0x1e, 0x00, 0x12),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::GlitchMode => Self {
                bg:           Color32::from_rgb(0x00, 0x03, 0x00),
                card_bg:      Color32::from_rgb(0x00, 0x08, 0x02),
                card_border:  Color32::from_rgb(0x00, 0x1c, 0x06),
                titlebar_bg:  Color32::from_rgb(0x00, 0x01, 0x00),
                hover_bg:     Color32::from_rgb(0x00, 0x10, 0x00),
                text_primary: Color32::from_rgb(0xcc, 0xff, 0xdd),
                text_subtle:  Color32::from_rgb(0x00, 0x77, 0x33),
                text_dim:     Color32::from_rgb(0x00, 0x08, 0x02),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::ColdSteel => Self {
                bg:           Color32::from_rgb(0x08, 0x08, 0x08),
                card_bg:      Color32::from_rgb(0x10, 0x10, 0x10),
                card_border:  Color32::from_rgb(0x28, 0x28, 0x28),
                titlebar_bg:  Color32::from_rgb(0x03, 0x03, 0x03),
                hover_bg:     Color32::from_rgb(0x1a, 0x1a, 0x1a),
                text_primary: Color32::from_rgb(0xec, 0xec, 0xec),
                text_subtle:  Color32::from_rgb(0x66, 0x66, 0x66),
                text_dim:     Color32::from_rgb(0x10, 0x10, 0x10),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::Lucky => Self {
                bg:           Color32::from_rgb(0x14, 0x00, 0x2d),
                card_bg:      Color32::from_rgb(0x2d, 0x08, 0x50),
                card_border:  Color32::from_rgb(0x3c, 0x14, 0x5a),
                titlebar_bg:  Color32::from_rgb(0x0a, 0x00, 0x18),
                hover_bg:     Color32::from_rgb(0x23, 0x04, 0x41),
                text_primary: Color32::from_rgb(0xc3, 0xff, 0x28),
                text_subtle:  Color32::from_rgb(0x00, 0xa0, 0x8c),
                text_dim:     Color32::from_rgb(0x2d, 0x08, 0x50),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
        }
    }

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

        visuals.widgets.active.bg_fill = self.hover_bg;
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
