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
    Terminal,
    Washi,
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
            ThemeId::Terminal   => "Terminal",
            ThemeId::Washi      => "Washi",
        }
    }

    pub const ALL: &'static [ThemeId] = &[
        ThemeId::CoralStorm,
        ThemeId::Shibui,
        ThemeId::Kasane,
        ThemeId::ColdSteel,
        ThemeId::Jizo,
        ThemeId::HotSteel,
        ThemeId::Terminal,
        ThemeId::Washi,
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

// Shared accent colours for the dark themes. Coral Storm, Jizo and Washi carry their own: the same
// hues, at a lightness that reaches 4.5:1 on their surfaces (Coral Storm's `#005151` panel is too
// light for the reds; the two light themes need every accent darkened).
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
                card_bg:      Color32::from_rgb(0x00, 0x51, 0x51),
                card_border:  Color32::from_rgb(0x00, 0x38, 0x38),
                titlebar_bg:  Color32::from_rgb(0x00, 0x0a, 0x0a),
                hover_bg:     Color32::from_rgb(0x00, 0x2c, 0x2c),
                text_primary: Color32::from_rgb(0xff, 0xf4, 0xee),
                text_subtle:  Color32::from_rgb(0xff, 0xf5, 0x71),
                text_dim:     Color32::from_rgb(0x00, 0x2c, 0x2c),
                accent_cpu:   Color32::from_rgb(0xff, 0x9e, 0x8a),
                accent_mem:   ACCENT_MEM,
                accent_gpu:   Color32::from_rgb(0xff, 0x9e, 0x82),
                accent_net:   ACCENT_NET,
                accent_disk:  Color32::from_rgb(0xff, 0x9d, 0x8b),
                accent_temp:  Color32::from_rgb(0xff, 0x9d, 0x8e),
                ok: OK, warn: WARN,
                crit:         Color32::from_rgb(0xff, 0x9d, 0x95),
            },
            ThemeId::Shibui => Self {
                bg:           Color32::from_rgb(0x14, 0x0a, 0x0c),
                card_bg:      Color32::from_rgb(0x24, 0x13, 0x18),
                card_border:  Color32::from_rgb(0x18, 0x0d, 0x10),
                titlebar_bg:  Color32::from_rgb(0x0a, 0x05, 0x06),
                hover_bg:     Color32::from_rgb(0x1c, 0x0f, 0x12),
                text_primary: Color32::from_rgb(0xed, 0xe3, 0xd6),
                text_subtle:  Color32::from_rgb(0xa8, 0x8a, 0x8c),
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
                hover_bg:     Color32::from_rgb(0x29, 0x67, 0xcc),
                text_primary: Color32::from_rgb(0xe8, 0xec, 0xf1),
                text_subtle:  Color32::from_rgb(0x7f, 0x8f, 0xa6),
                text_dim:     Color32::from_rgb(0x16, 0x17, 0x1b),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            // Light since vault 2026-10-10h: Coral Storm inverted.
            ThemeId::Jizo => Self {
                bg:           Color32::from_rgb(0xff, 0xed, 0xed),
                card_bg:      Color32::from_rgb(0xff, 0xae, 0xae),
                card_border:  Color32::from_rgb(0x82, 0x60, 0x67),
                titlebar_bg:  Color32::from_rgb(0xff, 0xd3, 0xd3),
                hover_bg:     Color32::from_rgb(0x5e, 0xb6, 0xb6),
                text_primary: Color32::from_rgb(0x00, 0x0b, 0x11),
                text_subtle:  Color32::from_rgb(0x00, 0x0a, 0x8e),
                text_dim:     Color32::from_rgb(0xff, 0xae, 0xae),
                accent_cpu:   Color32::from_rgb(0x9e, 0x1a, 0x00),
                accent_mem:   Color32::from_rgb(0x00, 0x5b, 0x4f),
                accent_gpu:   Color32::from_rgb(0x9a, 0x22, 0x00),
                accent_net:   Color32::from_rgb(0x00, 0x5b, 0x4d),
                accent_disk:  Color32::from_rgb(0x9f, 0x18, 0x00),
                accent_temp:  Color32::from_rgb(0xa0, 0x15, 0x00),
                ok:           Color32::from_rgb(0x00, 0x5b, 0x4d),
                warn:         Color32::from_rgb(0x72, 0x46, 0x00),
                crit:         Color32::from_rgb(0xa3, 0x0c, 0x00),
            },
            ThemeId::HotSteel => Self {
                bg:           Color32::from_rgb(0x0d, 0x0a, 0x0b),
                card_bg:      Color32::from_rgb(0x1b, 0x15, 0x17),
                card_border:  Color32::from_rgb(0x9f, 0x4f, 0x6f),
                titlebar_bg:  Color32::from_rgb(0x08, 0x06, 0x07),
                hover_bg:     Color32::from_rgb(0xc4, 0x29, 0x6b),
                text_primary: Color32::from_rgb(0xf1, 0xe8, 0xec),
                text_subtle:  Color32::from_rgb(0xa6, 0x80, 0x8a),
                text_dim:     Color32::from_rgb(0x1b, 0x15, 0x17),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            // Vault Theme-Palettes-and-Templates.md, added 2026-10-09: near-black terminal greys.
            ThemeId::Terminal => Self {
                bg:           Color32::from_rgb(0x0c, 0x0c, 0x0c),
                card_bg:      Color32::from_rgb(0x14, 0x14, 0x14),
                card_border:  Color32::from_rgb(0x70, 0x70, 0x70),
                titlebar_bg:  Color32::from_rgb(0x06, 0x06, 0x06),
                hover_bg:     Color32::from_rgb(0x00, 0x60, 0x68),
                text_primary: Color32::from_rgb(0xd0, 0xd0, 0xd0),
                text_subtle:  Color32::from_rgb(0x8a, 0x8a, 0x8a),
                text_dim:     Color32::from_rgb(0x14, 0x14, 0x14),
                accent_cpu: ACCENT_CPU, accent_mem: ACCENT_MEM, accent_gpu: ACCENT_GPU,
                accent_net: ACCENT_NET, accent_disk: ACCENT_DISK, accent_temp: ACCENT_TEMP,
                ok: OK, warn: WARN, crit: CRIT,
            },
            // Vault 2026-10-10c, light: rice-paper and pale-green surfaces, black ink.
            ThemeId::Washi => Self {
                bg:           Color32::from_rgb(0xf3, 0xed, 0xe0),
                card_bg:      Color32::from_rgb(0xe7, 0xff, 0xe5),
                card_border:  Color32::from_rgb(0x7a, 0x6d, 0x5a),
                titlebar_bg:  Color32::from_rgb(0xd5, 0xf1, 0xd4),
                hover_bg:     Color32::from_rgb(0xf2, 0xcd, 0xbd),
                text_primary: Color32::from_rgb(0x00, 0x00, 0x00),
                text_subtle:  Color32::from_rgb(0x59, 0x4e, 0x42),
                text_dim:     Color32::from_rgb(0xe7, 0xff, 0xe5),
                accent_cpu:   Color32::from_rgb(0xcc, 0x22, 0x00),
                accent_mem:   Color32::from_rgb(0x00, 0x76, 0x67),
                accent_gpu:   Color32::from_rgb(0xc7, 0x2c, 0x00),
                accent_net:   Color32::from_rgb(0x00, 0x76, 0x64),
                accent_disk:  Color32::from_rgb(0xcd, 0x20, 0x00),
                accent_temp:  Color32::from_rgb(0xce, 0x1c, 0x00),
                ok:           Color32::from_rgb(0x00, 0x76, 0x64),
                warn:         Color32::from_rgb(0x95, 0x5c, 0x00),
                crit:         Color32::from_rgb(0xd2, 0x0f, 0x00),
            },
        }
    }

    /// True for Jizo and Washi: the background is closer to white than to black. `apply` picks
    /// egui's light base from this, so no theme needs a "light" flag.
    pub fn is_light(&self) -> bool {
        luminance(self.bg) > 0.5
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
        let light = self.is_light();
        let mut visuals = if light { Visuals::light() } else { Visuals::dark() };

        visuals.panel_fill = self.bg;
        visuals.window_fill = self.bg;
        visuals.faint_bg_color = self.card_bg;
        visuals.extreme_bg_color = if light { self.bg } else { Color32::from_rgb(3, 4, 8) };
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

/// WCAG 2.x relative luminance.
fn luminance(c: Color32) -> f64 {
    let lin = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.040_45 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG 2.x contrast ratio, 1.0 to 21.0.
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::{contrast, Theme, ThemeId};
    use egui::Color32;

    /// Body text 7:1, and 4.5:1 for everything else drawn as text (card labels and values, the
    /// title), on every surface text sits on. The metric accents are text here, not only strokes.
    #[test]
    fn every_theme_meets_contrast_minimums() {
        let mut failures = Vec::new();
        for &id in ThemeId::ALL {
            let t = Theme::from_id(id);
            let surfaces = [("bg", t.bg), ("card_bg", t.card_bg), ("titlebar_bg", t.titlebar_bg)];
            let rules = [
                ("text_primary", t.text_primary, 7.0),
                ("text_subtle", t.text_subtle, 4.5),
                ("accent_cpu", t.accent_cpu, 4.5),
                ("accent_mem", t.accent_mem, 4.5),
                ("accent_gpu", t.accent_gpu, 4.5),
                ("accent_net", t.accent_net, 4.5),
                ("accent_disk", t.accent_disk, 4.5),
                ("accent_temp", t.accent_temp, 4.5),
                ("ok", t.ok, 4.5),
                ("warn", t.warn, 4.5),
                ("crit", t.crit, 4.5),
            ];
            for (token, fg, min) in rules {
                for (surface, bg) in surfaces {
                    let ratio = contrast(fg, bg);
                    if ratio < min {
                        failures.push(format!("{}: {token} on {surface} = {ratio:.2} < {min}", id.label()));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "contrast failures:\n{}", failures.join("\n"));
    }

    /// Pins the formula to published values, so a broken `contrast` cannot pass the gate above.
    #[test]
    fn contrast_matches_wcag_reference_values() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 1e-9);
        assert!((contrast(Color32::from_rgb(0x77, 0x77, 0x77), Color32::WHITE) - 4.48).abs() < 0.01);
    }

    #[test]
    fn only_jizo_and_washi_get_the_light_base() {
        for &id in ThemeId::ALL {
            assert_eq!(
                Theme::from_id(id).is_light(),
                matches!(id, ThemeId::Jizo | ThemeId::Washi),
                "{}",
                id.label()
            );
        }
    }

    #[test]
    fn all_lists_every_variant_once_including_the_default() {
        let mut labels: Vec<_> = ThemeId::ALL.iter().map(|t| t.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 8);
        assert!(ThemeId::ALL.contains(&ThemeId::default()));
    }
}
