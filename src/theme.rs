//! Colour palette and egui style configuration.

use egui::{Color32, Margin, Rounding, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ThemeId {
    CoralStorm,
    Shibui,
    Kasane,
    #[default]
    ColdSteel,
    Jizo,
    HotSteel,
}

impl ThemeId {
    pub fn label(self) -> &'static str {
        match self {
            ThemeId::CoralStorm => "Coral Storm",
            ThemeId::Shibui     => "Shibui",
            ThemeId::Kasane     => "Kasane",
            ThemeId::ColdSteel  => "Cold Steel",
            ThemeId::Jizo       => "Jizo",
            ThemeId::HotSteel   => "Hot Steel",
        }
    }

    pub const ALL: &'static [ThemeId] = &[
        ThemeId::CoralStorm,
        ThemeId::Shibui,
        ThemeId::Kasane,
        ThemeId::ColdSteel,
        ThemeId::Jizo,
        ThemeId::HotSteel,
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
    fn default() -> Self { Self::from_id(ThemeId::ColdSteel) }
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
            ThemeId::Shibui => Self {
                bg:           Color32::from_rgb(0x14, 0x0a, 0x0c),
                card_bg:      Color32::from_rgb(0x24, 0x13, 0x18),
                card_border:  Color32::from_rgb(0x18, 0x0d, 0x10),
                titlebar_bg:  Color32::from_rgb(0x0a, 0x05, 0x06),
                hover_bg:     Color32::from_rgb(0x1c, 0x0f, 0x12),
                text_primary: Color32::from_rgb(0xed, 0xe3, 0xd6),
                text_subtle:  Color32::from_rgb(0x7a, 0x27, 0x32),
                text_dim:     Color32::from_rgb(0x24, 0x13, 0x18),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::Kasane => Self {
                bg:           Color32::from_rgb(0x0d, 0x07, 0x14),
                card_bg:      Color32::from_rgb(0x1c, 0x0f, 0x2b),
                card_border:  Color32::from_rgb(0x14, 0x0a, 0x1e),
                titlebar_bg:  Color32::from_rgb(0x06, 0x03, 0x0a),
                hover_bg:     Color32::from_rgb(0x17, 0x0c, 0x22),
                text_primary: Color32::from_rgb(0xf7, 0xcf, 0xd4),
                text_subtle:  Color32::from_rgb(0xa8, 0x7c, 0xc2),
                text_dim:     Color32::from_rgb(0x1c, 0x0f, 0x2b),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::ColdSteel => Self {
                bg:           Color32::from_rgb(0x0a, 0x0b, 0x0d),
                card_bg:      Color32::from_rgb(0x16, 0x17, 0x1b),
                card_border:  Color32::from_rgb(0x3f, 0x6f, 0x99),
                titlebar_bg:  Color32::from_rgb(0x06, 0x07, 0x0a),
                hover_bg:     Color32::from_rgb(0x2f, 0x6f, 0xd6),
                text_primary: Color32::from_rgb(0xe8, 0xec, 0xf1),
                text_subtle:  Color32::from_rgb(0x7f, 0x8f, 0xa6),
                text_dim:     Color32::from_rgb(0x16, 0x17, 0x1b),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::Jizo => Self {
                bg:           Color32::from_rgb(0x12, 0x0f, 0x0c),
                card_bg:      Color32::from_rgb(0x1f, 0x19, 0x13),
                card_border:  Color32::from_rgb(0x6b, 0x5f, 0x52),
                titlebar_bg:  Color32::from_rgb(0x0c, 0x0a, 0x08),
                hover_bg:     Color32::from_rgb(0x2a, 0x10, 0x15),
                text_primary: Color32::from_rgb(0xe3, 0xcf, 0xd2),
                text_subtle:  Color32::from_rgb(0x8f, 0x81, 0x75),
                text_dim:     Color32::from_rgb(0x1f, 0x19, 0x13),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            ThemeId::HotSteel => Self {
                bg:           Color32::from_rgb(0x0d, 0x0a, 0x0b),
                card_bg:      Color32::from_rgb(0x1b, 0x15, 0x17),
                card_border:  Color32::from_rgb(0x8f, 0x3f, 0x5f),
                titlebar_bg:  Color32::from_rgb(0x08, 0x06, 0x07),
                hover_bg:     Color32::from_rgb(0xd6, 0x3f, 0x7f),
                text_primary: Color32::from_rgb(0xf1, 0xe8, 0xec),
                text_subtle:  Color32::from_rgb(0xa6, 0x80, 0x8a),
                text_dim:     Color32::from_rgb(0x1b, 0x15, 0x17),
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
        visuals.widgets.noninteractive.expansion = 0.0;

        visuals.widgets.inactive.bg_fill = self.card_bg;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_subtle);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.card_border);
        visuals.widgets.inactive.rounding = Rounding::same(4.0);
        visuals.widgets.inactive.expansion = 0.0;

        visuals.widgets.hovered.bg_fill = self.hover_bg;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.hovered.rounding = Rounding::same(4.0);
        visuals.widgets.hovered.expansion = 0.0;                     // NEVER > 0 — causes layout shift/bounce

        visuals.widgets.active.bg_fill = self.hover_bg;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.active.rounding = Rounding::same(4.0);
        visuals.widgets.active.expansion = 0.0;                      // NEVER > 0 — causes layout shift/bounce

        visuals.widgets.open.bg_fill = self.hover_bg;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, self.accent_cpu);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, self.accent_cpu); // match other states — prevents open-state bounce
        visuals.widgets.open.rounding = Rounding::same(4.0);
        visuals.widgets.open.expansion = 0.0;

        visuals.override_text_color = Some(self.text_primary);

        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.window_margin = Margin::same(0.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);

        ctx.set_style(style);
    }
}
