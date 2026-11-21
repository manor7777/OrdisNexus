use crate::app::{App, Tab};
use crate::theme::*;
use crate::util::{alecaframe_data_file, fmt_duration, fmt_n, now_ms};
use eframe::egui::{self, RichText};
use std::path::PathBuf;

enum Act {
    LoadMe(PathBuf),
    ReloadMe,
    LoadFriendFile(PathBuf),
    LoadSnapshot(PathBuf),
    ClearNotices,
}

fn stat(ui: &mut egui::Ui, label: &str, value: String) {
    card(ui, |ui| {
        ui.set_min_width(110.0);
        ui.label(RichText::new(label).size(11.0).color(SUBTLE));
        ui.label(RichText::new(value).size(20.0).strong());
    });
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut act: Option<Act> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.columns(2, |cols| {
            // ---- you
            card(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                subtle(ui, "YOUR INVENTORY");
                ui.label(RichText::new(&app.settings.my_name).size(20.0).strong());
                match (&app.me.acct, &app.me.error, &app.me.path) {
                    (Some(a), _, Some(p)) => {
                        ui.label(RichText::new(format!("✔  {} unique items · {} total", a.inv.len(), fmt_n(a.inv.values().map(|e| e.count).sum()))).color(GOOD));
                        subtle(ui, &p.to_string_lossy());
                    }
                    (_, Some(e), _) => {
                        ui.label(RichText::new(format!("⚠  {e}")).color(BAD));
                    }
                    _ => {
                        ui.label(RichText::new("No inventory loaded yet. AlecaFrame keeps it at %LOCALAPPDATA%\\AlecaFrame\\lastData.dat").color(SUBTLE));
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    if let Some(p) = alecaframe_data_file().filter(|p| p.exists()) {
                        if ui.add(accent_button("Use AlecaFrame's file")).clicked() {
                            act = Some(Act::LoadMe(p));
                        }
                    }
                    if ui.button("Browse…").clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("AlecaFrame data", &["dat", "json"]).add_filter("All files", &["*"]).pick_file() {
                            act = Some(Act::LoadMe(p));
                        }
                    }
                    if app.me.path.is_some() && ui.button("Reload").clicked() {
                        act = Some(Act::ReloadMe);
                    }
                });
                if app.settings.auto_reload && app.me.path.is_some() {
                    subtle(ui, "Reloads automatically when AlecaFrame updates the file.");
                }
            });
            // ---- friend
            card(&mut cols[1], |ui| {
                ui.set_width(ui.available_width());
                subtle(ui, "YOUR FRIEND");
                ui.label(RichText::new(&app.settings.friend_name).size(20.0).strong());
                match &app.friend_snap {
                    Some(s) => {
                        ui.label(RichText::new(format!("✔  Snapshot from {} · {} ago", s.name, fmt_duration(now_ms() - s.created_ms))).color(GOOD));
                    }
                    None => {
                        subtle(ui, "No snapshot yet. Ask them to share theirs, or set a shared folder in Settings.");
                    }
                }
                if let Some(a) = &app.friend_file.acct {
                    ui.label(RichText::new(format!("✔  Full inventory loaded ({} items)", a.inv.len())).color(GOOD));
                } else if let Some(e) = &app.friend_file.error {
                    ui.label(RichText::new(format!("⚠  {e}")).color(BAD));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.add(accent_button("Open snapshot…")).clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("Nexus snapshot", &["json"]).pick_file() {
                            act = Some(Act::LoadSnapshot(p));
                        }
                    }
                    if ui.button("Open their lastData.dat…").clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("AlecaFrame data", &["dat", "json"]).pick_file() {
                            act = Some(Act::LoadFriendFile(p));
                        }
                    }
                });
                subtle(ui, "You can also drag & drop files onto the window.");
            });
        });

        if let Some(me) = &app.me.acct {
            ui.add_space(4.0);
            let ready = app.relic_rows.iter().filter(|r| r.useful).count();
            let done = app.settings.goals.iter().filter(|g| crate::goals::status(&app.cat, me, g).done).count();
            let foundry_ready = me.foundry.iter().filter(|j| j.done_ms <= now_ms()).count();
            ui.horizontal_wrapped(|ui| {
                stat(ui, "MASTERY RANK", me.mr.to_string());
                stat(ui, "CREDITS", fmt_n(me.credits));
                stat(ui, "PLATINUM", fmt_n(me.platinum));
                stat(ui, "ENDO", fmt_n(me.endo));
                stat(ui, "GOALS DONE", format!("{done} / {}", app.settings.goals.len()));
                stat(ui, "FOUNDRY", format!("{} jobs · {foundry_ready} ready", me.foundry.len()));
                stat(ui, "USEFUL RELICS", ready.to_string());
            });
        }

        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            heading(ui, "What should I do tonight?");
            if app.me.acct.is_none() {
                subtle(ui, "Load your inventory and add a goal, and Ordis will plan around them.");
            } else if app.recs.is_empty() {
                subtle(ui, "Nothing to recommend yet. Add goals in the Goals tab.");
            } else {
                for r in &app.recs {
                    ui.horizontal_wrapped(|ui| {
                        chip(ui, &r.score.to_string(), ACCENT);
                        ui.label(RichText::new(&r.title).strong());
                    });
                    ui.label(RichText::new(&r.detail).color(SUBTLE).size(13.0));
                    ui.add_space(2.0);
                }
            }
        });

        ui.add_space(6.0);
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                heading(ui, "Notifications");
                if !app.notices.is_empty() && ui.small_button("Clear").clicked() {
                    act = Some(Act::ClearNotices);
                }
            });
            if app.notices.is_empty() {
                subtle(ui, "Nothing yet. Goal completions, mission summaries and friend updates show up here.");
            }
            for n in app.notices.iter().rev().take(12) {
                ui.label(format!("{}  ·  {} ago", n.text, fmt_duration(now_ms() - n.ts)));
            }
        });
    });

    match act {
        Some(Act::LoadMe(p)) => app.load_me(p),
        Some(Act::ReloadMe) => app.reload_me(),
        Some(Act::LoadFriendFile(p)) => app.load_friend_file(p),
        Some(Act::LoadSnapshot(p)) => app.load_friend_snapshot(p),
        Some(Act::ClearNotices) => app.notices.clear(),
        None => {}
    }
    let _ = Tab::Overview;
}
