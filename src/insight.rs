//! Read-only views over an account: relics, mastery, Foundry.

use crate::account::Account;
use crate::catalog::Catalog;
use crate::goals::{status, GoalStatus};
use crate::settings::Goal;
use crate::trade::{status as item_status, Status};
use crate::util::{blueprint_form, now_ms};
use std::collections::HashSet;

/// Lowercase blueprint/component paths of everything the goals still need to obtain.
pub fn needed_parts(cat: &Catalog, acct: &Account, goals: &[Goal]) -> (HashSet<String>, Vec<GoalStatus>) {
    let mut set = HashSet::new();
    let mut all = Vec::new();
    for g in goals {
        let st = status(cat, acct, g);
        if !st.done {
            for (u, _) in &st.missing_parts {
                set.insert(u.to_lowercase());
                set.insert(blueprint_form(u).to_lowercase());
            }
        }
        all.push(st);
    }
    (set, all)
}

pub struct RelicRewardRow {
    pub unique: String,
    pub name: String,
    pub rarity: String,
    /// The reward is something a goal still needs.
    pub useful: bool,
}

pub struct RelicRow {
    pub unique: String,
    pub name: String,
    pub refinement: &'static str,
    pub count: u64,
    pub rewards: Vec<RelicRewardRow>,
    pub useful: bool,
}

pub fn relic_rows(cat: &Catalog, acct: &Account, needed: &HashSet<String>) -> Vec<RelicRow> {
    let mut rows: Vec<RelicRow> = acct
        .relics()
        .into_iter()
        .filter_map(|(path, count)| {
            let (relic, refinement) = cat.relic(path)?;
            let rewards: Vec<RelicRewardRow> = relic
                .rewards
                .iter()
                .map(|r| RelicRewardRow {
                    unique: r.unique.clone(),
                    name: cat.display(&r.unique),
                    rarity: r.rarity.clone(),
                    useful: needed.contains(&r.unique.to_lowercase()),
                })
                .collect();
            let useful = rewards.iter().any(|r| r.useful);
            Some(RelicRow { unique: path.to_string(), name: relic.name.clone(), refinement, count, rewards, useful })
        })
        .collect();
    rows.sort_by(|a, b| b.useful.cmp(&a.useful).then(a.name.cmp(&b.name)).then(a.refinement.cmp(b.refinement)));
    rows
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MasteryState {
    Mastered,
    Owned,
    NotOwned,
}

pub struct MasteryRow {
    pub unique: String,
    pub name: String,
    pub category: String,
    pub mr_req: u32,
    pub state: MasteryState,
}

pub fn mastery_rows(cat: &Catalog, acct: &Account) -> Vec<MasteryRow> {
    let mut rows: Vec<MasteryRow> = cat
        .items
        .iter()
        .map(|it| {
            let state = match item_status(&acct.prog, &it.unique, it.mastery_xp) {
                Status::Mastered => MasteryState::Mastered,
                Status::Owned => MasteryState::Owned,
                Status::Missing => MasteryState::NotOwned,
            };
            MasteryRow { unique: it.unique.clone(), name: it.name.clone(), category: it.category.clone(), mr_req: it.mr, state }
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

pub struct FoundryRow {
    pub name: String,
    pub unique: String,
    pub remaining_ms: i64,
}

pub fn foundry_rows(cat: &Catalog, acct: &Account) -> Vec<FoundryRow> {
    let now = now_ms();
    acct.foundry
        .iter()
        .map(|j| {
            let result = cat.result_of_blueprint(&j.recipe).unwrap_or(&j.recipe).to_string();
            FoundryRow { name: cat.display(&result), unique: result, remaining_ms: j.done_ms - now }
        })
        .collect()
}

/// Blueprints in the inventory that could be built right now.
pub fn buildable_now(cat: &Catalog, acct: &Account) -> Vec<String> {
    let mut out = Vec::new();
    for (path, e) in &acct.inv {
        if e.count == 0 || !path.contains("/Recipes/") {
            continue;
        }
        let Some(result) = cat.result_of_blueprint(path) else { continue };
        if let Some(res) = crate::recipes::resolve(cat, acct, result, 1) {
            // the blueprint itself is held, so only resources/credits matter
            let others_ok = res.parts_missing().iter().all(|p| p.unique.eq_ignore_ascii_case(result));
            if others_ok && res.resources_missing().is_empty() && acct.credits >= res.credits {
                out.push(result.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_catalog;
    use crate::settings::{GoalKind, Settings};
    use serde_json::json;

    #[test]
    fn relics_flag_useful_rewards() {
        let cat = test_catalog();
        let mut s = Settings::default();
        s.add_goal(GoalKind::Build, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", 1);
        let acct = Account::from_json(&json!({
            "MiscItems": [{"ItemType": "/Lotus/Types/Game/Projections/T4VoidProjectionWispPrimeAGold", "ItemCount": 2}]
        }));
        let (needed, _) = needed_parts(&cat, &acct, &s.goals);
        let rows = relic_rows(&cat, &acct, &needed);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].useful);
        assert_eq!((rows[0].name.as_str(), rows[0].refinement, rows[0].count), ("Axi W1", "Flawless", 2));
    }

    #[test]
    fn mastery_states() {
        let cat = test_catalog();
        let acct = Account::from_json(&json!({
            "Suits": [{"ItemType": "/Lotus/Powersuits/Wisp/WispPrime"}],
            "XPInfo": [{"ItemType": "/Lotus/Powersuits/Wisp/WispPrime", "XP": 900000}]
        }));
        let rows = mastery_rows(&cat, &acct);
        assert!(rows[0].state == MasteryState::Mastered);
    }

    #[test]
    fn foundry_names_use_the_result() {
        let cat = test_catalog();
        let acct = Account::from_json(&json!({
            "PendingRecipes": [{"ItemType": "/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsBlueprint",
                                "CompletionDate": {"$date": {"$numberLong": "4102444800000"}}}]
        }));
        let rows = foundry_rows(&cat, &acct);
        assert_eq!(rows[0].name, "Wisp Prime Systems");
        assert!(rows[0].remaining_ms > 0);
    }
}
