use crate::app::App;
use crate::theme::*;
use crate::util::fmt_n;
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    if app.me.acct.is_none() {
        ui.label(RichText::new("Load your inventory on the Overview tab.").color(SUBTLE));
        return;
    }
    let mut cats: Vec<&str> = app.inv_rows.iter().map(|r| r.category.as_str()).collect();
    cats.dedup();
    let cats: Vec<String> = cats.into_iter().map(String::from).collect();

    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.inv_search).hint_text("🔍  Search your inventory…").desired_width(260.0));
        egui::ComboBox::from_id_salt("inv_cat")
            .selected_text(app.inv_category.clone().unwrap_or_else(|| "All categories".into()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.inv_category, None, "All categories");
                for c in &cats {
                    ui.selectable_value(&mut app.inv_category, Some(c.clone()), c);
                }
            });
        ui.checkbox(&mut app.inv_dupes, "Duplicates only (spares)");
    });

    let q = app.inv_search.to_lowercase();
    let visible: Vec<usize> = app
        .inv_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| app.inv_category.as_ref().is_none_or(|c| &r.category == c))
        .filter(|(_, r)| q.is_empty() || r.name_lower.contains(&q))
        .filter(|(_, r)| !app.inv_dupes || r.count > 1)
        .map(|(i, _)| i)
        .collect();
    let total: u64 = visible.iter().map(|&i| app.inv_rows[i].count).sum();
    subtle(ui, &format!("{} items · {} total copies", fmt_n(visible.len() as u64), fmt_n(total)));

    card(ui, |ui| {
        let w = ui.available_width();
        let (cname, ccat, cnum) = (w * 0.5, w * 0.25, 90.0);
        ui.horizontal(|ui| {
            ui.add_space(40.0);
            for (t, wd) in [("Item", cname), ("Category", ccat), ("Count", cnum)] {
                ui.add_sized([wd, 20.0], egui::Label::new(RichText::new(t).color(SUBTLE).strong()));
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 36.0, visible.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for &i in &visible[range] {
                let r = &app.inv_rows[i];
                ui.horizontal(|ui| {
                    ui.set_height(36.0);
                    icon(ui, &mut app.icons, app.cat.icon(&r.path), 32.0);
                    ui.add_sized([cname, 24.0], egui::Label::new(&r.name).truncate()).on_hover_text(&r.path);
                    ui.add_sized([ccat, 24.0], egui::Label::new(RichText::new(&r.category).color(SUBTLE)).truncate());
                    ui.add_sized([cnum, 24.0], egui::Label::new(RichText::new(fmt_n(r.count)).strong()));
                });
            }
        });
    });
}
