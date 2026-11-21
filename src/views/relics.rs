use crate::app::App;
use crate::theme::*;
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    if app.me.acct.is_none() {
        ui.label(RichText::new("Load your inventory on the Overview tab.").color(SUBTLE));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.relic_search).hint_text("🔍  Relic or reward…").desired_width(240.0));
        ui.checkbox(&mut app.relic_useful_only, "Only relics that help my goals");
    });
    let q = app.relic_search.to_lowercase();
    let rows: Vec<usize> = app
        .relic_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| !app.relic_useful_only || r.useful)
        .filter(|(_, r)| q.is_empty() || r.name.to_lowercase().contains(&q) || r.rewards.iter().any(|x| x.name.to_lowercase().contains(&q)))
        .map(|(i, _)| i)
        .collect();
    let total: u64 = rows.iter().map(|&i| app.relic_rows[i].count).sum();
    subtle(ui, &format!("{} relic types · {} relics. Highlighted rewards are parts your goals still need.", rows.len(), total));

    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 96.0, rows.len(), |ui, range| {
        for &i in &rows[range] {
            let r = &app.relic_rows[i];
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} Relic", r.name)).size(17.0).strong().color(if r.useful { ACCENT } else { TEXT }));
                    chip(ui, r.refinement, SUBTLE);
                    chip(ui, &format!("×{}", r.count), WARN);
                    if r.useful {
                        chip(ui, "helps a goal", GOOD);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for x in &r.rewards {
                        let col = if x.useful { GOOD } else { rarity_color(&x.rarity) };
                        icon(ui, &mut app.icons, app.cat.icon(&x.unique), 22.0);
                        ui.label(RichText::new(&x.name).color(col).size(13.0));
                    }
                });
            });
        }
    });
}
