use crate::app::App;
use crate::theme::*;
use crate::util::{default_ee_log, now_ms};
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut toggle = false;
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
