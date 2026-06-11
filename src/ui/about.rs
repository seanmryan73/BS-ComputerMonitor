//! Minimal HUD-style about panel.

use egui::{Align2, Color32, RichText, Vec2};

use crate::theme::Theme;

pub fn show(ctx: &egui::Context, theme: &Theme, open: &mut bool) {
    if !*open {
        return;
    }

    egui::Window::new("##bs_about")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(360.0, 218.0))
        .frame(
            egui::Frame::none()
                .fill(Color32::from_rgb(0x05, 0x04, 0x10))
                .stroke(egui::Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(
                        theme.accent_cpu.r(),
                        theme.accent_cpu.g(),
                        theme.accent_cpu.b(),
                        70,
                    ),
                ))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(24.0)),
        )
        .show(ctx, |ui| {
            // Title
            ui.label(
                RichText::new("BS COMPUTER MONITOR")
                    .color(theme.accent_cpu)
                    .monospace()
                    .size(17.0)
                    .strong(),
            );
            ui.label(
                RichText::new("v0.1.0  ·  System Resource Monitor")
                    .color(theme.text_subtle)
                    .monospace()
                    .size(10.0),
            );

            ui.add_space(10.0);
            divider(ui, theme.accent_cpu, 0.35);
            ui.add_space(8.0);

            row(ui, theme, "AUTHOR",   "seanmryan@gmail.com");
            row(ui, theme, "COMPANY",  "BagPipes — BS Solutions");

            ui.add_space(8.0);
            divider(ui, theme.accent_cpu, 0.18);
            ui.add_space(8.0);

            row(ui, theme, "RUNTIME",  "Rust  ·  egui 0.29  ·  eframe 0.29");
            row(ui, theme, "PLATFORM", "Windows  ·  x86_64");

            ui.add_space(14.0);

            ui.vertical_centered(|ui| {
                let btn = egui::Button::new(
                    RichText::new("CLOSE")
                        .color(theme.text_primary)
                        .monospace()
                        .size(11.0),
                )
                .fill(Color32::from_rgba_unmultiplied(30, 30, 50, 220))
                .stroke(egui::Stroke::new(1.0, theme.card_border))
                .min_size(Vec2::new(100.0, 26.0));

                if ui.add(btn).clicked() {
                    *open = false;
                }
            });
        });
}

fn row(ui: &mut egui::Ui, theme: &Theme, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:<10}", label))
                .color(theme.text_subtle)
                .monospace()
                .size(10.5),
        );
        ui.label(
            RichText::new("·")
                .color(theme.text_dim)
                .monospace()
                .size(10.5),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(value)
                .color(theme.text_primary)
                .monospace()
                .size(10.5),
        );
    });
}

fn divider(ui: &mut egui::Ui, color: Color32, alpha_frac: f32) {
    let alpha = (255.0 * alpha_frac) as u8;
    let [r, g, b, _] = color.to_array();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 1.0),
        egui::Sense::hover(),
    );
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, alpha)),
    );
}
