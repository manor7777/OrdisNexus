//! Global game state (fissures, invasions, cycles, Baro...) from api.warframestat.us.
//! This is public data, separate from your private inventory.

use crate::util::{fmt_duration, now_ms, parse_iso_ms};
use eframe::egui;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default, Debug)]
pub struct Fissure {
    pub node: String,
    pub mission: String,
    pub tier: String,
    pub storm: bool,
    pub hard: bool,
    pub expiry_ms: i64,
}

#[derive(Clone, Default, Debug)]
pub struct Invasion {
    pub node: String,
    pub desc: String,
    pub attacker: String,
    pub defender: String,
    pub completion: f64,
}

#[derive(Clone, Default, Debug)]
pub struct Cycle {
    pub name: String,
    pub state: String,
    pub expiry_ms: i64,
}

#[derive(Clone, Default, Debug)]
pub struct WorldState {
    pub fetched_ms: i64,
    pub error: Option<String>,
    pub fissures: Vec<Fissure>,
    pub invasions: Vec<Invasion>,
    pub cycles: Vec<Cycle>,
    pub sortie: Vec<String>,
    pub archon: Vec<String>,
    pub baro: Option<String>,
    pub nightwave: Vec<String>,
    pub steel_path: Option<String>,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn expiry(v: &Value) -> i64 {
    v.get("expiry").and_then(Value::as_str).and_then(parse_iso_ms).unwrap_or(0)
}

fn reward_text(side: &Value) -> String {
    let r = side.get("reward").unwrap_or(&Value::Null);
    let mut parts: Vec<String> = Vec::new();
    for c in r.get("countedItems").and_then(Value::as_array).into_iter().flatten() {
        parts.push(format!("{}× {}", c.get("count").and_then(Value::as_u64).unwrap_or(1), s(c, "type")));
    }
    for i in r.get("items").and_then(Value::as_array).into_iter().flatten() {
        if let Some(t) = i.as_str() {
            parts.push(t.to_string());
        }
    }
    let credits = r.get("credits").and_then(Value::as_u64).unwrap_or(0);
    if credits > 0 {
        parts.push(format!("{credits} credits"));
    }
    parts.join(", ")
}

pub fn parse(v: &Value) -> WorldState {
    let now = now_ms();
    let mut w = WorldState { fetched_ms: now, ..Default::default() };

    for f in v.get("fissures").and_then(Value::as_array).into_iter().flatten() {
        let e = expiry(f);
        if e != 0 && e < now {
            continue;
        }
        w.fissures.push(Fissure {
            node: s(f, "node"),
            mission: s(f, "missionType"),
            tier: s(f, "tier"),
            storm: f.get("isStorm").and_then(Value::as_bool).unwrap_or(false),
            hard: f.get("isHard").and_then(Value::as_bool).unwrap_or(false),
            expiry_ms: e,
        });
    }
    w.fissures.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.expiry_ms.cmp(&b.expiry_ms)));

    for i in v.get("invasions").and_then(Value::as_array).into_iter().flatten() {
        if i.get("completed").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        w.invasions.push(Invasion {
            node: s(i, "node"),
            desc: s(i, "desc"),
            attacker: reward_text(i.get("attacker").unwrap_or(&Value::Null)),
            defender: reward_text(i.get("defender").unwrap_or(&Value::Null)),
            completion: i.get("completion").and_then(Value::as_f64).unwrap_or(0.0),
        });
    }

    for (key, name) in [
        ("earthCycle", "Earth"),
        ("cetusCycle", "Plains of Eidolon"),
        ("vallisCycle", "Orb Vallis"),
        ("cambionCycle", "Cambion Drift"),
        ("zarimanCycle", "Zariman"),
        ("duviriCycle", "Duviri"),
    ] {
        if let Some(c) = v.get(key) {
            let state = s(c, "state");
            if !state.is_empty() {
                w.cycles.push(Cycle { name: name.into(), state, expiry_ms: expiry(c) });
            }
        }
    }

    if let Some(so) = v.get("sortie") {
        for var in so.get("variants").and_then(Value::as_array).into_iter().flatten() {
            w.sortie.push(format!("{} — {} ({})", s(var, "missionType"), s(var, "node"), s(var, "modifier")));
        }
    }
    if let Some(a) = v.get("archonHunt") {
        for m in a.get("missions").and_then(Value::as_array).into_iter().flatten() {
            w.archon.push(format!("{} — {}", s(m, "type"), s(m, "node")));
        }
    }
    if let Some(b) = v.get("voidTrader") {
        let act = b.get("activation").and_then(Value::as_str).and_then(parse_iso_ms).unwrap_or(0);
        let exp = expiry(b);
        let (name, loc) = (s(b, "character"), s(b, "location"));
        if act > 0 {
            w.baro = Some(if act <= now && now < exp {
                format!("{name} is at {loc} for another {}", fmt_duration(exp - now))
            } else if act > now {
                format!("{name} arrives at {loc} in {}", fmt_duration(act - now))
            } else {
                format!("{name} has left")
            });
        }
    }
    if let Some(n) = v.get("nightwave") {
        for c in n.get("activeChallenges").and_then(Value::as_array).into_iter().flatten() {
            let tag = if c.get("isElite").and_then(Value::as_bool).unwrap_or(false) {
                "elite"
            } else if c.get("isDaily").and_then(Value::as_bool).unwrap_or(false) {
                "daily"
            } else {
                "weekly"
            };
            w.nightwave.push(format!("[{tag}] {} — {}", s(c, "title"), s(c, "desc")));
        }
    }
    if let Some(sp) = v.get("steelPath") {
        let cur = sp.get("currentReward").unwrap_or(&Value::Null);
        let name = s(cur, "name");
        if !name.is_empty() {
            w.steel_path = Some(format!("Steel Path honors: {name} ({} Steel Essence)", cur.get("cost").and_then(Value::as_u64).unwrap_or(0)));
        }
    }
    w
}

#[derive(Clone)]
pub struct WorldFeed {
    pub state: Arc<Mutex<WorldState>>,
    busy: Arc<AtomicBool>,
}

impl WorldFeed {
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(WorldState::default())), busy: Arc::new(AtomicBool::new(false)) }
    }

    pub fn snapshot(&self) -> WorldState {
        self.state.lock().unwrap().clone()
    }

    pub fn age_ms(&self) -> i64 {
        let f = self.state.lock().unwrap().fetched_ms;
        if f == 0 { i64::MAX } else { now_ms() - f }
    }

    pub fn refresh(&self, ctx: &egui::Context) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let (state, busy, ctx) = (self.state.clone(), self.busy.clone(), ctx.clone());
        std::thread::spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).build().into();
            let res = agent
                .get("https://api.warframestat.us/pc?language=en")
                .header("Accept", "application/json")
                .call()
                .map_err(|e| format!("could not reach the WorldState API: {e}"))
                .and_then(|mut r| r.body_mut().read_to_vec().map_err(|e| e.to_string()))
                .and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| format!("bad WorldState data: {e}")));
            let mut g = state.lock().unwrap();
            match res {
                Ok(v) => *g = parse(&v),
                Err(e) => {
                    g.error = Some(e);
                    g.fetched_ms = now_ms();
                }
            }
            drop(g);
            busy.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_realistic_payload() {
        let v = json!({
            "fissures": [
                {"node": "Unda (Venus)", "missionType": "Spy", "tier": "Lith", "isStorm": false, "isHard": true, "expiry": "2999-01-01T00:00:00.000Z"},
                {"node": "Old (Earth)", "missionType": "Spy", "tier": "Meso", "expiry": "2001-01-01T00:00:00.000Z"}
            ],
            "invasions": [{"node": "Enceladus (Saturn)", "desc": "Corpus Siege", "completion": 40.5, "completed": false,
                "attacker": {"reward": {"countedItems": [{"type": "Fieldron", "count": 3}], "items": [], "credits": 0}},
                "defender": {"reward": {"countedItems": [], "items": ["Orokin Reactor Blueprint"], "credits": 0}}}],
            "cetusCycle": {"state": "night", "expiry": "2999-01-01T00:00:00.000Z"},
            "sortie": {"variants": [{"missionType": "Defense", "node": "Odin (Mercury)", "modifier": "Augmented Enemy Armor"}]},
            "voidTrader": {"character": "Baro Ki'Teer", "location": "Strata Relay (Earth)", "activation": "2999-01-01T00:00:00.000Z", "expiry": "2999-01-03T00:00:00.000Z"},
            "nightwave": {"activeChallenges": [{"title": "Swatter", "desc": "Kill 3 Drones", "isDaily": true, "isElite": false}]}
        });
        let w = parse(&v);
        assert_eq!(w.fissures.len(), 1); // the expired one is dropped
        assert!(w.fissures[0].hard);
        assert_eq!(w.invasions[0].attacker, "3× Fieldron");
        assert_eq!(w.invasions[0].defender, "Orokin Reactor Blueprint");
        assert_eq!(w.cycles[0].state, "night");
        assert_eq!(w.sortie.len(), 1);
        assert!(w.baro.unwrap().contains("arrives at Strata Relay"));
        assert!(w.nightwave[0].starts_with("[daily]"));
    }
}
