//! "What am I close to completing?" Ranks things you could finish soon and says
//! exactly what is missing and where to get it, using only the stored catalog
//! (recipes, relic tables, drop tables, resource hints) and your inventory.

use crate::account::Account;
use crate::catalog::Catalog;
use crate::recipes::resolve;
use crate::settings::{Goal, GoalKind};
use crate::util::{blueprint_form, fmt_n};
use std::collections::HashSet;

pub struct MissingPart {
    pub name: String,
    pub count: u64,
    /// Where it comes from, already phrased.
    pub how: String,
}

pub struct MissingRes {
    pub name: String,
    pub need: u64,
    pub have: u64,
    pub how: String,
}

pub struct Close {
    pub unique: String,
    pub name: String,
    pub fraction: f32,
    pub is_goal: bool,
    pub parts: Vec<MissingPart>,
    pub resources: Vec<MissingRes>,
    pub credits_short: u64,
    pub buildable: bool,
}

impl Close {
    /// Smaller is closer.
    fn effort(&self) -> u64 {
        let parts: u64 = self.parts.iter().map(|p| p.count).sum();
        parts * 100 + self.resources.len() as u64 * 8 + u64::from(self.credits_short > 0) * 5
    }
}

fn rarity_word(r: &str) -> &str {
    match r.chars().next() {
        Some('c') | Some('C') => "common",
        Some('u') | Some('U') => "uncommon",
        Some('r') | Some('R') => "rare",
        _ => r,
    }
}

/// How to obtain a Prime part: relics that drop it (marking ones you already hold).
pub fn how_part(cat: &Catalog, acct: &Account, part_unique: &str) -> String {
    let bp = blueprint_form(part_unique);
    let relics = cat.relics_with(&bp);
    if relics.is_empty() {
        return "no relic source on file (check the Prime Vault or the item's wiki page)".into();
    }
    let held: HashSet<String> = acct
        .relics()
        .into_iter()
        .filter_map(|(k, _)| cat.relic(k).map(|(r, _)| r.name.clone()))
        .collect();
    let mut sorted: Vec<_> = relics.iter().collect();
    sorted.sort_by_key(|(r, _)| !held.contains(&r.name));
    let list: Vec<String> = sorted
        .iter()
        .take(4)
        .map(|(r, rar)| {
            let own = if held.contains(&r.name) { ", you own it" } else { "" };
            format!("{} ({}{})", r.name, rarity_word(rar), own)
        })
        .collect();
    let more = relics.len().saturating_sub(4);
    let tail = if more > 0 { format!(" and {more} more") } else { String::new() };
    format!("relics: {}{}", list.join(", "), tail)
}

/// How to obtain a resource or other item: our hint text, else best drop tables.
pub fn how_item(cat: &Catalog, unique: &str) -> String {
    if let Some(h) = cat.resource_hint(unique) {
        return h.to_string();
    }
    let name = cat.display(unique);
    let drops = cat.drops_for(&name);
    if drops.is_empty() {
        return "no source on file".into();
    }
    let list: Vec<String> =
        drops.iter().take(3).map(|d| format!("{} ({:.1}%)", d.place, d.chance)).collect();
    format!("drops from {}", list.join("; "))
}

fn build_close(cat: &Catalog, acct: &Account, unique: &str, name: &str, is_goal: bool) -> Option<Close> {
    let res = resolve(cat, acct, unique, 1)?;
    let parts: Vec<MissingPart> = res
        .parts_missing()
        .into_iter()
        .map(|p| MissingPart { name: cat.display(&blueprint_form(&p.unique)), count: p.to_obtain, how: how_part(cat, acct, &p.unique) })
        .collect();
    let resources: Vec<MissingRes> = res
        .resources_missing()
        .into_iter()
        .map(|r| MissingRes { name: cat.display(&r.unique), need: r.need, have: r.have, how: how_item(cat, &r.unique) })
        .collect();
    Some(Close {
        unique: unique.to_string(),
        name: name.to_string(),
        fraction: res.fraction(),
        is_goal,
        parts,
        resources,
        credits_short: res.credits.saturating_sub(acct.credits),
        buildable: res.buildable(acct),
    })
}

/// Things you could finish soon, closest first. Includes unfinished Build goals
/// and every Prime you already hold at least one part of.
pub fn close_to_completing(cat: &Catalog, acct: &Account, goals: &[Goal], limit: usize) -> Vec<Close> {
    let owned = |u: &str| acct.prog.owned.iter().any(|o| o.eq_ignore_ascii_case(u));
    let mut out: Vec<Close> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for g in goals.iter().filter(|g| matches!(g.kind, GoalKind::Build) && !g.target.is_empty()) {
        if owned(&g.target) || !seen.insert(g.target.to_lowercase()) {
            continue;
        }
        if let Some(c) = build_close(cat, acct, &g.target, &g.name, true) {
            out.push(c);
        }
    }
    for p in &cat.primes {
        if owned(&p.unique) || !seen.insert(p.unique.to_lowercase()) {
            continue;
        }
        if let Some(c) = build_close(cat, acct, &p.unique, &p.name, false) {
            // only worth mentioning if you have made some progress
            if c.fraction > 0.0 {
                out.push(c);
            }
        }
    }
    out.sort_by(|a, b| a.effort().cmp(&b.effort()).then(b.fraction.total_cmp(&a.fraction)).then(a.name.cmp(&b.name)));
    out.truncate(limit);
    out
}

pub fn describe(c: &Close, detailed: bool) -> String {
    let mut lines = Vec::new();
    let tag = if c.is_goal { " (a goal)" } else { "" };
    if c.buildable {
        lines.push(format!("• {}{} — you have everything. Start the build in the Foundry.", c.name, tag));
        return lines.join("\n");
    }
    lines.push(format!("• {}{} — {:.0}% there", c.name, tag, c.fraction * 100.0));
    if detailed {
        for p in &c.parts {
            lines.push(format!("    needs {} ×{} → {}", p.name, p.count, p.how));
        }
        for r in c.resources.iter().take(6) {
            lines.push(format!("    short {}: need {}, have {} → {}", r.name, fmt_n(r.need), fmt_n(r.have), r.how));
        }
        if c.credits_short > 0 {
            lines.push(format!("    short {} credits", fmt_n(c.credits_short)));
        }
    } else {
        let mut bits = Vec::new();
        if !c.parts.is_empty() {
            bits.push(format!("{} part(s) to find", c.parts.iter().map(|p| p.count).sum::<u64>()));
        }
        if !c.resources.is_empty() {
            bits.push(format!("{} resource(s) short", c.resources.len()));
        }
        if c.credits_short > 0 {
            bits.push("credits short".into());
        }
        lines[0] = format!("{} — {}", lines[0], bits.join(", "));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_catalog;
    use serde_json::json;

    #[test]
    fn explains_where_to_get_missing_parts() {
        let cat = test_catalog();
        let acct = Account::from_json(&json!({
            "RegularCredits": 1000000,
            "Recipes": [{"ItemType": "/Lotus/Types/Recipes/WarframeRecipes/WispPrimeBlueprint", "ItemCount": 1}],
            "MiscItems": [
                {"ItemType": "/Lotus/Types/Items/MiscItems/Ferrite", "ItemCount": 500},
                {"ItemType": "/Lotus/Types/Game/Projections/T4VoidProjectionWispPrimeAIntact", "ItemCount": 2}
            ]
        }));
        let goals: Vec<Goal> = vec![];
        let list = close_to_completing(&cat, &acct, &goals, 5);
        // The test catalog has Wisp Prime in its prime list only if the data says so,
        // so just make sure the ranking and text never panic and stay consistent.
        for c in &list {
            let text = describe(c, true);
            assert!(text.contains(&c.name));
        }
        let c = build_close(&cat, &acct, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", true).unwrap();
        assert!(!c.parts.is_empty());
        assert!(c.parts.iter().any(|p| p.how.contains("Axi W1")), "{:?}", c.parts.iter().map(|p| p.how.clone()).collect::<Vec<_>>());
    }

    #[test]
    fn closer_items_rank_first() {
        let near = Close { unique: "a".into(), name: "A".into(), fraction: 0.9, is_goal: false, parts: vec![MissingPart { name: "x".into(), count: 1, how: String::new() }], resources: vec![], credits_short: 0, buildable: false };
        let far = Close { unique: "b".into(), name: "B".into(), fraction: 0.2, is_goal: false, parts: vec![MissingPart { name: "y".into(), count: 4, how: String::new() }], resources: vec![], credits_short: 0, buildable: false };
        assert!(near.effort() < far.effort());
    }
}
