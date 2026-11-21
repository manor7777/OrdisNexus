//! "What should I do?" — ranks activities that move one or several goals forward.

use crate::account::Account;
use crate::catalog::Catalog;
use crate::insight::{needed_parts, relic_rows};
use crate::settings::Settings;
use crate::share::{Matching, Snapshot};
use crate::worldstate::WorldState;
use std::collections::HashMap;

pub struct Rec {
    pub score: u32,
    pub title: String,
    pub detail: String,
}

fn era_of_tier(tier: &str) -> String {
    tier.to_string()
}

pub fn recommend(
    cat: &Catalog,
    acct: &Account,
    s: &Settings,
    world: &WorldState,
    friend: Option<&Snapshot>,
    matching: Option<&Matching>,
) -> Vec<Rec> {
    let mut recs: Vec<Rec> = Vec::new();
    let (needed, statuses) = needed_parts(cat, acct, &s.goals);

    // 1. relics you own that hold something you need, ideally with a matching fissure up right now
    for r in relic_rows(cat, acct, &needed).into_iter().filter(|r| r.useful) {
        let era = r.name.split_whitespace().next().unwrap_or("").to_string();
        let useful: Vec<String> = r.rewards.iter().filter(|x| x.useful).map(|x| format!("{} ({})", x.name, x.rarity)).collect();
        let fissure = world.fissures.iter().find(|f| era_of_tier(&f.tier).eq_ignore_ascii_case(&era) && !f.storm);
        let mut score = 20 + useful.len() as u32 * 5;
        let mut detail = format!("You own {}× {} ({}). It can drop: {}.", r.count, r.name, r.refinement, useful.join(", "));
        if let Some(f) = fissure {
            score += 15;
            detail.push_str(&format!(" A {} fissure is up now: {} at {}.", f.tier, f.mission, f.node));
        }
        recs.push(Rec { score, title: format!("Crack {} ({})", r.name, r.refinement), detail });
    }

    // 2. needed parts you have no relic for
    let owned_with_part = |part: &str| relic_rows(cat, acct, &needed).iter().any(|r| r.rewards.iter().any(|x| x.useful && x.unique.eq_ignore_ascii_case(part)));
    for st in &statuses {
        for (part, n) in &st.missing_parts {
            let bp = crate::util::blueprint_form(part);
            if owned_with_part(&bp) {
                continue;
            }
            let relics = cat.relics_with(&bp);
            if relics.is_empty() {
                continue;
            }
            let list: Vec<String> = relics.iter().take(4).map(|(r, rar)| format!("{} ({rar})", r.name)).collect();
            recs.push(Rec {
                score: 12,
                title: format!("Get {} (need {n})", cat.display(&bp)),
                detail: format!("You don't own a relic with it. It drops from {}.", list.join(", ")),
            });
        }
    }

    // 3. resource farming, merged by place so overlapping goals rank higher
    let mut by_place: HashMap<String, (u32, Vec<String>)> = HashMap::new();
    for st in &statuses {
        for (res, n) in &st.missing_resources {
            let name = cat.display(res);
            let drops = cat.drops_for(&name);
            if drops.is_empty() {
                if let Some(h) = cat.resource_hint(res) {
                    recs.push(Rec { score: 8, title: format!("Farm {name} (short {n})"), detail: h.to_string() });
                }
                continue;
            }
            for d in drops.into_iter().take(3) {
                let e = by_place.entry(d.place.clone()).or_insert((0, vec![]));
                e.0 += 1;
                e.1.push(format!("{name} ({}%)", d.chance));
            }
        }
    }
    for (place, (count, items)) in by_place {
        recs.push(Rec { score: 6 + count * 6, title: format!("Farm at {place}"), detail: format!("Drops: {}.", items.join(", ")) });
    }

    // 4. friends
    if let Some(m) = matching {
        if !m.they_have.is_empty() {
            let who = friend.map(|f| f.name.clone()).unwrap_or_else(|| s.friend_name.clone());
            let items: Vec<String> = m.they_have.iter().map(|(n, c)| format!("{n} ×{c}")).collect();
            recs.push(Rec { score: 40, title: format!("Ask {who} for what you need"), detail: format!("They have available: {}.", items.join(", ")) });
        }
        if m.mutual() {
            let items: Vec<String> = m.i_have.iter().map(|(n, c)| format!("{n} ×{c}")).collect();
            recs.push(Rec { score: 45, title: "Swap parts — you can both help each other".into(), detail: format!("You can give: {}.", items.join(", ")) });
        }
        if !m.shared_farming.is_empty() {
            recs.push(Rec {
                score: 35,
                title: "Farm together".into(),
                detail: format!("You're both short of: {}.", m.shared_farming.join(", ")),
            });
        }
    }

    recs.sort_by(|a, b| b.score.cmp(&a.score).then(a.title.cmp(&b.title)));
    recs.truncate(10);
    recs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_catalog;
    use crate::settings::GoalKind;
    use crate::worldstate::Fissure;
    use serde_json::json;

    #[test]
    fn fissure_boosts_an_owned_relic() {
        let cat = test_catalog();
        let mut s = Settings::default();
        s.add_goal(GoalKind::Build, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", 1);
        let acct = Account::from_json(&json!({
            "MiscItems": [{"ItemType": "/Lotus/Types/Game/Projections/T4VoidProjectionWispPrimeABronze", "ItemCount": 1}]
        }));
        let quiet = recommend(&cat, &acct, &s, &WorldState::default(), None, None);
        assert!(quiet[0].title.starts_with("Crack Axi W1"));
        let world = WorldState {
            fissures: vec![Fissure { node: "Hepit (Void)".into(), mission: "Capture".into(), tier: "Axi".into(), ..Default::default() }],
            ..Default::default()
        };
        let busy = recommend(&cat, &acct, &s, &world, None, None);
        assert!(busy[0].score > quiet[0].score);
        assert!(busy[0].detail.contains("Hepit"));
    }

    #[test]
    fn friend_help_ranks_first() {
        let cat = test_catalog();
        let s = Settings::default();
        let acct = Account::default();
        let m = Matching { they_have: vec![("Wisp Prime Systems".into(), 1)], ..Default::default() };
        let r = recommend(&cat, &acct, &s, &WorldState::default(), None, Some(&m));
        assert!(r[0].title.starts_with("Ask"));
    }
}
