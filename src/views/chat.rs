use crate::app::App;
use crate::theme::*;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

const QUICK: [&str; 7] = [
    "What should I farm tonight?",
    "What are my goals?",
    "What can I build right now?",
    "Which relics should I crack?",
    "Can my friend help me?",
    "Any good fissures?",
    "What's in the Foundry?",
];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut send: Option<String> = None;
    ui.horizontal_wrapped(|ui| {
        for q in QUICK {
            if ui.small_button(q).clicked() {
                send = Some(q.to_string());
            }
        }
    });
    let h = (ui.available_height() - 56.0).max(140.0);
    egui::ScrollArea::vertical().max_height(h).stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
        for (i, m) in app.chat.iter().enumerate() {
            let (fill, align) = if m.user {
                (ACCENT.gamma_multiply(0.22), egui::Align::Max)
            } else {
                (CARD, egui::Align::Min)
            };
            ui.with_layout(egui::Layout::top_down(align), |ui| {
                egui::Frame::new()
                    .fill(fill)
                    .stroke(Stroke::new(1.0, STROKE))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_max_width((ui.available_width() * 0.85).max(240.0));
                        if !m.user {
                            ui.label(RichText::new("ORDIS").size(10.0).color(ACCENT));
                        }
                        ui.label(&m.text);
                        if m.pending {
                            ui.label(RichText::new("Ordis is putting it in his own words…").color(SUBTLE).size(12.0));
                        }
                        if !m.user && app.settings.llm.enabled && !m.facts.is_empty() && !m.pending {
                            egui::CollapsingHeader::new(RichText::new("facts").color(SUBTLE).size(11.0)).id_salt(("facts", i)).show(ui, |ui| {
                                ui.label(RichText::new(&m.facts).color(Color32::GRAY).size(12.0));
                            });
                        }
                    });
            });
        }
    });
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 90.0).max(200.0);
        let r = ui.add(egui::TextEdit::singleline(&mut app.chat_input).hint_text("Ask Ordis… (Win+H dictates by voice)").desired_width(w));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (ui.add(accent_button("Send")).clicked() || enter) && !app.chat_input.trim().is_empty() {
            send = Some(std::mem::take(&mut app.chat_input));
            r.request_focus();
        }
    });
    if let Some(q) = send {
        let ctx = ui.ctx().clone();
        app.ask(&q, &ctx);
    }
}
