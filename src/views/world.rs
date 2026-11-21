use crate::app::App;
use crate::theme::*;
use crate::util::{fmt_duration, now_ms};
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let w = app.world.snapshot();
    ui.horizontal(|ui| {
        if ui.button("🔄 Refresh").clicked() {
            app.world.refresh(ui.ctx());
        }
        if w.fetched_ms > 0 {
            subtle(ui, &format!("updated {} ago", fmt_duration((now_ms() - w.fetched_ms).max(0))));
        }
        if let Some(e) = &w.error {
            ui.colored_label(BAD, e);
        }
    });
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.columns(2, |cols| {
            card(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                heading(ui, "Open world cycles");
                for c in &w.cycles {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&c.name).strong());
                        chip(ui, &c.state, ACCENT);
                        subtle(ui, &format!("{} left", fmt_duration((c.expiry_ms - now_ms()).max(0))));
                    });
                }
                ui.add_space(6.0);
                heading(ui, "Daily & weekly");
                if let Some(b) = &w.baro {
                    ui.label(format!("🛒 Baro: {b}"));
                }
                if let Some(s) = &w.steel_path {
                    ui.label(format!("🛡 Steel Path: {s}"));
                }
                if !w.sortie.is_empty() {
                    ui.label(RichText::new("Sortie").strong());
                    for l in &w.sortie {
                        ui.label(format!("• {l}"));
                    }
                }
                if !w.archon.is_empty() {
                    ui.label(RichText::new("Archon hunt").strong());
                    for l in &w.archon {
                        ui.label(format!("• {l}"));
                    }
                }
                if !w.nightwave.is_empty() {
                    ui.label(RichText::new("Nightwave").strong());
                    for l in &w.nightwave {
                        ui.label(format!("• {l}"));
                    }
                }
            });
            card(&mut cols[1], |ui| {
                ui.set_width(ui.available_width());
                heading(ui, "Void fissures");
                subtle(ui, "Handy for cracking relics.");
                for f in &w.fissures {
                    ui.horizontal(|ui| {
                        chip(ui, &f.tier, if f.hard { WARN } else { ACCENT });
                        ui.label(format!("{} · {}", f.node, f.mission));
                        if f.storm {
                            chip(ui, "railjack", SUBTLE);
                        }
                        if f.hard {
                            chip(ui, "steel path", WARN);
                        }
                        subtle(ui, &fmt_duration((f.expiry_ms - now_ms()).max(0)));
                    });
                }
                if w.fissures.is_empty() {
                    subtle(ui, "No data yet.");
                }
                ui.add_space(8.0);
                heading(ui, "Invasions");
                for i in &w.invasions {
                    ui.label(format!("{} — {} vs {} ({:.0}%)", i.node, i.attacker, i.defender, i.completion));
                    ui.label(RichText::new(&i.desc).color(SUBTLE).size(12.0));
                }
            });
        });
    });
}
