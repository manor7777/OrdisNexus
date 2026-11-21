use crate::app::App;
use crate::insight::MasteryState;
use crate::theme::*;
use eframe::egui::{self, RichText};

const FILTERS: [&str; 5] = ["All", "Not mastered", "Owned, not mastered", "Not owned", "Mastered"];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    if app.me.acct.is_none() {
        ui.label(RichText::new("Load your inventory on the Overview tab.").color(SUBTLE));
        return;
    }
    let total = app.mastery_rows.len().max(1);
    let mastered = app.mastery_rows.iter().filter(|r| r.state == MasteryState::Mastered).count();
    let owned = app.mastery_rows.iter().filter(|r| r.state == MasteryState::Owned).count();
    let mr = app.me.acct.as_ref().map_or(0, |a| a.mr);
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Mastery Rank {mr}")).size(20.0).strong());
        ui.add(egui::ProgressBar::new(mastered as f32 / total as f32).desired_width(260.0).text(format!("{mastered} / {total} mastered")));
        chip(ui, &format!("{owned} owned, ready to level"), WARN);
    });
    subtle(ui, "Mastery doesn't make anything available to friends. Sharing is controlled in the Friends tab.");

    let mut cats: Vec<String> = app.mastery_rows.iter().map(|r| r.category.clone()).collect();
    cats.sort();
    cats.dedup();
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.mastery_search).hint_text("🔍  Search…").desired_width(220.0));
        egui::ComboBox::from_id_salt("mastery_filter").selected_text(FILTERS[app.mastery_filter as usize]).show_ui(ui, |ui| {
            for (i, f) in FILTERS.iter().enumerate() {
                ui.selectable_value(&mut app.mastery_filter, i as u8, *f);
            }
        });
        egui::ComboBox::from_id_salt("mastery_cat")
            .selected_text(app.mastery_category.clone().unwrap_or_else(|| "All types".into()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.mastery_category, None, "All types");
                for c in &cats {
                    ui.selectable_value(&mut app.mastery_category, Some(c.clone()), c);
                }
            });
    });

    let q = app.mastery_search.to_lowercase();
    let filter = app.mastery_filter;
    let visible: Vec<usize> = app
        .mastery_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| match filter {
            1 => r.state != MasteryState::Mastered,
            2 => r.state == MasteryState::Owned,
            3 => r.state == MasteryState::NotOwned,
            4 => r.state == MasteryState::Mastered,
            _ => true,
        })
        .filter(|(_, r)| app.mastery_category.as_ref().is_none_or(|c| &r.category == c))
        .filter(|(_, r)| q.is_empty() || r.name.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();
    subtle(ui, &format!("{} items", visible.len()));
    card(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 36.0, visible.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for &i in &visible[range] {
                let r = &app.mastery_rows[i];
                ui.horizontal(|ui| {
                    ui.set_height(36.0);
                    icon(ui, &mut app.icons, app.cat.icon(&r.unique), 30.0);
                    ui.add_sized([280.0, 24.0], egui::Label::new(&r.name).truncate());
                    ui.add_sized([130.0, 24.0], egui::Label::new(RichText::new(&r.category).color(SUBTLE)).truncate());
                    if r.mr_req > 0 {
                        chip(ui, &format!("MR {}", r.mr_req), SUBTLE);
                    }
                    match r.state {
                        MasteryState::Mastered => chip(ui, "mastered", GOOD),
                        MasteryState::Owned => chip(ui, "owned", WARN),
                        MasteryState::NotOwned => chip(ui, "not owned", BAD),
                    }
                });
            }
        });
    });
}
