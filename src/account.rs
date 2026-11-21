//! A player's whole state, parsed from a decrypted inventory JSON.

use crate::compare::{flatten, Inventory};
use crate::trade::{progress, Progress};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct FoundryJob {
    /// The recipe (blueprint) being built.
    pub recipe: String,
    /// Unix milliseconds at which it finishes.
    pub done_ms: i64,
}

#[derive(Default)]
pub struct Account {
    pub inv: Inventory,
    pub prog: Progress,
    /// Mastery rank (`PlayerLevel`).
    pub mr: u32,
    pub credits: u64,
    pub platinum: u64,
    pub endo: u64,
    pub foundry: Vec<FoundryJob>,
    lower: HashMap<String, u64>,
}

fn u64_of(v: Option<&Value>) -> u64 {
    v.and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f.max(0.0) as u64))).unwrap_or(0)
}

/// Mongo-style dates: `{"$date":{"$numberLong":"1700000000000"}}` or `{"$date": 1700000000000}`.
pub fn json_ms(v: &Value) -> Option<i64> {
    let d = v.get("$date").unwrap_or(v);
    if let Some(n) = d.as_i64() {
        return Some(n);
    }
    d.get("$numberLong").and_then(Value::as_str).and_then(|s| s.parse().ok())
}

impl Account {
    pub fn from_json(root: &Value) -> Self {
        let inv = flatten(root);
        let lower = inv.iter().map(|(k, e)| (k.to_lowercase(), e.count)).collect();
        let mut foundry = Vec::new();
        for j in root.get("PendingRecipes").and_then(Value::as_array).into_iter().flatten() {
            if let (Some(t), Some(ms)) = (j.get("ItemType").and_then(Value::as_str), j.get("CompletionDate").and_then(json_ms)) {
                foundry.push(FoundryJob { recipe: t.to_string(), done_ms: ms });
            }
        }
        foundry.sort_by_key(|j| j.done_ms);
        Self {
            inv,
            prog: progress(root),
            mr: u64_of(root.get("PlayerLevel")) as u32,
            credits: u64_of(root.get("RegularCredits")),
            platinum: u64_of(root.get("PremiumCredits")) + u64_of(root.get("PremiumCreditsFree")),
            endo: u64_of(root.get("FusionPoints")),
            foundry,
            lower,
        }
    }

    /// Copies of an item (case-insensitive path).
    pub fn count(&self, unique: &str) -> u64 {
        self.lower.get(&unique.to_lowercase()).copied().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.inv.is_empty()
    }

    /// Relic items: (path, copies).
    pub fn relics(&self) -> Vec<(&str, u64)> {
        self.inv
            .iter()
            .filter(|(k, _)| k.contains("/Projections/") && k.contains("VoidProjection"))
            .map(|(k, e)| (k.as_str(), e.count))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_account() {
        let v = json!({
            "PlayerLevel": 28, "RegularCredits": 1000, "PremiumCredits": 20, "PremiumCreditsFree": 5, "FusionPoints": 777,
            "MiscItems": [{"ItemType": "/Lotus/Types/Items/MiscItems/Ferrite", "ItemCount": 12},
                          {"ItemType": "/Lotus/Types/Game/Projections/T1VoidProjectionAAIntact", "ItemCount": 3}],
            "PendingRecipes": [{"ItemType": "/Lotus/Recipes/X", "CompletionDate": {"$date": {"$numberLong": "1700000000000"}}}]
        });
        let a = Account::from_json(&v);
        assert_eq!((a.mr, a.credits, a.platinum, a.endo), (28, 1000, 25, 777));
        assert_eq!(a.count("/lotus/types/items/miscitems/FERRITE"), 12);
        assert_eq!(a.foundry.len(), 1);
        assert_eq!(a.foundry[0].done_ms, 1_700_000_000_000);
        assert_eq!(a.relics().len(), 1);
    }
}
