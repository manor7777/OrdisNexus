use crate::app::{App, CmpTab, Mode, NeedMode};
use crate::compare::Row;
use crate::theme::*;
use crate::trade::{Kind, Status};
use eframe::egui::{self, RichText};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let names = app.names();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{} vs {}", names[0], names[1])).size(18.0).strong());
        if ui.button("📂 Load friend's lastData.dat…").clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter("AlecaFrame data", &["dat"]).pick_file() {
                app.settings.friend_path = Some(p.to_string_lossy().into_owned());
                app.settings.save();
                app.load_friend_file(p);
            }
        }
        if let Some(e) = &app.friend_file.error {
            ui.colored_label(BAD, e);
        }
    });
    if app.plan.is_none() {
        ui.add_space(8.0);
        subtle(ui, "Load both inventories (yours on Overview, your friend's raw lastData.dat above) for the full side-by-side trade planner. Without their file, the Friends tab works from their shared snapshot.");
        return;
    }
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.cmp_tab, CmpTab::Trade, "Trades");
        ui.selectable_value(&mut app.cmp_tab, CmpTab::Needs, "Still needed");
        ui.selectable_value(&mut app.cmp_tab, CmpTab::Full, "Full compare");
    });
    ui.separator();
    match app.cmp_tab {
        CmpTab::Trade => trade_view(app, ui),
        CmpTab::Needs => needs_view(app, ui),
        CmpTab::Full => compare_view(app, ui),
    }
}

fn matches(app: &App, r: &Row) -> bool {
    let ok = match app.mode {
        Mode::All => true,
        Mode::OnlyA => r.a > 0 && r.b == 0,
        Mode::OnlyB => r.b > 0 && r.a == 0,
        Mode::MoreA => r.a > r.b && r.b > 0,
        Mode::MoreB => r.b > r.a && r.a > 0,
        Mode::Same => r.a == r.b,
    };
    ok && app.category.as_ref().is_none_or(|c| &r.category == c)
        && (app.search.is_empty()
            || r.name.to_lowercase().contains(&app.search.to_lowercase())
            || r.path.to_lowercase().contains(&app.search.to_lowercase()))
}

fn trade_view(app: &mut App, ui: &mut egui::Ui) {
    let Some(plan) = &app.plan else { return };
    let cat = &app.cat;
    let names = app.names();
    let icons = &mut app.icons;
    let count = |from: usize| plan.transfers.iter().filter(|t| t.from == from).map(|t| t.count).sum::<u32>();
    let (give_a, give_b) = (count(0), count(1));

    ui.add(egui::TextEdit::singleline(&mut app.search).hint_text("🔍  Filter by item…").desired_width(240.0));
    let q = app.search.to_lowercase();

    ui.columns(2, |cols| {
        for (from, ui) in cols.iter_mut().enumerate() {
            let to = 1 - from;
            let total = if from == 0 { give_a } else { give_b };
            card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("{} gives {}", names[from], names[to])).size(20.0).strong());
                ui.label(
                    RichText::new(format!("{total} part{} {} still needs", if total == 1 { "" } else { "s" }, names[to]))
                        .color(SUBTLE),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt(("give", from)).auto_shrink([false, false]).show(ui, |ui| {
                    let mut last = usize::MAX;
                    let mut any = false;
                    for t in plan.transfers.iter().filter(|t| t.from == from) {
                        let prime = &cat.primes[t.prime];
                        if !q.is_empty() && !prime.name.to_lowercase().contains(&q) {
                            continue;
                        }
                        any = true;
                        let part = &prime.parts[t.part];
                        if last != t.prime {
                            last = t.prime;
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                icon(ui, icons, cat.icon(&prime.unique), 34.0);
                                ui.label(RichText::new(&prime.name).strong().size(16.0).color(ACCENT));
                            });
                        }
                        let short = part.name.strip_prefix(prime.name.as_str()).unwrap_or(&part.name).trim();
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            icon(ui, icons, cat.icon(&part.unique), 28.0);
                            ui.label(RichText::new(format!("{short}  ×{}", t.count)).size(15.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match t.kind {
                                Kind::Free => chip(ui, "safe – they've mastered it", GOOD),
                                Kind::YourCall => chip(ui, "spare copy", WARN),
                            });
                        });
                    }
                    if !any {
                        ui.label(RichText::new("Nothing to hand over.").color(SUBTLE));
                    }

                    // Extra copies that nobody needs: just sell them.
                    let sells: Vec<_> = plan
                        .sells
                        .iter()
                        .filter(|s| s.who == from)
                        .filter(|s| q.is_empty() || cat.primes[s.prime].name.to_lowercase().contains(&q))
                        .collect();
                    if !sells.is_empty() {
                        ui.add_space(12.0);
                        egui::CollapsingHeader::new(RichText::new(format!("Free to sell ({})", sells.len())).color(SUBTLE))
                            .id_salt(("sell", from))
                            .show(ui, |ui| {
                                for s in sells {
                                    let part = &cat.primes[s.prime].parts[s.part];
                                    ui.horizontal(|ui| {
                                        icon(ui, icons, cat.icon(&part.unique), 24.0);
                                        ui.label(format!("{}  ×{}", part.name, s.count));
                                    });
                                }
                            });
                    }
                });
            });
        }
    });
}

fn needs_view(app: &mut App, ui: &mut egui::Ui) {
    enum Line {
        Header(usize),
        Part(usize, usize),
    }
    let Some(plan) = &app.plan else { return };
    let cat = &app.cat;
    let names = app.names();
    let icons = &mut app.icons;
    let (mode, started_only) = (app.need_mode, app.started_only);
    let q = app.search.to_lowercase();

    let part_ok = |m: [u32; 2]| match mode {
        NeedMode::Both => m[0] > 0 && m[1] > 0,
        NeedMode::OnlyA => m[0] > 0 && m[1] == 0,
        NeedMode::OnlyB => m[1] > 0 && m[0] == 0,
        NeedMode::Any => m[0] > 0 || m[1] > 0,
    };
    let mut lines = Vec::new();
    let (mut both, mut only) = (0, [0, 0]);
    for (ni, n) in plan.needs.iter().enumerate() {
        let prime = &cat.primes[n.prime];
        let started = n.parts.iter().any(|p| p.have != [0, 0]);
        for p in &n.parts {
            if p.missing[0] > 0 && p.missing[1] > 0 {
                both += 1;
            } else if p.missing[0] > 0 {
                only[0] += 1;
            } else if p.missing[1] > 0 {
                only[1] += 1;
            }
        }
        if started_only && !started {
            continue;
        }
        let name_hit = q.is_empty() || prime.name.to_lowercase().contains(&q);
        let vis: Vec<usize> = (0..n.parts.len())
            .filter(|&i| part_ok(n.parts[i].missing))
            .filter(|&i| name_hit || cat.primes[n.prime].parts[n.parts[i].part].name.to_lowercase().contains(&q))
            .collect();
        if !vis.is_empty() {
            lines.push(Line::Header(ni));
            lines.extend(vis.into_iter().map(|i| Line::Part(ni, i)));
        }
    }

    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.search).hint_text("🔍  Filter primes / parts…").desired_width(240.0));
        ui.selectable_value(&mut app.need_mode, NeedMode::Both, format!("Both need ({both})"));
        ui.selectable_value(&mut app.need_mode, NeedMode::OnlyA, format!("Only {} needs ({})", names[0], only[0]));
        ui.selectable_value(&mut app.need_mode, NeedMode::OnlyB, format!("Only {} needs ({})", names[1], only[1]));
        ui.selectable_value(&mut app.need_mode, NeedMode::Any, "Everything");
        ui.checkbox(&mut app.started_only, "Only primes someone has started");
    });
    ui.label(
        RichText::new("Parts still missing to build each prime. Mastered or already-built primes aren't counted.")
            .color(SUBTLE)
            .size(12.0),
    );

    card(ui, |ui| {
        let w = ui.available_width();
        let cnum = 150.0;
        let cname = (w - 2.0 * cnum - 60.0).max(160.0);
        ui.horizontal(|ui| {
            ui.add_space(50.0);
            ui.add_sized([cname, 20.0], egui::Label::new(RichText::new("Part").color(SUBTLE).strong()));
            for n in &names {
                ui.add_sized([cnum, 20.0], egui::Label::new(RichText::new(n).color(SUBTLE).strong()).truncate());
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 40.0, lines.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for line in &lines[range] {
                ui.horizontal(|ui| {
                    ui.set_height(40.0);
                    match *line {
                        Line::Header(ni) => {
                            let n = &plan.needs[ni];
                            let prime = &cat.primes[n.prime];
                            icon(ui, icons, cat.icon(&prime.unique), 34.0);
                            ui.label(RichText::new(&prime.name).strong().size(16.0).color(ACCENT));
                            for i in 0..2 {
                                let (t, c) = match n.status[i] {
                                    Status::Mastered => ("mastered", GOOD),
                                    Status::Owned => ("built", WARN),
                                    Status::Missing => ("missing", BAD),
                                };
                                chip(ui, &format!("{}: {t}", names[i]), c);
                            }
                        }
                        Line::Part(ni, pi) => {
                            let n = &plan.needs[ni];
                            let pn = &n.parts[pi];
                            let part = &cat.primes[n.prime].parts[pn.part];
                            ui.add_space(14.0);
                            icon(ui, icons, cat.icon(&part.unique), 32.0);
                            ui.add_sized([cname - 14.0, 24.0], egui::Label::new(&part.name).truncate());
                            for i in 0..2 {
                                let (txt, col) = if pn.missing[i] > 0 {
                                    (format!("needs {}  ({}/{})", pn.missing[i], pn.have[i], part.need), BAD)
                                } else if n.status[i] == Status::Missing {
                                    (format!("✔ has {}/{}", pn.have[i], part.need), GOOD)
                                } else {
                                    ("— not needed".to_string(), SUBTLE)
                                };
                                ui.add_sized([cnum, 24.0], egui::Label::new(RichText::new(txt).color(col)));
                            }
                        }
                    }
                });
            }
        });
    });
}

fn compare_view(app: &mut App, ui: &mut egui::Ui) {
    let [na, nb] = app.names();
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.search).hint_text("🔍  Have you got…?").desired_width(240.0));
        let modes = [
            (Mode::OnlyB, format!("Only {nb} has")),
            (Mode::OnlyA, format!("Only {na} has")),
            (Mode::MoreB, format!("{nb} has more")),
            (Mode::MoreA, format!("{na} has more")),
            (Mode::Same, "Same".to_string()),
            (Mode::All, "Everything".to_string()),
        ];
        for (m, label) in modes {
            ui.selectable_value(&mut app.mode, m, label);
        }
        egui::ComboBox::from_id_salt("cat")
            .selected_text(app.category.clone().unwrap_or_else(|| "All categories".into()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.category, None, "All categories");
                for c in app.categories.clone() {
                    ui.selectable_value(&mut app.category, Some(c.clone()), c);
                }
            });
    });

    let visible: Vec<&Row> = app.rows.iter().filter(|r| matches(app, r)).collect();
    ui.label(RichText::new(format!("{} items", visible.len())).color(SUBTLE));

    let catalog = &app.cat;
    let icons = &mut app.icons;
    card(ui, |ui| {
        let w = ui.available_width();
        let (cname, ccat, cnum) = (w * 0.42, w * 0.2, 80.0);
        ui.horizontal(|ui| {
            ui.add_space(40.0);
            for (t, wd) in [("Item", cname), ("Category", ccat), (na.as_str(), cnum), (nb.as_str(), cnum)] {
                ui.add_sized([wd, 20.0], egui::Label::new(RichText::new(t).color(SUBTLE).strong()).truncate());
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 36.0, visible.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for r in &visible[range] {
                ui.horizontal(|ui| {
                    ui.set_height(36.0);
                    icon(ui, icons, catalog.icon(&r.path), 32.0);
                    ui.add_sized([cname, 24.0], egui::Label::new(&r.name).truncate()).on_hover_text(&r.path);
                    ui.add_sized([ccat, 24.0], egui::Label::new(RichText::new(&r.category).color(SUBTLE)).truncate());
                    let col = |n: u64, other: u64| if n == 0 { BAD } else if n > other { GOOD } else if n < other { WARN } else { TEXT };
                    ui.add_sized([cnum, 24.0], egui::Label::new(RichText::new(r.a.to_string()).color(col(r.a, r.b))));
                    ui.add_sized([cnum, 24.0], egui::Label::new(RichText::new(r.b.to_string()).color(col(r.b, r.a))));
                });
            }
        });
    });
}
