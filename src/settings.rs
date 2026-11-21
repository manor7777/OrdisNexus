//! Everything the app remembers between runs.

use crate::util::data_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GoalKind {
    /// Build an item from its recipe.
    Build,
    /// Own at least `count` of something (resource, part, mod...).
    Collect,
    /// Reach a mastery rank.
    Mastery,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Goal {
    pub id: u64,
    pub kind: GoalKind,
    /// Item path (empty for mastery goals).
    pub target: String,
    pub name: String,
    pub count: u64,
    pub note: String,
    /// Include this goal's needs in what friends can see (when goal sharing is on).
    pub shared: bool,
}

/// What friends are allowed to see. Technical access is never social permission.
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Sharing {
    pub goals: bool,
    pub activity: bool,
    pub available_items: bool,
    pub full_inventory: bool,
    pub mastery: bool,
    pub resource_quantities: bool,
}

impl Default for Sharing {
    fn default() -> Self {
        Self { goals: true, activity: true, available_items: true, full_inventory: false, mastery: true, resource_quantities: false }
    }
}

/// Which owned items count as "available to friends".
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Rules {
    pub prime_parts: bool,
    pub mods: bool,
    pub relics: bool,
    /// Only parts of items the owner has already mastered.
    pub require_mastered: bool,
    /// Minimum number of spare copies before sharing.
    pub min_spare: u32,
    /// Platinum ceiling; 0 turns the price check off.
    pub max_price: u32,
    /// Share items whose price couldn't be looked up.
    pub unknown_price_ok: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self { prime_parts: true, mods: false, relics: false, require_mastered: true, min_spare: 1, max_price: 0, unknown_price_ok: true }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EePattern {
    pub label: String,
    pub contains: String,
    /// One of: `mission_start`, `mission_end`, `note`.
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LlmCfg {
    pub enabled: bool,
    /// OpenAI-compatible base URL, e.g. `http://localhost:11434/v1` for Ollama.
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

impl Default for LlmCfg {
    fn default() -> Self {
        Self { enabled: false, endpoint: "http://localhost:11434/v1".into(), model: "llama3.2".into(), api_key: String::new() }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Notify {
    pub goals: bool,
    pub friends: bool,
    pub events: bool,
    pub farming: bool,
}

impl Default for Notify {
    fn default() -> Self {
        Self { goals: true, friends: true, events: true, farming: true }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub my_name: String,
    pub friend_name: String,
    /// Override for your lastData.dat (default: AlecaFrame's own folder).
    pub own_path: Option<String>,
    /// Friend's lastData.dat, if they sent you one.
    pub friend_path: Option<String>,
    /// A folder both of you can see (OneDrive/Dropbox/Syncthing). Snapshots are exchanged here.
    pub sync_dir: Option<String>,
    pub goals: Vec<Goal>,
    pub next_goal_id: u64,
    pub sharing: Sharing,
    pub rules: Rules,
    pub always_share: Vec<String>,
    pub never_share: Vec<String>,
    /// Free-text "what I'm doing tonight" shown to friends when activity sharing is on.
    pub activity: String,
    pub auto_reload: bool,
    pub ee_log_path: Option<String>,
    pub ee_patterns: Vec<EePattern>,
    pub llm: LlmCfg,
    pub voice: bool,
    pub notify: Notify,
    pub use_market: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            my_name: "Me".into(),
            friend_name: "Friend".into(),
            own_path: None,
            friend_path: None,
            sync_dir: None,
            goals: vec![],
            next_goal_id: 1,
            sharing: Sharing::default(),
            rules: Rules::default(),
            always_share: vec![],
            never_share: vec![],
            activity: String::new(),
            auto_reload: true,
            ee_log_path: None,
            ee_patterns: default_patterns(),
            llm: LlmCfg::default(),
            voice: false,
            notify: Notify::default(),
            use_market: true,
        }
    }
}

/// Best-effort EE.log patterns. Edit them in Settings if a Warframe update changes the wording.
pub fn default_patterns() -> Vec<EePattern> {
    let p = |label: &str, contains: &str, kind: &str| EePattern { label: label.into(), contains: contains.into(), kind: kind.into() };
    vec![
        p("Mission started", "GameRulesImpl - changing state from SS_WAITING_FOR_PLAYERS to SS_STARTED", "mission_start"),
        p("Mission started", "changing state from SS_STARTING to SS_STARTED", "mission_start"),
        p("Mission ended", "changing state from SS_STARTED to SS_ENDING", "mission_end"),
        p("Mission ended", "EndOfMatch.lua", "mission_end"),
        p("Loading into a mission", "ThemedSquadOverlay.lua: Host loading", "note"),
    ]
}

impl Settings {
    pub fn path() -> PathBuf {
        data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_dir());
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), s);
        }
    }

    pub fn add_goal(&mut self, kind: GoalKind, target: &str, name: &str, count: u64) {
        let id = self.next_goal_id.max(1);
        self.next_goal_id = id + 1;
        self.goals.push(Goal { id, kind, target: target.to_string(), name: name.to_string(), count, note: String::new(), shared: true });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_partial_files() {
        let mut s = Settings::default();
        s.add_goal(GoalKind::Build, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", 1);
        let j = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&j).unwrap();
        assert_eq!(back.goals.len(), 1);
        // an old/partial settings file still loads
        let partial: Settings = serde_json::from_str(r#"{"my_name":"Dave"}"#).unwrap();
        assert_eq!(partial.my_name, "Dave");
        assert!(partial.sharing.goals && !partial.sharing.full_inventory);
        assert!(!partial.ee_patterns.is_empty());
    }
}
