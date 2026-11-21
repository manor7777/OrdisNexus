//! Goal progress: what each goal still needs, computed from the account.

use crate::account::Account;
use crate::catalog::Catalog;
use crate::recipes::{resolve, Resolution};
use crate::settings::{Goal, GoalKind};

pub struct GoalStatus {
    pub done: bool,
    pub fraction: f32,
    pub summary: String,
    /// Parts still to obtain: (item path, copies).
    pub missing_parts: Vec<(String, u64)>,
    /// Raw resources still short: (item path, amount).
    pub missing_resources: Vec<(String, u64)>,
    pub resolution: Option<Resolution>,
}

pub fn status(cat: &Catalog, acct: &Account, goal: &Goal) -> GoalStatus {
    match goal.kind {
        GoalKind::Mastery => {
            let target = goal.count.max(1);
            let have = acct.mr as u64;
            GoalStatus {
                done: have >= target,
                fraction: (have as f32 / target as f32).min(1.0),
                summary: format!("Mastery rank {have} / {target}"),
                missing_parts: vec![],
                missing_resources: vec![],
                resolution: None,
            }
        }
        GoalKind::Collect => {
            let have = acct.count(&goal.target);
            let need = goal.count.max(1);
            let missing = need.saturating_sub(have);
            GoalStatus {
                done: missing == 0,
                fraction: (have as f32 / need as f32).min(1.0),
                summary: format!("{have} / {need}"),
                missing_parts: vec![],
                missing_resources: if missing > 0 { vec![(goal.target.clone(), missing)] } else { vec![] },
                resolution: None,
            }
        }
        GoalKind::Build => {
            // Already owning the finished item completes the goal.
            let owned = acct.prog.owned.iter().any(|o| o.eq_ignore_ascii_case(&goal.target));
            let Some(res) = resolve(cat, acct, &goal.target, 1) else {
                return GoalStatus {
                    done: owned,
                    fraction: if owned { 1.0 } else { 0.0 },
                    summary: if owned { "Owned".into() } else { "No recipe data for this item".into() },
                    missing_parts: vec![],
                    missing_resources: vec![],
                    resolution: None,
                };
            };
            if owned {
                return GoalStatus {
                    done: true,
                    fraction: 1.0,
                    summary: "Built".into(),
                    missing_parts: vec![],
                    missing_resources: vec![],
                    resolution: Some(res),
                };
            }
            let missing_parts: Vec<(String, u64)> = res.parts_missing().iter().map(|p| (p.unique.clone(), p.to_obtain)).collect();
            let missing_resources: Vec<(String, u64)> = res.resources_missing().iter().map(|r| (r.unique.clone(), r.missing())).collect();
            let summary = if res.buildable(acct) {
                "Ready to build".to_string()
            } else if !missing_parts.is_empty() {
                format!("{} part(s) to obtain", missing_parts.iter().map(|p| p.1).sum::<u64>())
            } else {
                format!("{} resource type(s) short", missing_resources.len())
            };
            GoalStatus { done: false, fraction: res.fraction(), summary, missing_parts, missing_resources, resolution: Some(res) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_catalog;
    use crate::settings::Settings;
    use serde_json::json;

    #[test]
    fn goal_progress() {
        let cat = test_catalog();
        let mut s = Settings::default();
        s.add_goal(GoalKind::Build, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", 1);
        s.add_goal(GoalKind::Collect, "/Lotus/Types/Items/MiscItems/Ferrite", "Ferrite", 100);
        s.add_goal(GoalKind::Mastery, "", "MR", 30);
        let acct = Account::from_json(&json!({
            "PlayerLevel": 12,
            "MiscItems": [{"ItemType": "/Lotus/Types/Items/MiscItems/Ferrite", "ItemCount": 40}]
        }));
        let a = status(&cat, &acct, &s.goals[0]);
        assert!(!a.done);
        assert_eq!(a.missing_parts.len(), 1);
        let b = status(&cat, &acct, &s.goals[1]);
        assert_eq!(b.missing_resources, vec![("/Lotus/Types/Items/MiscItems/Ferrite".to_string(), 60)]);
        let c = status(&cat, &acct, &s.goals[2]);
        assert!((c.fraction - 0.4).abs() < 1e-6);
    }

    #[test]
    fn owning_the_item_completes_the_goal() {
        let cat = test_catalog();
        let mut s = Settings::default();
        s.add_goal(GoalKind::Build, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", 1);
        let acct = Account::from_json(&json!({"Suits": [{"ItemType": "/Lotus/Powersuits/Wisp/WispPrime"}]}));
        assert!(status(&cat, &acct, &s.goals[0]).done);
    }
}
