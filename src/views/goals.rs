use crate::app::App;
use crate::goals::status;
use crate::settings::GoalKind;
use crate::theme::*;
use crate::util::{blueprint_form, fmt_duration, fmt_n};
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut remove: Option<usize> = None;
    let mut add: Option<(GoalKind, String, String, u64)> = None;
    let mut error: Option<String> = None;

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Add a goal");
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("goal_kind")
                    .selected_text(["Build an item", "Collect something", "Reach a mastery rank"][app.goal_kind as usize])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.goal_kind, 0, "Build an item");
                        ui.selectable_value(&mut app.goal_kind, 1, "Collect something");
                        ui.selectable_value(&mut app.goal_kind, 2, "Reach a mastery rank");
                    });
                if app.goal_kind != 2 {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.goal_query)
                            .hint_text(if app.goal_kind == 0 { "e.g. Wisp Prime" } else { "e.g. Oxium, Forma, a Prime part…" })
                            .desired_width(260.0),
                    );
                }
                if app.goal_kind != 0 {
                    ui.label(if app.goal_kind == 1 { "how many" } else { "target MR" });
                    ui.add(egui::DragValue::new(&mut app.goal_count).range(1..=1_000_000));
                }
                if ui.add(accent_button("Add goal")).clicked() {
                    match app.goal_kind {
                        0 => match app.cat.find_item(&app.goal_query) {
                            Some(it) => add = Some((GoalKind::Build, it.unique.clone(), it.name.clone(), 1)),
                            None => error = Some(format!("No item called '{}'", app.goal_query)),
                        },
                        1 => match app.cat.search_names(&app.goal_query, 1).into_iter().next() {
                            Some((u, n)) => add = Some((GoalKind::Collect, u, n, app.goal_count)),
                            None => error = Some(format!("Nothing found for '{}'", app.goal_query)),
                        },
                        _ => add = Some((GoalKind::Mastery, String::new(), format!("Mastery rank {}", app.goal_count), app.goal_count)),
                    }
                }
            });
            // suggestions
            if app.goal_kind != 2 && app.goal_query.trim().len() >= 2 {
                let q = app.goal_query.to_lowercase();
                let sugg: Vec<String> = if app.goal_kind == 0 {
                    let mut v: Vec<&str> = app.cat.items.iter().filter(|i| i.name.to_lowercase().contains(&q)).map(|i| i.name.as_str()).collect();
                    v.sort_by_key(|n| (!n.contains("Prime"), n.len()));
                    v.into_iter().take(6).map(String::from).collect()
                } else {
                    app.cat.search_names(&q, 6).into_iter().map(|(_, n)| n).collect()
                };
                if !(sugg.len() == 1 && sugg[0].to_lowercase() == q) {
                    ui.horizontal_wrapped(|ui| {
                        for s in sugg {
                            if ui.small_button(&s).clicked() {
                                app.goal_query = s;
                            }
                        }
                    });
                }
            }
            if let Some(e) = &error {
                ui.label(RichText::new(e).color(BAD));
            }
        });

        if app.settings.goals.is_empty() {
            ui.add_space(8.0);
            subtle(ui, "No goals yet. Try 'Build an item' → Wisp Prime, or 'Collect' → Oxium ×1500.");
        }

        let Some(me) = &app.me.acct else {
            ui.add_space(8.0);
            ui.label(RichText::new("Load your inventory on the Overview tab to see progress.").color(SUBTLE));
            return;
        };
        for (i, g) in app.settings.goals.iter_mut().enumerate() {
            let st = status(&app.cat, me, g);
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    icon(ui, &mut app.icons, app.cat.icon(&g.target), 40.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&g.name).size(18.0).strong());
                            if st.done {
                                chip(ui, "done", GOOD);
                            } else {
                                chip(ui, &st.summary, WARN);
                            }
                        });
                        ui.add(egui::ProgressBar::new(st.fraction).desired_width(320.0).text(format!("{:.0}%", st.fraction * 100.0)));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑").on_hover_text("Remove goal").clicked() {
                            remove = Some(i);
                        }
                        if ui.checkbox(&mut g.shared, "share").on_hover_text("Let your friend see what this goal still needs").changed() {
                            changed = true;
                        }
                    });
                });
                if !st.done {
                    egui::CollapsingHeader::new("What's still needed").id_salt(("goal", g.id)).show(ui, |ui| {
                        for (u, n) in &st.missing_parts {
                            let bp = blueprint_form(u);
                            ui.horizontal(|ui| {
                                icon(ui, &mut app.icons, app.cat.icon(&bp), 26.0);
                                ui.label(RichText::new(format!("{} ×{n}", app.cat.display(&bp))).strong());
                            });
                            let relics = app.cat.relics_with(&bp);
                            if !relics.is_empty() {
                                let owned: Vec<String> = app
                                    .relic_rows
                                    .iter()
                                    .filter(|r| r.rewards.iter().any(|x| x.useful && x.unique.eq_ignore_ascii_case(&bp)))
                                    .map(|r| format!("{} {} ×{}", r.name, r.refinement, r.count))
                                    .collect();
                                if !owned.is_empty() {
                                    ui.label(RichText::new(format!("   You own: {}", owned.join(", "))).color(GOOD).size(13.0));
                                }
                                let list: Vec<String> = relics.iter().take(5).map(|(r, rar)| format!("{} ({rar})", r.name)).collect();
                                ui.label(RichText::new(format!("   Relics: {}", list.join(", "))).color(SUBTLE).size(13.0));
                            }
                        }
                        for (u, n) in &st.missing_resources {
                            let name = app.cat.display(u);
                            ui.horizontal(|ui| {
                                icon(ui, &mut app.icons, app.cat.icon(u), 26.0);
                                ui.label(RichText::new(format!("{name}: short by {}", fmt_n(*n))).strong());
                                ui.label(RichText::new(format!("(have {})", fmt_n(me.count(u)))).color(SUBTLE));
                            });
                            let drops = app.cat.drops_for(&name);
                            if !drops.is_empty() {
                                let l: Vec<String> = drops.iter().take(3).map(|d| format!("{} ({}%)", d.place, d.chance)).collect();
                                ui.label(RichText::new(format!("   Drops from: {}", l.join("; "))).color(SUBTLE).size(13.0));
                            } else if let Some(h) = app.cat.resource_hint(u) {
                                ui.label(RichText::new(format!("   {h}")).color(SUBTLE).size(13.0));
                            }
                        }
                        if let Some(r) = &st.resolution {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!("Credits {} · build time {}", fmt_n(r.credits), fmt_duration(r.secs as i64 * 1000))).color(SUBTLE).size(13.0),
                            );
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("note").color(SUBTLE).size(12.0));
                    if ui.add(egui::TextEdit::singleline(&mut g.note).desired_width(320.0)).changed() {
                        changed = true;
                    }
                });
            });
        }
    });

    if let Some((kind, target, name, count)) = add {
        app.settings.add_goal(kind, &target, &name, count);
        app.goal_query.clear();
        changed = true;
    }
    if let Some(i) = remove {
        app.settings.goals.remove(i);
        changed = true;
    }
    if changed {
        app.settings_changed();
    }
}
