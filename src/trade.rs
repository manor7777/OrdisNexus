//! Prime-part trade finder.
//!
//! For every prime item we work out, per player, whether the item is
//! `Mastered`, `Owned` (built, not yet mastered) or `Missing`, then decide who
//! can hand which parts to whom.

use crate::catalog::Catalog;
use crate::compare::Inventory;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Missing,
    Owned,
    Mastered,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Giver has mastered the item: parts are safe to hand over.
    Free,
    /// Giver has not mastered it (spare copies only): up to the players.
    YourCall,
}

/// What we need from a player's save besides item counts.
#[derive(Default)]
pub struct Progress {
    pub xp: HashMap<String, u64>,
    pub owned: HashSet<String>,
}

const OWNED_ARRAYS: &[&str] = &[
    "Suits", "LongGuns", "Pistols", "Melee", "SpaceSuits", "SpaceGuns", "SpaceMelee", "Sentinels",
    "KubrowPets", "MechSuits", "SentinelWeapons", "Horses",
];

pub fn progress(root: &Value) -> Progress {
    let mut p = Progress::default();
    let Some(obj) = root.as_object() else { return p };
    if let Some(arr) = obj.get("XPInfo").and_then(Value::as_array) {
        for x in arr {
            if let (Some(t), Some(xp)) = (x.get("ItemType").and_then(Value::as_str), x.get("XP").and_then(Value::as_u64)) {
                let e = p.xp.entry(t.to_string()).or_insert(0);
                *e = (*e).max(xp);
            }
        }
    }
    for key in OWNED_ARRAYS {
        for x in obj.get(*key).and_then(Value::as_array).into_iter().flatten() {
            if let Some(t) = x.get("ItemType").and_then(Value::as_str) {
                p.owned.insert(t.to_string());
            }
        }
    }
    p
}

pub fn status(p: &Progress, unique: &str, mastery_xp: u64) -> Status {
    if p.xp.get(unique).is_some_and(|&x| x >= mastery_xp) {
        Status::Mastered
    } else if p.owned.contains(unique) {
        Status::Owned
    } else {
        Status::Missing
    }
}

pub struct Transfer {
    pub prime: usize,
    pub part: usize,
    /// Index (0/1) of the player who gives.
    pub from: usize,
    pub count: u32,
    pub kind: Kind,
}

pub struct Sell {
    pub prime: usize,
    pub part: usize,
    pub who: usize,
    pub count: u32,
}

pub struct PartNeed {
    pub part: usize,
    pub have: [u32; 2],
    /// Copies still missing per player (0 when the item is owned/mastered).
    pub missing: [u32; 2],
}

/// A prime item at least one player still has to build.
pub struct PrimeNeed {
    pub prime: usize,
    pub status: [Status; 2],
    pub parts: Vec<PartNeed>,
}

#[derive(Default)]
pub struct Plan {
    pub transfers: Vec<Transfer>,
    pub sells: Vec<Sell>,
    pub needs: Vec<PrimeNeed>,
    /// Per prime: status for each player.
    pub status: Vec<[Status; 2]>,
}

fn count_of(inv: &Inventory, unique: &str) -> u64 {
    inv.get(unique).map_or(0, |e| e.count)
}

/// `(copies that count as having the part, copies that can be handed over)`.
fn have_give(inv: &Inventory, unique: &str) -> (u32, u32) {
    let built = count_of(inv, unique);
    match unique.strip_suffix("Component") {
        Some(base) => {
            let bp = count_of(inv, &format!("{base}Blueprint"));
            ((built + bp) as u32, bp as u32)
        }
        None => (built as u32, built as u32),
    }
}

pub fn plan(cat: &Catalog, inv: [&Inventory; 2], prog: [&Progress; 2]) -> Plan {
    let mut plan = Plan::default();
    for (pi, prime) in cat.primes.iter().enumerate() {
        let st = [
            status(prog[0], &prime.unique, prime.mastery_xp),
            status(prog[1], &prime.unique, prime.mastery_xp),
        ];
        plan.status.push(st);
        let mut need_parts = Vec::new();
        for (pt, part) in prime.parts.iter().enumerate() {
            // Parts you hold as an unbuilt blueprint or as a built component both
            // count as "having" the part, but only blueprints can be handed over.
            let hg = [have_give(inv[0], &part.unique), have_give(inv[1], &part.unique)];
            let c = [hg[0].0, hg[1].0];
            let give_cap = [hg[0].1, hg[1].1];
            let missing = [0, 1].map(|i| if st[i] == Status::Missing { part.need.saturating_sub(c[i]) } else { 0 });
            if missing != [0, 0] {
                need_parts.push(PartNeed { part: pt, have: c, missing });
            }
            if c == [0, 0] {
                continue;
            }
            for x in 0..2 {
                let y = 1 - x;
                let need_y = if st[y] == Status::Missing { part.need.saturating_sub(c[y]) } else { 0 };
                let spare_x = (if st[x] == Status::Missing { c[x].saturating_sub(part.need) } else { c[x] }).min(give_cap[x]);
                let give = spare_x.min(need_y);
                if give > 0 {
                    let kind = if st[x] == Status::Mastered { Kind::Free } else { Kind::YourCall };
                    plan.transfers.push(Transfer { prime: pi, part: pt, from: x, count: give, kind });
                }
                let leftover = spare_x - give;
                if st[x] == Status::Mastered && leftover > 0 {
                    plan.sells.push(Sell { prime: pi, part: pt, who: x, count: leftover });
                }
            }
        }
        if !need_parts.is_empty() {
            plan.needs.push(PrimeNeed { prime: pi, status: st, parts: need_parts });
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::{flatten, Inventory};
    use serde_json::json;

    const NAMI: &str = "/Lotus/Weapons/Tenno/Melee/Swords/PrimeNamiSkyla/PrimeNamiSkyla";
    const HANDLE: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeNamiSkylaHandle";

    fn cat() -> Catalog {
        Catalog::from_json(&format!(
            r#"{{"names":{{}},"icons":{{}},"primes":[{{"u":"{NAMI}","n":"Nami Skyla Prime","x":450000,
                "p":[["{HANDLE}","Nami Skyla Prime Handle",2]]}}]}}"#
        ))
        .unwrap()
    }

    fn player(handles: u64, xp: u64) -> (Inventory, Progress) {
        let v = json!({"MiscItems":[{"ItemType":HANDLE,"ItemCount":handles}],"XPInfo":[{"ItemType":NAMI,"XP":xp}]});
        (flatten(&v), progress(&v))
    }

    #[test]
    fn mastered_gives_everything() {
        let (ia, pa) = player(3, 450000);
        let (ib, pb) = player(0, 0);
        let p = plan(&cat(), [&ia, &ib], [&pa, &pb]);
        assert_eq!(p.transfers.len(), 1);
        let t = &p.transfers[0];
        assert_eq!((t.from, t.count, t.kind), (0, 2, Kind::Free));
        // 1 left over after B's 2 needed copies -> free to sell
        assert_eq!((p.sells[0].who, p.sells[0].count), (0, 1));
    }

    #[test]
    fn needs_only_for_players_who_lack_the_item() {
        let (ia, pa) = player(1, 450000); // mastered: needs nothing
        let (ib, pb) = player(1, 0); // has 1 of 2 handles
        let p = plan(&cat(), [&ia, &ib], [&pa, &pb]);
        assert_eq!(p.needs.len(), 1);
        assert_eq!(p.needs[0].parts[0].missing, [0, 1]);
        let (ic, pc) = player(0, 0);
        let p = plan(&cat(), [&ib, &ic], [&pb, &pc]);
        assert_eq!(p.needs[0].parts[0].missing, [1, 2]); // both need it
    }

    #[test]
    fn both_mastered_is_free_to_sell() {
        let (ia, pa) = player(1, 900000);
        let (ib, pb) = player(4, 450000);
        let p = plan(&cat(), [&ia, &ib], [&pa, &pb]);
        assert!(p.transfers.is_empty());
        assert_eq!(p.sells.len(), 2);
    }

    #[test]
    fn neither_mastered_only_spares_and_your_call() {
        let (ia, pa) = player(3, 0); // needs 2, spare 1
        let (ib, pb) = player(1, 0); // needs 1 more
        let p = plan(&cat(), [&ia, &ib], [&pa, &pb]);
        assert_eq!(p.transfers.len(), 1);
        assert_eq!((p.transfers[0].from, p.transfers[0].count, p.transfers[0].kind), (0, 1, Kind::YourCall));
        assert!(p.sells.is_empty());
    }
}
