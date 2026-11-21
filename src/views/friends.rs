use crate::app::App;
use crate::share::friend_has;
use crate::theme::*;
use crate::util::{fmt_duration, now_ms};
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut load_snap: Option<std::path::PathBuf> = None;
    let mut export = false;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.columns(2, |cols| {
            // ------------------------------------------------ what your friend shares
            let ui = &mut cols[0];
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                heading(ui, &format!("{}'s snapshot", app.settings.friend_name));
                match &app.friend_snap {
                    Some(f) => {
                        ui.label(RichText::new(&f.name).size(18.0).strong());
                        subtle(ui, &format!("updated {} ago", fmt_duration((now_ms() - f.created_ms).max(0))));
                        if let Some(a) = &f.activity {
                            if !a.is_empty() {
                                ui.label(format!("🎮 {a}"));
                            }
                        }
                        if let Some(m) = &f.mastery {
                            ui.label(format!("Mastery Rank {} · {} items mastered", m.rank, m.mastered.len()));
                        }
                    }
                    None => {
                        subtle(ui, "Nothing yet. Set a shared sync folder in Settings, or load a .nexus.json your friend sent.");
                    }
                }
                if let Some(e) = &app.snap_error {
                    ui.colored_label(BAD, e);
                }
                if ui.button("Load a .nexus.json…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("Ordis snapshot", &["json"]).pick_file() {
                        load_snap = Some(p);
                    }
                }
            });

            if let Some(m) = &app.matching {
                ui.add_space(6.0);
                card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    heading(ui, "Working together");
                    if m.mutual() {
                        chip(ui, "mutual trade possible", GOOD);
                    }
                    subtle(ui, &format!("{} has, you need:", app.settings.friend_name));
                    if m.they_have.is_empty() {
                        ui.label("—");
                    }
                    for (n, c) in &m.they_have {
                        ui.label(format!("• {n} ×{c}"));
                    }
                    ui.add_space(4.0);
                    subtle(ui, &format!("You have, {} needs:", app.settings.friend_name));
                    if m.i_have.is_empty() {
                        ui.label("—");
                    }
                    for (n, c) in &m.i_have {
                        ui.label(format!("• {n} ×{c}"));
                    }
                    if !m.shared_farming.is_empty() {
                        ui.add_space(4.0);
                        subtle(ui, "You are both short of (farm together):");
                        for n in &m.shared_farming {
                            ui.label(format!("• {n}"));
                        }
                    }
                });
            }

            if let Some(f) = &app.friend_snap {
                if let Some(g) = &f.goals {
                    ui.add_space(6.0);
                    card(ui, |ui| {
                        ui.set_width(ui.available_width());
                        heading(ui, "Their goals");
                        if g.is_empty() {
                            subtle(ui, "None shared.");
                        }
                        for x in g {
                            ui.label(RichText::new(&x.name).strong());
                            ui.add(egui::ProgressBar::new(x.fraction).text(&x.summary));
                        }
                    });
                }
                ui.add_space(6.0);
                card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    heading(ui, "Does your friend have…?");
                    ui.add(egui::TextEdit::singleline(&mut app.friend_query).hint_text("🔍  item name").desired_width(240.0));
                    if !app.friend_query.trim().is_empty() {
                        let hits = friend_has(f, app.friend_query.trim());
                        if hits.is_empty() {
                            subtle(ui, "Nothing shared matching that.");
                        }
                        for h in hits.iter().take(30) {
                            ui.horizontal(|ui| {
                                icon(ui, &mut app.icons, app.cat.icon(&h.unique), 24.0);
                                ui.label(format!("{} ×{}", h.name, h.count));
                                if let Some(p) = h.price {
                                    chip(ui, &format!("~{p:.0}p"), SUBTLE);
                                }
                            });
                        }
                    }
                });
            }

            // ------------------------------------------------ what YOU share
            let ui = &mut cols[1];
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                heading(ui, "What you share");
                subtle(ui, "Friends only ever see what is ticked here. Nothing leaves your PC except a snapshot file in your sync folder.");
                let s = &mut app.settings.sharing;
                changed |= ui.checkbox(&mut s.goals, "My goals and progress").changed();
                changed |= ui.checkbox(&mut s.activity, "What I'm doing right now (activity note)").changed();
                changed |= ui.checkbox(&mut s.available_items, "Items I could give away (rules below)").changed();
                changed |= ui.checkbox(&mut s.mastery, "Mastery rank and mastered items").changed();
                changed |= ui.checkbox(&mut s.resource_quantities, "Resource quantities").changed();
                changed |= ui.checkbox(&mut s.full_inventory, "My entire inventory (off unless you really want it)").changed();
                ui.add_space(4.0);
                ui.label("Activity note:");
                changed |= ui.add(egui::TextEdit::singleline(&mut app.settings.activity).hint_text("e.g. farming Rhino Prime relics").desired_width(f32::INFINITY)).changed();
            });
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                heading(ui, "Which items count as giveable");
                let r = &mut app.settings.rules;
                changed |= ui.checkbox(&mut r.prime_parts, "Prime parts (spares)").changed();
                changed |= ui.checkbox(&mut r.mods, "Mods").changed();
                changed |= ui.checkbox(&mut r.relics, "Relics").changed();
                changed |= ui.checkbox(&mut r.require_mastered, "Only parts of items I've already mastered").changed();
                changed |= ui.checkbox(&mut r.unknown_price_ok, "Share items even if their price is unknown").changed();
                ui.horizontal(|ui| {
                    ui.label("Keep this many spare:");
                    changed |= ui.add(egui::DragValue::new(&mut r.min_spare).range(0..=50)).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Don't share above (platinum, 0 = off):");
                    changed |= ui.add(egui::DragValue::new(&mut r.max_price).range(0..=5000)).changed();
                });
                ui.add_space(4.0);
                list_editor(ui, "Always share (name contains)", &mut app.settings.always_share, &mut app.share_query, &mut changed);
                ui.add_space(4.0);
                list_editor(ui, "Never share (name contains)", &mut app.settings.never_share, &mut app.never_query, &mut changed);
            });
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                heading(ui, "Preview: what a friend sees as giveable");
                subtle(ui, &format!("{} item types", app.my_avail.len()));
                for a in app.my_avail.iter().take(25) {
                    ui.label(format!("• {} ×{}", a.name, a.count));
                }
                if app.my_avail.len() > 25 {
                    subtle(ui, &format!("…and {} more", app.my_avail.len() - 25));
                }
                ui.add_space(4.0);
                if ui.button("Export snapshot to a file…").clicked() {
                    export = true;
                }
            });
        });
    });
    if let Some(p) = load_snap {
        app.load_friend_snapshot(p);
    }
    if export {
        let name = crate::share::snapshot_file_name(&app.settings.my_name);
        if let Some(p) = rfd::FileDialog::new().set_file_name(name).save_file() {
            if let Err(e) = app.export_snapshot(&p) {
                app.snap_error = Some(e);
            }
        }
    }
    if changed {
        app.settings_changed();
    }
}

fn list_editor(ui: &mut egui::Ui, title: &str, list: &mut Vec<String>, input: &mut String, changed: &mut bool) {
    ui.label(RichText::new(title).strong());
    let mut remove = None;
    ui.horizontal_wrapped(|ui| {
        for (i, x) in list.iter().enumerate() {
            if ui.small_button(format!("{x}  ✕")).clicked() {
                remove = Some(i);
            }
        }
    });
    if let Some(i) = remove {
        list.remove(i);
        *changed = true;
    }
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(input).desired_width(180.0));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (ui.button("Add").clicked() || enter) && !input.trim().is_empty() {
            list.push(input.trim().to_string());
            input.clear();
            *changed = true;
        }
    });
}
