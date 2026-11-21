//! Flattening an inventory JSON into `item path -> count` and diffing two of them.

use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default, Clone)]
pub struct Entry {
    pub count: u64,
    pub category: String,
}

pub type Inventory = BTreeMap<String, Entry>;

/// Arrays that mention `ItemType` but are history/progress, not things you own.
const IGNORED: &[&str] = &["XPInfo", "LoreFragmentScans", "PersonalTechProjects", "PendingRecipes", "Boosters"];

pub fn category_name(key: &str) -> String {
    match key {
        "Suits" => "Warframes",
        "LongGuns" => "Primary weapons",
        "Pistols" => "Secondary weapons",
        "Melee" => "Melee weapons",
        "RawUpgrades" | "Upgrades" => "Mods",
        "MiscItems" => "Resources & misc",
        "Recipes" => "Blueprints",
        "SpaceSuits" => "Archwings",
        "SpaceGuns" | "SpaceMelee" => "Archwing weapons",
        "Sentinels" => "Sentinels",
        "SentinelWeapons" => "Sentinel weapons",
        "KubrowPets" => "Companions",
        "FlavourItems" => "Cosmetics",
        "WeaponSkins" => "Skins",
        "MechSuits" => "Necramechs",
        "OperatorAmps" => "Amps",
        "CrewShipSalvagedWeapons" | "CrewShipWeapons" => "Railjack weapons",
        other => return pretty_name_raw(other),
    }
    .to_string()
}

/// Every top-level array of objects carrying an `ItemType` is a category
/// (Suits, LongGuns, MiscItems, RawUpgrades, Recipes, ...). Stackables carry
/// `ItemCount`; owned gear counts as one per entry.
pub fn flatten(root: &Value) -> Inventory {
    let mut inv = Inventory::new();
    let Some(obj) = root.as_object() else { return inv };
    for (cat, v) in obj {
        if IGNORED.contains(&cat.as_str()) {
            continue;
        }
        let Some(arr) = v.as_array() else { continue };
        let cat = category_name(cat);
        for item in arr {
            let Some(path) = item.get("ItemType").and_then(Value::as_str) else { continue };
            let n = item.get("ItemCount").and_then(Value::as_u64).unwrap_or(1);
            let e = inv.entry(path.to_string()).or_insert_with(|| Entry {
                count: 0,
                category: cat.clone(),
            });
            e.count += n;
        }
    }
    inv
}

/// `/Lotus/Types/Items/MiscItems/OrokinCatalyst` -> `Orokin Catalyst`
pub fn pretty_name(path: &str) -> String {
    pretty_name_raw(path.rsplit('/').next().unwrap_or(path))
}

fn pretty_name_raw(last: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = last.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && c.is_uppercase() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower) {
                out.push(' ');
            }
        }
        out.push(if c == '_' { ' ' } else { c });
    }
    out
}

pub struct Row {
    pub path: String,
    pub name: String,
    pub category: String,
    pub a: u64,
    pub b: u64,
}

pub fn diff(a: &Inventory, b: &Inventory) -> Vec<Row> {
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut rows: Vec<Row> = keys
        .into_iter()
        .map(|k| {
            let ea = a.get(k);
            let eb = b.get(k);
            Row {
                path: k.clone(),
                name: pretty_name(k),
                category: ea.or(eb).map(|e| e.category.clone()).unwrap_or_default(),
                a: ea.map_or(0, |e| e.count),
                b: eb.map_or(0, |e| e.count),
            }
        })
        .collect();
    rows.sort_by(|x, y| x.category.cmp(&y.category).then(x.name.cmp(&y.name)));
    rows
}

/// Items whose count went up between two inventories: (path, how many more).
pub fn gained(before: &Inventory, after: &Inventory) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = after
        .iter()
        .filter_map(|(k, e)| {
            let b = before.get(k).map_or(0, |x| x.count);
            (e.count > b).then(|| (k.clone(), e.count - b))
        })
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flatten_and_diff() {
        let a = flatten(&json!({
            "MiscItems": [{"ItemType": "/Lotus/Forma", "ItemCount": 5}],
            "Suits": [{"ItemType": "/Lotus/Powersuits/Excalibur/Excalibur"}],
            "PlayerLevel": 30
        }));
        let b = flatten(&json!({"MiscItems": [{"ItemType": "/Lotus/Forma", "ItemCount": 2}]}));
        let rows = diff(&a, &b);
        assert_eq!(rows.len(), 2);
        let forma = rows.iter().find(|r| r.name == "Forma").unwrap();
        assert_eq!((forma.a, forma.b), (5, 2));
        let ex = rows.iter().find(|r| r.name == "Excalibur").unwrap();
        assert_eq!((ex.a, ex.b), (1, 0));
    }

    #[test]
    fn gained_items() {
        let a = flatten(&json!({"MiscItems": [{"ItemType": "/Lotus/Forma", "ItemCount": 5}]}));
        let b = flatten(&json!({"MiscItems": [{"ItemType": "/Lotus/Forma", "ItemCount": 7}, {"ItemType": "/Lotus/Oxium", "ItemCount": 40}]}));
        assert_eq!(gained(&a, &b), vec![("/Lotus/Oxium".to_string(), 40), ("/Lotus/Forma".to_string(), 2)]);
        assert!(gained(&b, &a).is_empty());
    }

    #[test]
    fn names() {
        assert_eq!(pretty_name("/Lotus/Types/Items/MiscItems/OrokinCatalyst"), "Orokin Catalyst");
        assert_eq!(pretty_name("/Lotus/Weapons/MK1Braton"), "MK1 Braton");
    }
}
