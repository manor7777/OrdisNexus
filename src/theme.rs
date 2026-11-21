//! Fluent-style look and the small widgets every view shares.

use crate::icons::Icons;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

pub const BG: Color32 = Color32::from_rgb(32, 32, 32);
pub const CARD: Color32 = Color32::from_rgb(43, 43, 43);
pub const CARD_HI: Color32 = Color32::from_rgb(55, 55, 55);
pub const STROKE: Color32 = Color32::from_rgb(64, 64, 64);
pub const ACCENT: Color32 = Color32::from_rgb(96, 205, 255);
pub const TEXT: Color32 = Color32::from_rgb(255, 255, 255);
pub const SUBTLE: Color32 = Color32::from_rgb(170, 170, 170);
pub const GOOD: Color32 = Color32::from_rgb(108, 203, 95);
pub const WARN: Color32 = Color32::from_rgb(252, 225, 0);
pub const BAD: Color32 = Color32::from_rgb(255, 153, 164);

pub fn apply_fluent_theme(ctx: &egui::Context) {
    // Use Segoe UI (the WinUI font) when running on Windows.
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("segoe".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "segoe".into());
        ctx.set_fonts(fonts);
    }
    let mut v = egui::Visuals::dark();
    let r = CornerRadius::same(6);
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(28, 28, 28);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STROKE);
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, STROKE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(66, 66, 66);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(66, 66, 66);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, STROKE);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.interact_size.y = 30.0;
    });
}

pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, STROKE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(14))
        .show(ui, add)
        .inner
}

pub fn accent_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).color(Color32::BLACK).strong())
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(6))
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(20.0).strong());
}

pub fn subtle(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(SUBTLE).size(12.5));
}

pub fn chip(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| ui.label(RichText::new(text).color(color).size(12.0)));
}

pub fn rarity_color(rarity: &str) -> Color32 {
    match rarity.to_lowercase().as_str() {
        "rare" | "r" => WARN,
        "uncommon" | "u" => ACCENT,
        _ => SUBTLE,
    }
}

/// Draws an item icon (or an empty placeholder) and queues its download only
/// when it is actually on screen.
pub fn icon(ui: &mut egui::Ui, icons: &mut Icons, image: Option<&str>, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(36, 36, 36));
    if let Some(img) = image.filter(|_| ui.is_rect_visible(rect)) {
        let ctx = ui.ctx().clone();
        if let Some(tex) = icons.get(&ctx, img) {
            egui::Image::from_texture((tex.id(), rect.size())).paint_at(ui, rect.shrink(2.0));
        }
    }
}
