use crate::app::App;
use crate::theme::*;
use crate::util::{default_ee_log, now_ms};
use eframe::egui::{self, RichText};

fn ago(ms: i64) -> String {
    if ms <= 0 {
        "never".into()
    } else {
        format!("{} ago", crate::util::fmt_duration((now_ms() - ms).max(0)))
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut toggle = false;
    card(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            heading(ui, "Overwolf bridge");
            match &app.gep {
                None => chip(ui, "off (Settings)", SUBTLE),
                Some(g) => {
                    let st = g.lock().unwrap().clone();
                    if let Some(e) = &st.error {
                        chip(ui, "problem", BAD);
                        ui.colored_label(BAD, e);
                    } else if st.last_contact_ms > 0 && now_ms() - st.last_contact_ms < 45_000 {
                        chip(ui, "connected", GOOD);
                    } else if st.listening {
                        chip(ui, "waiting for the Overwolf app", WARN);
                    }
                }
            }
        });
        match &app.gep {
            Some(g) => {
                let st = g.lock().unwrap().clone();
                subtle(
                    ui,
                    &format!(
                        "Inventory updates received: {} (last {}). Other updates: {} (last {}). Warframe {}.",
                        st.inventory_updates,
                        ago(st.last_inventory_ms),
                        st.updates.saturating_sub(st.inventory_updates),
                        ago(st.last_update_ms),
                        if st.game_running { "is running" } else { "not detected" }
                    ),
                );
                if st.partial_updates > 0 {
                    ui.colored_label(WARN, format!("{} incomplete inventory update(s) were ignored so they can't wipe your data.", st.partial_updates));
                    if let Some(k) = &st.last_partial_keys {
                        subtle(ui, &format!("Last one contained only: {k}"));
                    }
                }
                if let Some(u) = &st.username {
                    subtle(ui, &format!("In-game name reported by Overwolf: {u}"));
                }
                subtle(ui, "Warframe only sends a fresh inventory occasionally (often after leaving the Relay or Orbiter), so right after a mission it can be a little behind.");
                if !st.recent.is_empty() {
                    ui.collapsing("Recent bridge messages", |ui| {
                        for l in st.recent.iter().rev().take(20) {
                            ui.label(RichText::new(l).monospace().size(11.0).color(SUBTLE));
                        }
                    });
                }
            }
            None => {
                subtle(ui, "Turn on the Overwolf bridge in Settings to receive your live inventory from the Overwolf app.");
            }
        }
    });
    ui.add_space(6.0);
    card(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            heading(ui, "Live game tracking");
            if app.ee.is_some() {
                chip(ui, if app.mission_active { "in mission" } else { "watching" }, if app.mission_active { WARN } else { GOOD });
            } else {
                chip(ui, "off", SUBTLE);
            }
        });
        subtle(ui, "Reads Warframe's EE.log as the game writes it. It's read-only and needs nothing installed. See the README for how to point it at the right file.");
        let path = app.settings.ee_log_path.clone().map(std::path::PathBuf::from).or_else(default_ee_log);
        subtle(ui, &format!("Log file: {}", path.map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "not found (Windows only by default)".into())));
        if let Some(ee) = &app.ee {
            subtle(ui, &ee.status.lock().map(|s| s.clone()).unwrap_or_default());
        }
        let label = if app.ee.is_some() { "Stop" } else { "Start watching" };
        if ui.add(accent_button(label)).clicked() {
            toggle = true;
        }
    });
    if toggle {
        if app.ee.is_some() {
            app.stop_live();
            app.settings.live_enabled = false;
        } else {
            let ctx = ui.ctx().clone();
            app.start_live(&ctx);
            app.settings.live_enabled = true;
        }
        app.settings.save();
    }

    ui.add_space(6.0);
    ui.columns(2, |cols| {
        card(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Events");
            if app.pending_summary {
                subtle(ui, "Waiting for AlecaFrame to refresh your inventory so Ordis can summarise the mission…");
            }
            egui::ScrollArea::vertical().id_salt("events").max_height(420.0).auto_shrink([false, false]).show(ui, |ui| {
                if app.events.is_empty() {
                    subtle(ui, "Nothing yet. Start a mission.");
                }
                for e in app.events.iter().rev() {
                    let ago = ((now_ms() - e.ts) / 1000).max(0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&e.text);
                        ui.label(RichText::new(format!("{ago}s ago")).color(SUBTLE).size(11.0));
                    });
                }
            });
        });
        card(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "Raw log (latest lines)");
            ui.add(egui::TextEdit::singleline(&mut app.log_filter).hint_text("filter…").desired_width(f32::INFINITY));
            let f = app.log_filter.to_lowercase();
            let lines: Vec<String> = app
                .ee
                .as_ref()
                .map(|e| e.lines.lock().map(|l| l.iter().rev().filter(|x| f.is_empty() || x.to_lowercase().contains(&f)).take(200).cloned().collect()).unwrap_or_default())
                .unwrap_or_default();
            egui::ScrollArea::vertical().id_salt("raw").max_height(380.0).auto_shrink([false, false]).show(ui, |ui| {
                for l in lines {
                    ui.label(RichText::new(l).monospace().size(11.0).color(SUBTLE));
                }
            });
            subtle(ui, "Tip: if missions aren't detected, find the real lines here and edit the patterns in Settings.");
        });
    });
}
