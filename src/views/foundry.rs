use crate::app::App;
use crate::insight::foundry_rows;
use crate::theme::*;
use crate::util::{fmt_duration, fmt_n};
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let Some(me) = &app.me.acct else {
        ui.label(RichText::new("Load your inventory on the Overview tab.").color(SUBTLE));
        return;
    };
    let rows = foundry_rows(&app.cat, me);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "In the Foundry");
            if rows.is_empty() {
                subtle(ui, "Nothing is being built.");
            }
            for r in &rows {
                ui.horizontal(|ui| {
                    icon(ui, &mut app.icons, app.cat.icon(&r.unique), 34.0);
                    ui.label(RichText::new(&r.name).size(16.0).strong());
                    if r.remaining_ms <= 0 {
                        chip(ui, "ready to claim", GOOD);
                    } else {
                        chip(ui, &format!("{} left", fmt_duration(r.remaining_ms)), WARN);
                    }
                });
            }
            ui.add_space(4.0);
            subtle(ui, &format!("Credits {} · Platinum {} · Endo {}", fmt_n(me.credits), fmt_n(me.platinum), fmt_n(me.endo)));
        });

        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Ready to build now");
            subtle(ui, "Blueprints you hold where every resource and the credits are covered.");
            if app.buildable.is_empty() {
                ui.label("Nothing right now.");
            }
            for u in &app.buildable {
                ui.horizontal(|ui| {
                    icon(ui, &mut app.icons, app.cat.icon(u), 28.0);
                    ui.label(app.cat.display(u));
                    if let Some(r) = app.cat.recipe(u) {
                        subtle(ui, &format!("{} credits · {}", fmt_n(r.credits), fmt_duration(r.secs as i64 * 1000)));
                    }
                });
            }
        });
    });
}
