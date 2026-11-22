//! Application state, background work and the frame loop.

use crate::account::Account;
use crate::assistant::{self, Ctx};
use crate::catalog::Catalog;
use crate::compare::{diff, gained, Inventory, Row};
use crate::decode;
use crate::eelog::{self, EeWatcher};
use crate::goals::status;
use crate::insight::{buildable_now, mastery_rows, needed_parts, relic_rows, MasteryRow, RelicRow};
use crate::icons::Icons;
use crate::market::Market;
use crate::recommend::recommend;
use crate::settings::Settings;
use crate::share::{self, Avail, Matching, Need, Snapshot};
use crate::theme::*;
use crate::trade::{self, Plan};
use crate::update;
use crate::util::{alecaframe_data_file, default_ee_log, fmt_n, now_ms};
use crate::views;
use crate::worldstate::WorldFeed;
use eframe::egui::{self, RichText};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Overview,
    Ordis,
    Goals,
    Inventory,
    Foundry,
    Relics,
    Mastery,
    World,
    Friends,
    Compare,
    Live,
    Settings,
}

impl Tab {
    pub const ALL: [(Tab, &'static str); 12] = [
        (Tab::Overview, "🏠 Overview"),
        (Tab::Ordis, "💬 Ordis"),
        (Tab::Goals, "🎯 Goals"),
        (Tab::Inventory, "🎒 Inventory"),
        (Tab::Foundry, "🔨 Foundry"),
        (Tab::Relics, "🔮 Relics"),
        (Tab::Mastery, "⭐ Mastery"),
        (Tab::World, "🌍 World"),
        (Tab::Friends, "🤝 Friends"),
        (Tab::Compare, "⚖ Compare"),
        (Tab::Live, "📡 Live"),
        (Tab::Settings, "⚙ Settings"),
    ];
}

#[derive(PartialEq, Clone, Copy)]
pub enum CmpTab {
    Trade,
    Needs,
    Full,
}

#[derive(PartialEq, Clone, Copy)]
pub enum NeedMode {
    Both,
    OnlyA,
    OnlyB,
    Any,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    All,
    OnlyA,
    OnlyB,
    MoreA,
    MoreB,
    Same,
}

pub struct Slot {
    pub path: Option<PathBuf>,
    pub acct: Option<Account>,
    pub error: Option<String>,
    pub mtime: Option<SystemTime>,
}

impl Slot {
    pub fn new() -> Self {
        Self { path: None, acct: None, error: None, mtime: None }
    }

    pub fn load(&mut self, path: PathBuf) {
        self.mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        match decode::load(&path) {
            Ok(v) => {
                let a = Account::from_json(&v);
                if a.is_empty() {
                    self.acct = None;
                    self.error = Some("Decrypted fine, but no items were found in it.".into());
                } else {
                    self.acct = Some(a);
                    self.error = None;
                }
            }
            Err(e) => {
                self.acct = None;
                self.error = Some(e);
            }
        }
        self.path = Some(path);
    }

    fn changed_on_disk(&self) -> bool {
        match (&self.path, self.mtime) {
            (Some(p), Some(t)) => std::fs::metadata(p).and_then(|m| m.modified()).map(|m| m != t).unwrap_or(false),
            _ => false,
        }
    }
}

#[derive(Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(update::Release),
    Installing,
    Error(String),
}

pub struct InvRow {
    pub path: String,
    pub name: String,
    pub name_lower: String,
    pub category: String,
    pub count: u64,
}

pub struct Notice {
    pub ts: i64,
    pub text: String,
}

pub struct ChatMsg {
    pub user: bool,
    pub text: String,
    pub facts: String,
    pub pending: bool,
}

pub struct LiveEvent {
    pub ts: i64,
    pub text: String,
}

pub struct App {
    pub settings: Settings,
    pub cat: Catalog,
    pub icons: Icons,
    pub market: Market,
    pub world: WorldFeed,
    pub tab: Tab,

    pub me: Slot,
    pub friend_file: Slot,
    pub friend_snap: Option<Snapshot>,
    pub friend_snap_path: Option<PathBuf>,
    friend_snap_mtime: Option<SystemTime>,
    pub snap_error: Option<String>,
    pub my_avail: Vec<Avail>,
    pub my_needs: Vec<Need>,
    pub my_farming: Vec<Need>,
    pub matching: Option<Matching>,
    pub social_dirty: bool,
    last_social: Instant,
    last_snap_key: String,
    pub last_snap_written: Option<i64>,

    // compare (two full inventories)
    pub cmp_tab: CmpTab,
    pub plan: Option<Plan>,
    pub rows: Vec<Row>,
    pub categories: Vec<String>,
    pub category: Option<String>,
    pub mode: Mode,
    pub search: String,
    pub need_mode: NeedMode,
    pub started_only: bool,

    // generic view state
    pub inv_search: String,
    pub inv_category: Option<String>,
    pub inv_dupes: bool,
    pub mastery_search: String,
    pub mastery_filter: u8,
    pub mastery_category: Option<String>,
    pub relic_useful_only: bool,
    pub relic_search: String,
    pub goal_kind: u8,
    pub goal_query: String,
    pub goal_count: u64,
    pub friend_query: String,
    pub share_query: String,
    pub never_query: String,

    // chat
    pub chat: Vec<ChatMsg>,
    pub chat_input: String,
    llm_tx: Sender<(usize, Result<String, String>)>,
    llm_rx: Receiver<(usize, Result<String, String>)>,

    // live game
    pub ee: Option<EeWatcher>,
    pub gep: Option<crate::gep::Shared>,
    pub events: Vec<LiveEvent>,
    pub mission_active: bool,
    pub mission_name: Option<String>,
    pub pre_mission: Option<Inventory>,
    pub pending_summary: bool,
    pub log_filter: String,

    pub notices: Vec<Notice>,
    // derived, rebuilt when inventories/goals/settings change
    pub inv_rows: Vec<InvRow>,
    pub relic_rows: Vec<RelicRow>,
    pub mastery_rows: Vec<MasteryRow>,
    pub buildable: Vec<String>,
    pub recs: Vec<crate::recommend::Rec>,
    pub update: Arc<Mutex<UpdateState>>,
    last_poll: Instant,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let settings = Settings::load();
        let (llm_tx, llm_rx) = channel();
        let world = WorldFeed::new();
        world.refresh(ctx);
        let mut app = Self {
            cat: Catalog::load(),
            icons: Icons::new(ctx),
            market: Market::new(ctx),
            world,
            tab: Tab::Overview,
            me: Slot::new(),
            friend_file: Slot::new(),
            friend_snap: None,
            friend_snap_path: None,
            friend_snap_mtime: None,
            snap_error: None,
            my_avail: vec![],
            my_needs: vec![],
            my_farming: vec![],
            matching: None,
            social_dirty: true,
            last_social: Instant::now(),
            last_snap_key: String::new(),
            last_snap_written: None,
            cmp_tab: CmpTab::Trade,
            plan: None,
            rows: vec![],
            categories: vec![],
            category: None,
            mode: Mode::OnlyB,
            search: String::new(),
            need_mode: NeedMode::Both,
            started_only: false,
            inv_search: String::new(),
            inv_category: None,
            inv_dupes: false,
            mastery_search: String::new(),
            mastery_filter: 1,
            mastery_category: None,
            relic_useful_only: false,
            relic_search: String::new(),
            goal_kind: 0,
            goal_query: String::new(),
            goal_count: 100,
            friend_query: String::new(),
            share_query: String::new(),
            never_query: String::new(),
            chat: vec![ChatMsg {
                user: false,
                text: "Operator! Ordis is online. Ask me what you're missing, what to farm, or whether a friend can help.".into(),
                facts: String::new(),
                pending: false,
            }],
            chat_input: String::new(),
            llm_tx,
            llm_rx,
            ee: None,
            gep: None,
            events: vec![],
            mission_active: false,
            mission_name: None,
            pre_mission: None,
            pending_summary: false,
            log_filter: String::new(),
            notices: vec![],
            inv_rows: vec![],
            relic_rows: vec![],
            mastery_rows: vec![],
            buildable: vec![],
            recs: vec![],
            update: Arc::new(Mutex::new(UpdateState::Idle)),
            last_poll: Instant::now(),
            settings,
        };
        // your own inventory: the configured file, else AlecaFrame's own
        if app.settings.overwolf_bridge {
            app.gep = Some(crate::gep::start(ctx.clone()));
        }
        let own = app.pick_inventory_path();
        if let Some(p) = own {
            app.me.load(p);
        }
        if let Some(p) = app.settings.friend_path.clone().map(PathBuf::from).filter(|p| p.exists()) {
            app.friend_file.load(p);
        }
        app.rebuild_compare();
        app.reload_friend_snapshot(true);
        if app.settings.live_enabled {
            app.start_live(ctx);
        }
        app.check_updates(ctx);
        app
    }

    /// Where your inventory comes from: AlecaFrame's file, the live Overwolf
    /// copy, or (auto) whichever was written most recently.
    pub fn pick_inventory_path(&self) -> Option<PathBuf> {
        let aleca = self.settings.own_path.clone().map(PathBuf::from).or_else(alecaframe_data_file).filter(|p| p.exists());
        let ow = Some(crate::gep::inventory_path()).filter(|p| p.exists());
        let mtime = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        match self.settings.inventory_source.as_str() {
            "alecaframe" => aleca,
            "overwolf" => ow,
            _ => match (aleca, ow) {
                (Some(a), Some(o)) => {
                    if mtime(&o) > mtime(&a) {
                        Some(o)
                    } else {
                        Some(a)
                    }
                }
                (a, o) => a.or(o),
            },
        }
    }

    pub fn check_updates(&self, ctx: &egui::Context) {
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

    pub fn install_update(&self, rel: update::Release, ctx: &egui::Context) {
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

    pub fn names(&self) -> [String; 2] {
        [self.settings.my_name.clone(), self.settings.friend_name.clone()]
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.notices.last().is_some_and(|n| n.text == text) {
            return;
        }
        self.notices.push(Notice { ts: now_ms(), text });
        if self.notices.len() > 60 {
            self.notices.remove(0);
        }
    }

    /// Persist settings and re-derive everything that depends on them.
    pub fn settings_changed(&mut self) {
        self.settings.save();
        self.social_dirty = true;
    }

    pub fn price_of(&self, unique: &str) -> Option<f32> {
        if !self.settings.use_market {
            return None;
        }
        let slug = self.cat.slug(unique)?;
        self.market.price(slug)
    }

    // ---------------------------------------------------------------- live game
    pub fn start_live(&mut self, ctx: &egui::Context) {
        let path = self.settings.ee_log_path.clone().map(PathBuf::from).or_else(default_ee_log);
        self.ee = path.map(|p| eelog::start(p, self.settings.ee_patterns.clone(), ctx));
    }

    pub fn stop_live(&mut self) {
        self.ee = None;
        self.mission_active = false;
    }

    fn drain_ee(&mut self) {
        let mut evs = Vec::new();
        if let Some(ee) = &self.ee {
            while let Ok(e) = ee.rx.try_recv() {
                evs.push(e);
            }
        }
        for e in evs {
            match e.kind.as_str() {
                "mission_name" => {
                    if let Some(n) = crate::eelog::tail_after(&e.line, "Mission name: ") {
                        self.mission_name = Some(n);
                    }
                }
                "credits" => {
                    if let Some(n) = crate::eelog::number_after(&e.line, "): ") {
                        self.events.push(LiveEvent { ts: e.ts_ms, text: format!("💰 +{} credits from the mission", fmt_n(n)) });
                    }
                }
                "mission_start" => {
                    if !self.mission_active {
                        self.mission_active = true;
                        self.pre_mission = self.me.acct.as_ref().map(|a| a.inv.clone());
                        let name = self.mission_name.clone().unwrap_or_default();
                        let text = if name.is_empty() { "🚀 Mission started".to_string() } else { format!("🚀 Mission started: {name}") };
                        self.events.push(LiveEvent { ts: e.ts_ms, text });
                        if self.settings.notify.events {
                            self.notify("Mission started. Ordis will summarise the results.");
                        }
                    }
                }
                "mission_end" => {
                    if self.mission_active {
                        self.mission_active = false;
                        self.pending_summary = true;
                        self.events.push(LiveEvent { ts: e.ts_ms, text: "🏁 Mission ended. Waiting for AlecaFrame to refresh your inventory…".into() });
                        if self.settings.notify.events {
                            self.notify("Mission ended. The summary appears when your inventory refreshes.");
                        }
                    }
                }
                _ => self.events.push(LiveEvent { ts: e.ts_ms, text: e.label }),
            }
        }
        if self.events.len() > 200 {
            self.events.drain(..self.events.len() - 200);
        }
    }

    // ------------------------------------------------------------- inventories
    pub fn goal_done_set(&self) -> HashSet<u64> {
        let Some(me) = &self.me.acct else { return HashSet::new() };
        self.settings.goals.iter().filter(|g| status(&self.cat, me, g).done).map(|g| g.id).collect()
    }

    pub fn reload_me(&mut self) {
        let Some(path) = self.me.path.clone() else { return };
        let before = self.goal_done_set();
        let old_inv = self.me.acct.as_ref().map(|a| a.inv.clone());
        self.me.load(path);
        self.after_reload(before, old_inv);
    }

    pub fn load_me(&mut self, path: PathBuf) {
        let before = self.goal_done_set();
        let old_inv = self.me.acct.as_ref().map(|a| a.inv.clone());
        self.settings.own_path = Some(path.to_string_lossy().to_string());
        self.me.load(path);
        self.settings.save();
        self.after_reload(before, old_inv);
    }

    pub fn load_friend_file(&mut self, path: PathBuf) {
        self.settings.friend_path = Some(path.to_string_lossy().to_string());
        self.friend_file.load(path);
        self.settings.save();
        self.rebuild_compare();
    }

    fn after_reload(&mut self, before: HashSet<u64>, old_inv: Option<Inventory>) {
        self.rebuild_compare();
        self.social_dirty = true;
        let after = self.goal_done_set();
        if self.settings.notify.goals {
            let done: Vec<String> =
                self.settings.goals.iter().filter(|g| after.contains(&g.id) && !before.contains(&g.id)).map(|g| g.name.clone()).collect();
            for n in done {
                self.notify(format!("🎉 Goal complete: {n}"));
            }
        }
        if self.pending_summary {
            if let (Some(me), Some(pre)) = (&self.me.acct, self.pre_mission.take().or(old_inv)) {
                let g = gained(&pre, &me.inv);
                let text = if g.is_empty() {
                    "Mission summary: nothing new in your inventory yet.".to_string()
                } else {
                    let items: Vec<String> =
                        g.iter().take(12).map(|(p, n)| format!("+{} {}", fmt_n(*n), self.cat.display(p))).collect();
                    format!("Mission summary: {}", items.join(", "))
                };
                self.events.push(LiveEvent { ts: now_ms(), text: text.clone() });
                self.notify(text);
                self.pending_summary = false;
            }
        }
    }

    fn rebuild_inventory_rows(&mut self) {
        self.inv_rows.clear();
        let Some(me) = &self.me.acct else { return };
        for (path, e) in &me.inv {
            let name = match self.cat.relic(path) {
                Some((r, refinement)) => format!("{} Relic ({refinement})", r.name),
                None => self.cat.display(path),
            };
            self.inv_rows.push(InvRow { path: path.clone(), name_lower: name.to_lowercase(), name, category: e.category.clone(), count: e.count });
        }
        self.inv_rows.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        self.mastery_rows = mastery_rows(&self.cat, me);
        self.buildable = buildable_now(&self.cat, me);
    }

    pub fn rebuild_compare(&mut self) {
        self.rebuild_inventory_rows();
        self.rows.clear();
        self.plan = None;
        if let (Some(a), Some(b)) = (&self.me.acct, &self.friend_file.acct) {
            self.rows = diff(&a.inv, &b.inv);
            for r in &mut self.rows {
                if let Some(n) = self.cat.name(&r.path) {
                    r.name = n.to_string();
                }
            }
            self.rows.sort_by(|x, y| x.category.cmp(&y.category).then(x.name.cmp(&y.name)));
            self.plan = Some(trade::plan(&self.cat, [&a.inv, &b.inv], [&a.prog, &b.prog]));
        }
        let mut cats: Vec<String> = self.rows.iter().map(|r| r.category.clone()).collect();
        cats.sort();
        cats.dedup();
        self.categories = cats;
        if self.category.as_ref().is_some_and(|c| !self.categories.contains(c)) {
            self.category = None;
        }
    }

    // ----------------------------------------------------------------- social
    pub fn reload_friend_snapshot(&mut self, force: bool) {
        let sync = self.settings.sync_dir.clone().map(PathBuf::from);
        let own_name = share::snapshot_file_name(&self.settings.my_name);
        let path = self
            .friend_snap_path
            .clone()
            .filter(|p| p.exists())
            .or_else(|| sync.as_ref().and_then(|d| share::newest_friend_file(d, &own_name)));
        let Some(path) = path else { return };
        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if !force && mtime == self.friend_snap_mtime && self.friend_snap_path.as_ref() == Some(&path) {
            return;
        }
        match Snapshot::load(&path) {
            Ok(s) => {
                self.snap_error = None;
                if self.settings.friend_name == "Friend" && !s.name.is_empty() {
                    self.settings.friend_name = s.name.clone();
                    self.settings.save();
                }
                self.friend_snap = Some(s);
            }
            Err(e) => self.snap_error = Some(e),
        }
        self.friend_snap_mtime = mtime;
        self.friend_snap_path = Some(path);
        self.social_dirty = true;
    }

    pub fn load_friend_snapshot(&mut self, path: PathBuf) {
        self.friend_snap_path = Some(path);
        self.reload_friend_snapshot(true);
    }

    pub fn refresh_social(&mut self) {
        self.last_social = Instant::now();
        self.social_dirty = false;
        let Some(me) = &self.me.acct else { return };
        let want_price = self.settings.rules.max_price > 0;
        let (avail, needs, farming, matching, snap) = {
            let price = |u: &str| if want_price { self.price_of(u) } else { None };
            let avail = share::available_items(&self.cat, me, &self.settings, &price);
            let (needs, farming, _) = share::my_needs(&self.cat, me, &self.settings);
            let matching = self.friend_snap.as_ref().map(|f| share::match_up(&needs, &avail, &farming, f));
            let snap = share::build_snapshot(&self.cat, me, &self.settings, &price);
            (avail, needs, farming, matching, snap)
        };

        // tell the player when a friend newly has something they need
        if self.settings.notify.friends {
            if let (Some(m), Some(f)) = (&matching, &self.friend_snap) {
                let old: HashSet<&String> = self.matching.iter().flat_map(|o| o.they_have.iter().map(|(n, _)| n)).collect();
                let fresh: Vec<String> = m.they_have.iter().filter(|(n, _)| !old.contains(n)).map(|(n, _)| n.clone()).collect();
                let who = f.name.clone();
                if self.matching.is_some() {
                    for n in fresh {
                        self.notices.push(Notice { ts: now_ms(), text: format!("🤝 {who} now has {n}, which you need.") });
                    }
                }
            }
        }
        self.my_avail = avail;
        {
            let me = self.me.acct.as_ref().unwrap();
            let (needed, _) = needed_parts(&self.cat, me, &self.settings.goals);
            self.relic_rows = relic_rows(&self.cat, me, &needed);
        }
        self.my_needs = needs;
        self.my_farming = farming;
        self.matching = matching;
        self.recs = self.recommendations();

        // publish to the shared folder, only when something actually changed
        if let Some(dir) = self.settings.sync_dir.clone().filter(|d| !d.is_empty()) {
            let mut snap = snap;
            snap.created_ms = 0;
            let key = serde_json::to_string(&snap).unwrap_or_default();
            if key != self.last_snap_key {
                snap.created_ms = now_ms();
                let path = PathBuf::from(dir).join(share::snapshot_file_name(&self.settings.my_name));
                if snap.save(&path).is_ok() {
                    self.last_snap_written = Some(snap.created_ms);
                    self.last_snap_key = key;
                }
            }
        }
    }

    pub fn export_snapshot(&self, path: &std::path::Path) -> Result<(), String> {
        let me = self.me.acct.as_ref().ok_or("load your inventory first")?;
        let price = |_: &str| None;
        share::build_snapshot(&self.cat, me, &self.settings, &price).save(path)
    }

    // ------------------------------------------------------------------- chat
    pub fn ask(&mut self, question: &str, ctx: &egui::Context) {
        let q = question.trim().to_string();
        if q.is_empty() {
            return;
        }
        let facts = {
            let world = self.world.snapshot();
            let c = Ctx {
                cat: &self.cat,
                me: self.me.acct.as_ref(),
                friend: self.friend_snap.as_ref(),
                matching: self.matching.as_ref(),
                my_avail: &self.my_avail,
                settings: &self.settings,
                world: &world,
            };
            assistant::answer(&c, &q)
        };
        self.chat.push(ChatMsg { user: true, text: q.clone(), facts: String::new(), pending: false });
        let idx = self.chat.len();
        let text = assistant::with_flavor(&facts, idx);
        let use_llm = self.settings.llm.enabled;
        self.chat.push(ChatMsg { user: false, text, facts: facts.clone(), pending: use_llm });
        if use_llm {
            let (cfg, tx, ctx) = (self.settings.llm.clone(), self.llm_tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                let r = assistant::ask_llm(&cfg, &q, &facts);
                let _ = tx.send((idx, r));
                ctx.request_repaint();
            });
        } else if self.settings.voice {
            assistant::speak(&self.chat[idx].text);
        }
    }

    fn drain_llm(&mut self) {
        while let Ok((idx, r)) = self.llm_rx.try_recv() {
            if let Some(m) = self.chat.get_mut(idx) {
                m.pending = false;
                match r {
                    Ok(t) => m.text = t,
                    Err(e) => m.text = format!("{}\n\n(Ordis could not reach the language model: {e}. Showing the plain answer.)", m.text),
                }
                if self.settings.voice {
                    assistant::speak(&m.text);
                }
            }
        }
    }

    pub fn recommendations(&self) -> Vec<crate::recommend::Rec> {
        match &self.me.acct {
            Some(me) => {
                let world = self.world.snapshot();
                recommend(&self.cat, me, &self.settings, &world, self.friend_snap.as_ref(), self.matching.as_ref())
            }
            None => vec![],
        }
    }

    // ------------------------------------------------------------- frame loop
    fn tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if now.duration_since(self.last_poll) > Duration::from_secs(3) {
            self.last_poll = now;
            if self.settings.auto_reload {
                match self.pick_inventory_path() {
                    Some(p) if Some(&p) != self.me.path.as_ref() => {
                        let before = self.goal_done_set();
                        let old_inv = self.me.acct.as_ref().map(|a| a.inv.clone());
                        self.me.load(p);
                        self.after_reload(before, old_inv);
                    }
                    _ if self.me.changed_on_disk() => self.reload_me(),
                    _ => {}
                }
            }
            if self.friend_file.changed_on_disk() {
                if let Some(p) = self.friend_file.path.clone() {
                    self.friend_file.load(p);
                    self.rebuild_compare();
                }
            }
            self.reload_friend_snapshot(false);
        }
        if self.world.age_ms() > 120_000 {
            self.world.refresh(ctx);
        }
        self.drain_ee();
        self.drain_llm();
        if self.social_dirty || now.duration_since(self.last_social) > Duration::from_secs(20) {
            self.refresh_social();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ordis Nexus").size(28.0).strong());
            ui.label(RichText::new(format!("v{}", update::current_version())).color(SUBTLE));
            let state = self.update.lock().unwrap().clone();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match &state {
                    UpdateState::Available(r) => {
                        if ui.add(accent_button(&format!("⬇ Update to {}", r.tag))).clicked() {
                            let ctx = ui.ctx().clone();
                            self.install_update(r.clone(), &ctx);
                        }
                    }
                    UpdateState::Installing => {
                        ui.label(RichText::new("Installing update… the app will restart").color(WARN));
                    }
                    UpdateState::Error(_) => {
                        if ui.small_button("Update check failed – retry").clicked() {
                            let ctx = ui.ctx().clone();
                            self.check_updates(&ctx);
                        }
                    }
                    _ => {}
                }
                if self.mission_active {
                    chip(ui, "● In mission", GOOD);
                }
                if let Some(m) = &self.me.acct {
                    chip(ui, &format!("MR {}", m.mr), ACCENT);
                }
                if !self.notices.is_empty() && ui.button(format!("🔔 {}", self.notices.len())).clicked() {
                    self.tab = Tab::Overview;
                }
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);

        // Drag & drop: a .nexus.json is a friend's snapshot; other files are inventories.
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        for p in dropped {
            if p.to_string_lossy().ends_with(".nexus.json") {
                self.load_friend_snapshot(p);
            } else if self.me.acct.is_none() {
                self.load_me(p);
            } else {
                self.load_friend_file(p);
            }
        }

        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(20))).show(ui, |ui| {
            self.header(ui);
            ui.horizontal_wrapped(|ui| {
                for (t, label) in Tab::ALL {
                    ui.selectable_value(&mut self.tab, t, label);
                }
            });
            ui.separator();
            match self.tab {
                Tab::Overview => views::overview::show(self, ui),
                Tab::Ordis => views::chat::show(self, ui),
                Tab::Goals => views::goals::show(self, ui),
                Tab::Inventory => views::inventory::show(self, ui),
                Tab::Foundry => views::foundry::show(self, ui),
                Tab::Relics => views::relics::show(self, ui),
                Tab::Mastery => views::mastery::show(self, ui),
                Tab::World => views::world::show(self, ui),
                Tab::Friends => views::friends::show(self, ui),
                Tab::Compare => views::compare::show(self, ui),
                Tab::Live => views::live::show(self, ui),
                Tab::Settings => views::settings::show(self, ui),
            }
        });
    }
}

#[allow(dead_code)]
fn _unused(_: HashMap<u8, u8>) {}
