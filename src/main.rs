#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod catalog;
mod compare;
mod decode;
mod icons;
mod trade;
mod update;

use catalog::Catalog;
use compare::{diff, flatten, Inventory, Row};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use icons::Icons;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use trade::{Kind, Plan, Progress, Status};

#[derive(PartialEq, Clone, Copy)]
enum NeedMode {
    Both,
    OnlyA,
    OnlyB,
    Any,
}

const BG: Color32 = Color32::from_rgb(32, 32, 32);
const CARD: Color32 = Color32::from_rgb(43, 43, 43);
const CARD_HI: Color32 = Color32::from_rgb(55, 55, 55);
const STROKE: Color32 = Color32::from_rgb(64, 64, 64);
const ACCENT: Color32 = Color32::from_rgb(96, 205, 255);
const TEXT: Color32 = Color32::from_rgb(255, 255, 255);
const SUBTLE: Color32 = Color32::from_rgb(170, 170, 170);
const GOOD: Color32 = Color32::from_rgb(108, 203, 95);
const WARN: Color32 = Color32::from_rgb(252, 225, 0);
const BAD: Color32 = Color32::from_rgb(255, 153, 164);

fn apply_fluent_theme(ctx: &egui::Context) {
    // Use Segoe UI (the WinUI font) when running on Windows.
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("segoe".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "segoe".into());
        ctx.set_fonts(fonts);
    }
    let mut v = egui::Visuals::dark();
    let r = CornerRadius::same(6);
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(28, 28, 28);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STROKE);
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, STROKE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(66, 66, 66);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(66, 66, 66);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, STROKE);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(10.0, 10.0);
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.spacing.interact_size.y = 32.0;
    });
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, STROKE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add)
        .inner
}

fn accent_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).color(Color32::BLACK).strong())
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(6))
}

struct Slot {
    label: String,
    path: Option<PathBuf>,
    inv: Option<Inventory>,
    prog: Progress,
    error: Option<String>,
}

impl Slot {
    fn new(label: &str) -> Self {
        Self { label: label.into(), path: None, inv: None, prog: Progress::default(), error: None }
    }
    fn load(&mut self, path: PathBuf) {
        match decode::load(&path) {
            Ok(v) => {
                let inv = flatten(&v);
                self.prog = trade::progress(&v);
                if inv.is_empty() {
                    self.inv = None;
                    self.error = Some("Decrypted fine, but no items were found in it.".into());
                } else {
                    self.inv = Some(inv);
                    self.error = None;
                }
            }
            Err(e) => {
                self.inv = None;
                self.error = Some(e);
            }
        }
        self.path = Some(path);
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    All,
    OnlyA,
    OnlyB,
    MoreA,
    MoreB,
    Same,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Trade,
    Needs,
    Compare,
}

#[derive(Clone)]
enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(update::Release),
    Installing,
    Error(String),
}

struct App {
    slots: [Slot; 2],
    catalog: Catalog,
    icons: Icons,
    plan: Option<Plan>,
    tab: Tab,
    rows: Vec<Row>,
    categories: Vec<String>,
    category: Option<String>,
    mode: Mode,
    search: String,
    need_mode: NeedMode,
    started_only: bool,
    update: Arc<Mutex<UpdateState>>,
}

/// Draws an item icon (or an empty placeholder) and queues its download only
/// when it is actually on screen.
fn icon(ui: &mut egui::Ui, icons: &mut Icons, image: Option<&str>, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(36, 36, 36));
    if let Some(img) = image.filter(|_| ui.is_rect_visible(rect)) {
        let ctx = ui.ctx().clone();
        if let Some(tex) = icons.get(&ctx, img) {
            egui::Image::from_texture((tex.id(), rect.size())).paint_at(ui, rect.shrink(2.0));
        }
    }
}

fn chip(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| ui.label(RichText::new(text).color(color).size(12.0)));
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        let app = Self {
            slots: [Slot::new("Player 1"), Slot::new("Player 2")],
            catalog: Catalog::load(),
            icons: Icons::new(ctx),
            plan: None,
            tab: Tab::Trade,
            rows: vec![],
            categories: vec![],
            category: None,
            mode: Mode::OnlyB,
            search: String::new(),
            need_mode: NeedMode::Both,
            started_only: false,
            update: Arc::new(Mutex::new(UpdateState::Idle)),
        };
        app.check_updates(ctx);
        app
    }

    fn check_updates(&self, ctx: &egui::Context) {
        {
            let mut st = self.update.lock().unwrap();
            if matches!(*st, UpdateState::Checking | UpdateState::Installing) {
                return;
            }
            *st = UpdateState::Checking;
        }
        let (st, ctx) = (self.update.clone(), ctx.clone());
        std::thread::spawn(move || {
            let res = match update::check() {
                Ok(Some(r)) => UpdateState::Available(r),
                Ok(None) => UpdateState::UpToDate,
                Err(e) => UpdateState::Error(e),
            };
            *st.lock().unwrap() = res;
            ctx.request_repaint();
        });
    }

    fn install_update(&self, rel: update::Release, ctx: &egui::Context) {
        *self.update.lock().unwrap() = UpdateState::Installing;
        let (st, ctx) = (self.update.clone(), ctx.clone());
        std::thread::spawn(move || {
            // On success this restarts the app and never returns.
            if let Err(e) = update::install(&rel) {
                *st.lock().unwrap() = UpdateState::Error(e);
                ctx.request_repaint();
            }
        });
    }

    fn pick(&mut self, i: usize) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("AlecaFrame data", &["dat", "json"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.slots[i].load(p);
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.rows.clear();
        self.plan = None;
        if let (Some(a), Some(b)) = (&self.slots[0].inv, &self.slots[1].inv) {
            self.rows = diff(a, b);
            for r in &mut self.rows {
                if let Some(n) = self.catalog.name(&r.path) {
                    r.name = n.to_string();
                }
            }
            self.rows.sort_by(|x, y| x.category.cmp(&y.category).then(x.name.cmp(&y.name)));
            self.plan = Some(trade::plan(&self.catalog, [a, b], [&self.slots[0].prog, &self.slots[1].prog]));
        }
        let mut cats: Vec<String> = self.rows.iter().map(|r| r.category.clone()).collect();
        cats.sort();
        cats.dedup();
        self.categories = cats;
        if self.category.as_ref().is_some_and(|c| !self.categories.contains(c)) {
            self.category = None;
        }
    }

    fn matches(&self, r: &Row) -> bool {
        let ok = match self.mode {
            Mode::All => true,
            Mode::OnlyA => r.a > 0 && r.b == 0,
            Mode::OnlyB => r.b > 0 && r.a == 0,
            Mode::MoreA => r.a > r.b && r.b > 0,
            Mode::MoreB => r.b > r.a && r.a > 0,
            Mode::Same => r.a == r.b,
        };
        ok && self.category.as_ref().is_none_or(|c| &r.category == c)
            && (self.search.is_empty()
                || r.name.to_lowercase().contains(&self.search.to_lowercase())
                || r.path.to_lowercase().contains(&self.search.to_lowercase()))
    }

    fn slot_card(&mut self, ui: &mut egui::Ui, i: usize) {
        let mut pick = false;
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(120.0);
            let s = &mut self.slots[i];
            ui.label(RichText::new(if i == 0 { "INVENTORY A" } else { "INVENTORY B" }).size(11.0).color(SUBTLE));
            ui.add(egui::TextEdit::singleline(&mut s.label).font(egui::TextStyle::Heading).frame(egui::Frame::NONE));
            match (&s.inv, &s.error, &s.path) {
                (Some(inv), _, Some(p)) => {
                    let total: u64 = inv.values().map(|e| e.count).sum();
                    ui.label(RichText::new(format!("✔  {} unique items · {} total", inv.len(), total)).color(GOOD));
                    ui.label(RichText::new(p.file_name().unwrap_or_default().to_string_lossy()).color(SUBTLE).size(12.0));
                }
                (_, Some(e), _) => {
                    ui.label(RichText::new(format!("⚠  {e}")).color(BAD));
                }
                _ => {
                    ui.label(RichText::new("Drop a lastData.dat here, or browse").color(SUBTLE));
                }
            }
            if ui.add(accent_button("Open file…")).clicked() {
                pick = true;
            }
        });
        if pick {
            self.pick(i);
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let state = self.update.lock().unwrap().clone();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Inventory Compare").size(28.0).strong());
            ui.label(RichText::new(format!("v{}", update::current_version())).color(SUBTLE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match &state {
                UpdateState::Available(r) => {
                    if ui.add(accent_button(&format!("⬇  Update to {}", r.tag))).clicked() {
                        let ctx = ui.ctx().clone();
                        self.install_update(r.clone(), &ctx);
                    }
                    ui.hyperlink_to("What's new", &r.page);
                }
                UpdateState::Checking => {
                    ui.label(RichText::new("Checking for updates…").color(SUBTLE));
                }
                UpdateState::Installing => {
                    ui.label(RichText::new("Installing update… the app will restart").color(WARN));
                }
                UpdateState::Error(e) => {
                    if ui.button("Retry").clicked() {
                        let ctx = ui.ctx().clone();
                        self.check_updates(&ctx);
                    }
                    ui.label(RichText::new(format!("⚠ {e}")).color(BAD).size(12.0));
                }
                UpdateState::UpToDate | UpdateState::Idle => {
                    if ui.button("Check for updates").clicked() {
                        let ctx = ui.ctx().clone();
                        self.check_updates(&ctx);
                    }
                    if matches!(state, UpdateState::UpToDate) {
                        ui.label(RichText::new("✔ Up to date").color(GOOD).size(12.0));
                    }
                }
            });
        });
    }

    /// The simple answer: what each player should hand to the other.
    fn trade_view(&mut self, ui: &mut egui::Ui) {
        let Some(plan) = &self.plan else { return };
        let cat = &self.catalog;
        let icons = &mut self.icons;
        let names = [self.slots[0].label.clone(), self.slots[1].label.clone()];
        let count = |from: usize| plan.transfers.iter().filter(|t| t.from == from).map(|t| t.count).sum::<u32>();
        let (give_a, give_b) = (count(0), count(1));

        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("🔍  Filter by item…").desired_width(240.0));
        let q = self.search.to_lowercase();

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

    fn needs_view(&mut self, ui: &mut egui::Ui) {
        enum Line {
            Header(usize),
            Part(usize, usize),
        }
        let Some(plan) = &self.plan else { return };
        let cat = &self.catalog;
        let icons = &mut self.icons;
        let names = [self.slots[0].label.clone(), self.slots[1].label.clone()];
        let (mode, started_only) = (self.need_mode, self.started_only);
        let q = self.search.to_lowercase();

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
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("🔍  Filter primes / parts…").desired_width(240.0));
            ui.selectable_value(&mut self.need_mode, NeedMode::Both, format!("Both need ({both})"));
            ui.selectable_value(&mut self.need_mode, NeedMode::OnlyA, format!("Only {} needs ({})", names[0], only[0]));
            ui.selectable_value(&mut self.need_mode, NeedMode::OnlyB, format!("Only {} needs ({})", names[1], only[1]));
            ui.selectable_value(&mut self.need_mode, NeedMode::Any, "Everything");
            ui.checkbox(&mut self.started_only, "Only primes someone has started");
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

    fn compare_view(&mut self, ui: &mut egui::Ui) {
        let (na, nb) = (self.slots[0].label.clone(), self.slots[1].label.clone());
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("🔍  Have you got…?").desired_width(240.0));
            let modes = [
                (Mode::OnlyB, format!("Only {nb} has")),
                (Mode::OnlyA, format!("Only {na} has")),
                (Mode::MoreB, format!("{nb} has more")),
                (Mode::MoreA, format!("{na} has more")),
                (Mode::Same, "Same".to_string()),
                (Mode::All, "Everything".to_string()),
            ];
            for (m, label) in modes {
                ui.selectable_value(&mut self.mode, m, label);
            }
            egui::ComboBox::from_id_salt("cat")
                .selected_text(self.category.clone().unwrap_or_else(|| "All categories".into()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category, None, "All categories");
                    for c in self.categories.clone() {
                        ui.selectable_value(&mut self.category, Some(c.clone()), c);
                    }
                });
        });

        let visible: Vec<&Row> = self.rows.iter().filter(|r| self.matches(r)).collect();
        ui.label(RichText::new(format!("{} items", visible.len())).color(SUBTLE));

        let catalog = &self.catalog;
        let icons = &mut self.icons;
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
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Drag & drop: first file fills the first empty slot, second the next.
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if !dropped.is_empty() {
            for p in dropped {
                let idx = if self.slots[0].inv.is_none() { 0 } else { 1 };
                self.slots[idx].load(p);
            }
            self.rebuild();
        }

        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(24))).show(ui, |ui| {
            self.header(ui);

            ui.columns(2, |cols| {
                self.slot_card(&mut cols[0], 0);
                self.slot_card(&mut cols[1], 1);
            });

            if self.rows.is_empty() {
                if self.slots.iter().all(|s| s.inv.is_some()) {
                    ui.label("Nothing to compare.");
                } else {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Load two inventories to see who has what.").color(SUBTLE).size(16.0));
                }
                return;
            }

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Trade, "🔁  Trades");
                ui.selectable_value(&mut self.tab, Tab::Needs, "📋  Who needs what");
                ui.selectable_value(&mut self.tab, Tab::Compare, "⚖  Full compare");
            });
            match self.tab {
                Tab::Trade => self.trade_view(ui),
                Tab::Needs => self.needs_view(ui),
                Tab::Compare => self.compare_view(ui),
            }
        });
    }
}

fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?.to_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1000.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title("Inventory Compare")
        .with_drag_and_drop(true);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    eframe::run_native(
        "Inventory Compare",
        eframe::NativeOptions { viewport, ..Default::default() },
        Box::new(|cc| {
            apply_fluent_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}
