//! Works out what it takes to build something: which parts are missing and how
//! many raw resources, credits and hours the rest costs.

use crate::account::Account;
use crate::catalog::Catalog;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct PartRow {
    pub unique: String,
    pub need: u64,
    /// Built copies in the inventory.
    pub built: u64,
    /// Unbuilt blueprints in the inventory.
    pub blueprints: u64,
    /// Copies that still have to be obtained (neither built nor held as blueprint).
    pub to_obtain: u64,
}

#[derive(Clone, Debug)]
pub struct ResRow {
    pub unique: String,
    pub need: u64,
    pub have: u64,
}

impl ResRow {
    pub fn missing(&self) -> u64 {
        self.need.saturating_sub(self.have)
    }
}

#[derive(Default, Clone, Debug)]
pub struct Resolution {
    pub target: String,
    pub root_blueprint: u64,
    pub parts: Vec<PartRow>,
    pub resources: Vec<ResRow>,
    pub credits: u64,
    pub secs: u64,
}

impl Resolution {
    pub fn parts_missing(&self) -> Vec<&PartRow> {
        self.parts.iter().filter(|p| p.to_obtain > 0).collect()
    }

    pub fn resources_missing(&self) -> Vec<&ResRow> {
        self.resources.iter().filter(|r| r.missing() > 0).collect()
    }

    /// True when nothing is missing and the credits are covered.
    pub fn buildable(&self, acct: &Account) -> bool {
        self.parts_missing().is_empty() && self.resources_missing().is_empty() && acct.credits >= self.credits
    }

    /// 0.0..=1.0, averaged over parts and resource totals.
    pub fn fraction(&self) -> f32 {
        let mut total = 0.0f32;
        let mut n = 0.0f32;
        for p in &self.parts {
            total += (p.need.saturating_sub(p.to_obtain)) as f32 / p.need.max(1) as f32;
            n += 1.0;
        }
        for r in &self.resources {
            total += (r.have.min(r.need)) as f32 / r.need.max(1) as f32;
            n += 1.0;
        }
        if n == 0.0 { 1.0 } else { total / n }
    }
}

const MAX_DEPTH: u32 = 4;

/// What building one `target` item needs, given what `acct` owns.
pub fn resolve(cat: &Catalog, acct: &Account, target: &str, qty: u64) -> Option<Resolution> {
    let rec = cat.recipe(target)?;
    let mut out = Resolution { target: rec.result.clone(), root_blueprint: acct.count(&rec.blueprint), ..Default::default() };
    let mut res: BTreeMap<String, u64> = BTreeMap::new();
    out.credits += rec.credits * qty;
    out.secs += rec.secs * qty;
    for (ing, n) in &rec.ingredients {
        expand(cat, acct, ing, n * qty, 1, &mut out, &mut res);
    }
    out.resources = res
        .into_iter()
        .map(|(unique, need)| {
            let have = acct.count(&unique);
            ResRow { unique, need, have }
        })
        .collect();
    out.parts.sort_by(|a, b| a.unique.cmp(&b.unique));
    Some(out)
}

fn expand(cat: &Catalog, acct: &Account, ing: &str, need: u64, depth: u32, out: &mut Resolution, res: &mut BTreeMap<String, u64>) {
    let sub = if depth <= MAX_DEPTH { cat.recipe(ing) } else { None };
    let Some(sub) = sub else {
        *res.entry(ing.to_string()).or_insert(0) += need;
        return;
    };
    let built = acct.count(ing);
    let blueprints = acct.count(&sub.blueprint);
    let from_built = need.min(built);
    let to_craft = need - from_built;
    let to_obtain = to_craft.saturating_sub(blueprints);
    match out.parts.iter_mut().find(|p| p.unique == ing) {
        Some(p) => {
            p.need += need;
            p.to_obtain += to_obtain;
        }
        None => out.parts.push(PartRow { unique: ing.to_string(), need, built, blueprints, to_obtain }),
    }
    if to_craft > 0 {
        out.credits += sub.credits * to_craft;
        out.secs += sub.secs * to_craft;
        for (i2, n2) in &sub.ingredients {
            expand(cat, acct, i2, n2 * to_craft, depth + 1, out, res);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_catalog;
    use serde_json::json;

    const SYSTEMS_BP: &str = "/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsBlueprint";
    const SYSTEMS: &str = "/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsComponent";

    #[test]
    fn missing_part_and_resources() {
        let cat = test_catalog();
        let acct = Account::from_json(&json!({
            "MiscItems": [{"ItemType": "/Lotus/Types/Items/MiscItems/OrokinCell", "ItemCount": 2}]
        }));
        let r = resolve(&cat, &acct, "/Lotus/Powersuits/Wisp/WispPrime", 1).unwrap();
        assert_eq!(r.parts.len(), 1);
        assert_eq!(r.parts[0].to_obtain, 1);
        // crafting the missing component costs resources too
        let ferrite = r.resources.iter().find(|x| x.unique.ends_with("Ferrite")).unwrap();
        assert_eq!((ferrite.need, ferrite.have), (100, 0));
        let cells = r.resources.iter().find(|x| x.unique.ends_with("OrokinCell")).unwrap();
        assert_eq!((cells.need, cells.missing()), (5, 3));
        assert_eq!(r.credits, 25000 + 15000);
        assert!(!r.buildable(&acct));
    }

    #[test]
    fn blueprint_in_hand_counts_but_still_costs_resources() {
        let cat = test_catalog();
        let acct = Account::from_json(&json!({
            "Recipes": [{"ItemType": SYSTEMS_BP, "ItemCount": 1}]
        }));
        let r = resolve(&cat, &acct, "/Lotus/Powersuits/Wisp/WispPrime", 1).unwrap();
        assert_eq!(r.parts[0].blueprints, 1);
        assert_eq!(r.parts[0].to_obtain, 0);
        assert!(r.resources.iter().any(|x| x.unique.ends_with("Circuits")));
    }

    #[test]
    fn built_part_needs_no_resources() {
        let cat = test_catalog();
        let acct = Account::from_json(&json!({
            "MiscItems": [{"ItemType": SYSTEMS, "ItemCount": 1}, {"ItemType": "/Lotus/Types/Items/MiscItems/OrokinCell", "ItemCount": 5}],
            "RegularCredits": 30000
        }));
        let r = resolve(&cat, &acct, "/Lotus/Powersuits/Wisp/WispPrime", 1).unwrap();
        assert!(r.parts_missing().is_empty());
        assert!(r.resources.iter().all(|x| !x.unique.ends_with("Ferrite")));
        assert!(r.buildable(&acct));
        assert!((r.fraction() - 1.0).abs() < 1e-6);
    }
}
