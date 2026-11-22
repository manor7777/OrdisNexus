use crate::app::App;
use crate::settings::EePattern;
use crate::theme::*;
use eframe::egui::{self, RichText};

fn path_row(ui: &mut egui::Ui, label: &str, value: &mut Option<String>, folder: bool, changed: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut s = value.clone().unwrap_or_default();
        if ui.add(egui::TextEdit::singleline(&mut s).desired_width(380.0)).changed() {
            *value = if s.trim().is_empty() { None } else { Some(s) };
            *changed = true;
        }
        if ui.button("Browse…").clicked() {
            let p = if folder { rfd::FileDialog::new().pick_folder() } else { rfd::FileDialog::new().pick_file() };
            if let Some(p) = p {
                *value = Some(p.to_string_lossy().into_owned());
                *changed = true;
            }
        }
        if value.is_some() && ui.small_button("✕").clicked() {
            *value = None;
            *changed = true;
        }
    });
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut recheck = false;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "You & your friend");
            ui.horizontal(|ui| {
                ui.label("Your name");
                changed |= ui.text_edit_singleline(&mut app.settings.my_name).changed();
                ui.label("Friend's name");
                changed |= ui.text_edit_singleline(&mut app.settings.friend_name).changed();
            });
            path_row(ui, "Your lastData.dat (blank = AlecaFrame's default)", &mut app.settings.own_path, false, &mut changed);
            path_row(ui, "Friend's lastData.dat (optional)", &mut app.settings.friend_path, false, &mut changed);
            path_row(ui, "Shared sync folder (OneDrive/Dropbox/Syncthing)", &mut app.settings.sync_dir, true, &mut changed);
            subtle(ui, "Both of you point this at the same synced folder. Each writes a small snapshot file; no server is involved.");
            changed |= ui.checkbox(&mut app.settings.auto_reload, "Reload inventories automatically when the file changes").changed();
            ui.horizontal(|ui| {
                ui.label("Inventory source");
                egui::ComboBox::from_id_salt("inv_source")
                    .selected_text(match app.settings.inventory_source.as_str() {
                        "alecaframe" => "AlecaFrame file only",
                        "overwolf" => "Overwolf live inventory only",
                        _ => "Automatic (whichever is newest)",
                    })
                    .show_ui(ui, |ui| {
                        for (v, t) in [("auto", "Automatic (whichever is newest)"), ("alecaframe", "AlecaFrame file only"), ("overwolf", "Overwolf live inventory only")] {
                            changed |= ui.selectable_value(&mut app.settings.inventory_source, v.to_string(), t).changed();
                        }
                    });
            });
            ui.horizontal(|ui| {
                if ui.checkbox(&mut app.settings.overwolf_bridge, "Listen for the Overwolf bridge (restart to apply)").changed() {
                    changed = true;
                }
                subtle(ui, &format!("local port {}", crate::gep::PORT));
            });
        });
        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Notifications");
            let n = &mut app.settings.notify;
            changed |= ui.checkbox(&mut n.goals, "Goal completed").changed();
            changed |= ui.checkbox(&mut n.friends, "A friend now has something I need").changed();
            changed |= ui.checkbox(&mut n.events, "Mission started / ended").changed();
            changed |= ui.checkbox(&mut n.farming, "Farming tips").changed();
            changed |= ui.checkbox(&mut app.settings.use_market, "Look up platinum prices on warframe.market").changed();
        });
        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Live game (EE.log)");
            changed |= ui.checkbox(&mut app.settings.live_enabled, "Start watching the game log on launch").changed();
            path_row(ui, "EE.log path (blank = default)", &mut app.settings.ee_log_path, false, &mut changed);
            ui.add_space(4.0);
            ui.label(RichText::new("Patterns").strong());
            subtle(ui, "A log line containing the text triggers the event. kind: mission_start, mission_end or note.");
            let mut remove = None;
            for (i, p) in app.settings.ee_patterns.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(&mut p.label).desired_width(140.0)).changed();
                    changed |= ui.add(egui::TextEdit::singleline(&mut p.contains).desired_width(360.0)).changed();
                    egui::ComboBox::from_id_salt(("kind", i)).selected_text(p.kind.clone()).show_ui(ui, |ui| {
                        for k in ["mission_start", "mission_end", "mission_name", "credits", "note"] {
                            changed |= ui.selectable_value(&mut p.kind, k.to_string(), k).changed();
                        }
                    });
                    if ui.small_button("✕").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                app.settings.ee_patterns.remove(i);
                changed = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Add pattern").clicked() {
                    app.settings.ee_patterns.push(EePattern { label: "New".into(), contains: String::new(), kind: "note".into() });
                    changed = true;
                }
                if ui.button("Reset to defaults").clicked() {
                    app.settings.ee_patterns = crate::settings::default_patterns();
                    changed = true;
                }
            });
            subtle(ui, "Pattern changes apply the next time you start watching.");
        });
        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Ordis assistant");
            changed |= ui.checkbox(&mut app.settings.voice, "Speak answers aloud (Windows speech)").changed();
            let l = &mut app.settings.llm;
            changed |= ui.checkbox(&mut l.enabled, "Let a language model rephrase answers (optional)").changed();
            ui.horizontal(|ui| {
                ui.label("Endpoint");
                changed |= ui.add(egui::TextEdit::singleline(&mut l.endpoint).desired_width(320.0)).changed();
                ui.label("Model");
                changed |= ui.add(egui::TextEdit::singleline(&mut l.model).desired_width(140.0)).changed();
            });
            ui.horizontal(|ui| {
                ui.label("API key");
                changed |= ui.add(egui::TextEdit::singleline(&mut l.api_key).password(true).desired_width(320.0)).changed();
            });
            subtle(ui, "Any OpenAI-compatible server works, e.g. Ollama at http://localhost:11434/v1. Facts always come from your own data; the model only rewords.");
        });
        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, &format!("About · v{}", env!("CARGO_PKG_VERSION")));
            if ui.button("Check for updates").clicked() {
                recheck = true;
            }
            subtle(ui, &format!("Settings are stored in {}", crate::util::data_dir().display()));
        });
    });
    if recheck {
        let ctx = ui.ctx().clone();
        app.check_updates(&ctx);
    }
    if changed {
        app.settings_changed();
    }
}
