//! The private social layer: what you share, in what form, and how two
//! players' shared snapshots are matched up. Nothing leaves the machine unless
//! you put the snapshot file somewhere your friend can read it.

use crate::account::Account;
use crate::catalog::Catalog;
use crate::goals::status;
use crate::settings::Settings;
use crate::trade::{status as item_status, Status};
use crate::util::{blueprint_form, now_ms};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Avail {
    pub unique: String,
    pub name: String,
    pub count: u64,
    /// `prime part`, `mod`, `relic` or `pinned`.
    pub kind: String,
    pub price: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Need {
    pub unique: String,
    pub name: String,
    pub count: u64,
    /// Which goal it serves.
    pub goal: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GoalPub {
    pub name: String,
    pub summary: String,
    pub fraction: f32,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct MasteryPub {
    pub rank: u32,
    pub mastered: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Snapshot {
    pub version: u32,
    /// The name you chose for Ordis Nexus. Never your Warframe username.
    pub name: String,
    pub created_ms: i64,
    pub goals: Option<Vec<GoalPub>>,
    pub needs: Option<Vec<Need>>,
    pub activity: Option<String>,
    pub farming: Option<Vec<Need>>,
    pub available: Option<Vec<Avail>>,
    pub mastery: Option<MasteryPub>,
    pub resources: Option<BTreeMap<String, u64>>,
    pub inventory: Option<BTreeMap<String, u64>>,
}

fn have_give(acct: &Account, unique: &str) -> (u64, u64) {
    let built = acct.count(unique);
    match unique.strip_suffix("Component") {
        Some(base) => {
            let bp = acct.count(&format!("{base}Blueprint"));
            (built + bp, bp)
        }
        None => (built, built),
    }
}

/// Everything the rules say you could give away.
pub fn available_items(cat: &Catalog, acct: &Account, s: &Settings, price: &dyn Fn(&str) -> Option<f32>) -> Vec<Avail> {
    let rules = &s.rules;
    let never: Vec<String> = s.never_share.iter().map(|u| u.to_lowercase()).collect();
    let mut out: Vec<Avail> = Vec::new();

    if rules.prime_parts {
        for prime in &cat.primes {
            let st = item_status(&acct.prog, &prime.unique, prime.mastery_xp);
            if rules.require_mastered && st != Status::Mastered {
                continue;
            }
            for part in &prime.parts {
                let (have, give) = have_give(acct, &part.unique);
                let spare = (if st == Status::Missing { have.saturating_sub(part.need as u64) } else { have }).min(give);
                if spare < (rules.min_spare as u64).max(1) {
                    continue;
                }
                let bp = blueprint_form(&part.unique);
                let p = price(&bp);
                if rules.max_price > 0 {
                    match p {
                        Some(v) if v > rules.max_price as f32 => continue,
                        None if !rules.unknown_price_ok => continue,
                        _ => {}
                    }
                }
                out.push(Avail { unique: bp, name: part.name.clone(), count: spare, kind: "prime part".into(), price: p });
            }
        }
    }
    if rules.mods {
        for (path, e) in &acct.inv {
            if e.category == "Mods" && e.count > 1 {
                out.push(Avail { unique: path.clone(), name: cat.display(path), count: e.count - 1, kind: "mod".into(), price: None });
            }
        }
    }
    if rules.relics {
        for (path, count) in acct.relics() {
            if count > 1 {
                let name = cat.relic(path).map(|(r, ref_)| format!("{} ({ref_})", r.name)).unwrap_or_else(|| cat.display(path));
                out.push(Avail { unique: path.to_string(), name, count: count - 1, kind: "relic".into(), price: None });
            }
        }
    }
    // pinned: always share, whatever the rules say
    for u in &s.always_share {
        let n = acct.count(u);
        if n > 0 && !out.iter().any(|a| a.unique.eq_ignore_ascii_case(u)) {
            out.push(Avail { unique: u.clone(), name: cat.display(u), count: n, kind: "pinned".into(), price: price(u) });
        }
    }
    out.retain(|a| !never.contains(&a.unique.to_lowercase()));
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Parts and resources your shared goals still need.
pub fn my_needs(cat: &Catalog, acct: &Account, s: &Settings) -> (Vec<Need>, Vec<Need>, Vec<GoalPub>) {
    let (mut parts, mut res, mut goals) = (Vec::new(), Vec::new(), Vec::new());
    for g in s.goals.iter().filter(|g| g.shared) {
        let st = status(cat, acct, g);
        goals.push(GoalPub { name: g.name.clone(), summary: st.summary.clone(), fraction: st.fraction });
        if st.done {
            continue;
        }
        for (u, n) in &st.missing_parts {
            parts.push(Need { unique: blueprint_form(u), name: cat.display(&blueprint_form(u)), count: *n, goal: g.name.clone() });
        }
        for (u, n) in &st.missing_resources {
            res.push(Need { unique: u.clone(), name: cat.display(u), count: *n, goal: g.name.clone() });
        }
    }
    (parts, res, goals)
}

pub fn build_snapshot(cat: &Catalog, acct: &Account, s: &Settings, price: &dyn Fn(&str) -> Option<f32>) -> Snapshot {
    let sh = &s.sharing;
    let (parts, res, goals) = my_needs(cat, acct, s);
    let mut snap = Snapshot { version: 1, name: s.my_name.clone(), created_ms: now_ms(), ..Default::default() };
    if sh.goals {
        snap.goals = Some(goals);
        snap.needs = Some(parts);
    }
    if sh.activity {
        snap.activity = Some(s.activity.clone());
        snap.farming = Some(res);
    }
    if sh.available_items {
        snap.available = Some(available_items(cat, acct, s, price));
    }
    if sh.mastery {
        let mastered = cat
            .items
            .iter()
            .filter(|it| item_status(&acct.prog, &it.unique, it.mastery_xp) == Status::Mastered)
            .map(|it| it.unique.clone())
            .collect();
        snap.mastery = Some(MasteryPub { rank: acct.mr, mastered });
    }
    if sh.resource_quantities {
        let r: BTreeMap<String, u64> =
            acct.inv.iter().filter(|(_, e)| e.category == "Resources & misc").map(|(k, e)| (k.clone(), e.count)).collect();
        snap.resources = Some(r);
    }
    if sh.full_inventory {
        snap.inventory = Some(acct.inv.iter().map(|(k, e)| (k.clone(), e.count)).collect());
    }
    snap
}

pub fn snapshot_file_name(name: &str) -> String {
    let clean: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    format!("{}.nexus.json", if clean.is_empty() { "me".to_string() } else { clean })
}

impl Snapshot {
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| e.to_string())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        serde_json::from_str(&s).map_err(|e| format!("not a Nexus snapshot: {e}"))
    }
}

/// The newest `*.nexus.json` in `dir` that isn't `own_file_name`.
pub fn newest_friend_file(dir: &Path, own_file_name: &str) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".nexus.json") || name.eq_ignore_ascii_case(own_file_name) {
            continue;
        }
        let m = e.metadata().ok()?.modified().ok()?;
        if best.as_ref().is_none_or(|(t, _)| m > *t) {
            best = Some((m, e.path()));
        }
    }
    best.map(|(_, p)| p)
}

#[derive(Default, Debug)]
pub struct Matching {
    /// Things you need that the friend has available: (name, count).
    pub they_have: Vec<(String, u64)>,
    /// Things the friend needs that you have available.
    pub i_have: Vec<(String, u64)>,
    /// Resources you are both short of, so you could farm together.
    pub shared_farming: Vec<String>,
}

impl Matching {
    pub fn mutual(&self) -> bool {
        !self.they_have.is_empty() && !self.i_have.is_empty()
    }
}

fn key(u: &str) -> String {
    blueprint_form(u).to_lowercase()
}

pub fn match_up(my_needs: &[Need], my_avail: &[Avail], my_farming: &[Need], friend: &Snapshot) -> Matching {
    let mut m = Matching::default();
    if let Some(av) = &friend.available {
        let have: HashMap<String, &Avail> = av.iter().map(|a| (key(&a.unique), a)).collect();
        for n in my_needs {
            if let Some(a) = have.get(&key(&n.unique)) {
                m.they_have.push((a.name.clone(), a.count.min(n.count.max(1))));
            }
        }
    }
    if let Some(needs) = &friend.needs {
        let mine: HashMap<String, &Avail> = my_avail.iter().map(|a| (key(&a.unique), a)).collect();
        for n in needs {
            if let Some(a) = mine.get(&key(&n.unique)) {
                m.i_have.push((a.name.clone(), a.count.min(n.count.max(1))));
            }
        }
    }
    if let Some(f) = &friend.farming {
        for mine in my_farming {
            if f.iter().any(|x| x.unique.eq_ignore_ascii_case(&mine.unique)) {
                m.shared_farming.push(mine.name.clone());
            }
        }
    }
    m
}

/// Search a friend's shared items by name.
pub fn friend_has(friend: &Snapshot, query: &str) -> Vec<Avail> {
    let q = query.to_lowercase();
    friend.available.iter().flatten().filter(|a| a.name.to_lowercase().contains(&q)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::settings::{GoalKind, Settings};
    use serde_json::json;

    const NAMI: &str = "/Lotus/Weapons/Tenno/Melee/Swords/PrimeNamiSkyla/PrimeNamiSkyla";
    const HANDLE: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeNamiSkylaHandle";

    fn cat() -> Catalog {
        Catalog::from_json(&format!(
            r#"{{"names":{{}},"icons":{{}},"primes":[{{"u":"{NAMI}","n":"Nami Skyla Prime","x":450000,"p":[["{HANDLE}","Nami Skyla Prime Handle",2]]}}]}}"#
        ))
        .unwrap()
    }

    fn acct(handles: u64, xp: u64) -> Account {
        Account::from_json(&json!({
            "MiscItems": [{"ItemType": HANDLE, "ItemCount": handles}],
            "XPInfo": [{"ItemType": NAMI, "XP": xp}],
            "Melee": [{"ItemType": NAMI}]
        }))
    }

    #[test]
    fn rules_gate_what_is_available() {
        let none = |_: &str| None;
        let mut s = Settings::default();
        // mastered with 3 handles -> 3 available (mastered items need none)
        let a = available_items(&cat(), &acct(3, 450000), &s, &none);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].count, 3);
        // not mastered: rule keeps it private
        assert!(available_items(&cat(), &acct(3, 0), &s, &none).is_empty());
        // pinned overrides, never overrides everything
        s.always_share.push(HANDLE.into());
        assert_eq!(available_items(&cat(), &acct(3, 0), &s, &none).len(), 1);
        s.never_share.push(HANDLE.into());
        assert!(available_items(&cat(), &acct(3, 450000), &s, &none).is_empty());
    }

    #[test]
    fn price_ceiling() {
        let mut s = Settings::default();
        s.rules.max_price = 20;
        let pricey = |_: &str| Some(45.0);
        let cheap = |_: &str| Some(10.0);
        assert!(available_items(&cat(), &acct(2, 450000), &s, &pricey).is_empty());
        assert_eq!(available_items(&cat(), &acct(2, 450000), &s, &cheap).len(), 1);
    }

    #[test]
    fn privacy_flags_control_the_snapshot() {
        let none = |_: &str| None;
        let mut s = Settings::default();
        s.add_goal(GoalKind::Collect, "/Lotus/Types/Items/MiscItems/Oxium", "Oxium", 100);
        let snap = build_snapshot(&cat(), &acct(1, 450000), &s, &none);
        assert!(snap.goals.is_some() && snap.available.is_some());
        assert!(snap.inventory.is_none() && snap.resources.is_none());
        s.sharing.goals = false;
        s.sharing.available_items = false;
        let snap = build_snapshot(&cat(), &acct(1, 450000), &s, &none);
        assert!(snap.goals.is_none() && snap.needs.is_none() && snap.available.is_none());
    }

    #[test]
    fn mutual_help_and_shared_farming() {
        let me_need = vec![Need { unique: "/A/WispPrimeSystemsBlueprint".into(), name: "Wisp Prime Systems".into(), count: 1, goal: "Wisp".into() }];
        let me_avail = vec![Avail { unique: "/A/VoltPrimeHelmetBlueprint".into(), name: "Volt Prime Neuroptics".into(), count: 2, ..Default::default() }];
        let me_farm = vec![Need { unique: "/Lotus/Oxium".into(), name: "Oxium".into(), count: 500, goal: "x".into() }];
        let friend = Snapshot {
            available: Some(vec![Avail { unique: "/A/WispPrimeSystemsComponent".into(), name: "Wisp Prime Systems".into(), count: 1, ..Default::default() }]),
            needs: Some(vec![Need { unique: "/A/VoltPrimeHelmetBlueprint".into(), name: "Volt Prime Neuroptics".into(), count: 1, goal: "Volt".into() }]),
            farming: Some(vec![Need { unique: "/lotus/oxium".into(), name: "Oxium".into(), count: 900, goal: "y".into() }]),
            ..Default::default()
        };
        let m = match_up(&me_need, &me_avail, &me_farm, &friend);
        assert!(m.mutual());
        assert_eq!(m.they_have, vec![("Wisp Prime Systems".to_string(), 1)]);
        assert_eq!(m.i_have, vec![("Volt Prime Neuroptics".to_string(), 1)]);
        assert_eq!(m.shared_farming, vec!["Oxium".to_string()]);
        assert_eq!(friend_has(&friend, "wisp").len(), 1);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(snapshot_file_name("Dave & Co"), "Dave___Co.nexus.json");
        assert_eq!(snapshot_file_name(""), "me.nexus.json");
    }
}
