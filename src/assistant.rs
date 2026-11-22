//! Ordis: answers questions by calling deterministic tools over real data.
//! The text is always built from facts; an optional language model only
//! rephrases it, so it can never invent an inventory.

use crate::account::Account;
use crate::catalog::Catalog;
use crate::goals::status;
use crate::insight::{buildable_now, foundry_rows, mastery_rows, needed_parts, relic_rows, MasteryState};
use crate::recipes::resolve;
use crate::recommend::recommend;
use crate::settings::{LlmCfg, Settings};
use crate::share::{friend_has, Avail, Matching, Snapshot};
use crate::util::{fmt_duration, fmt_n};
use crate::worldstate::WorldState;
use serde_json::{json, Value};
use std::time::Duration;

pub struct Ctx<'a> {
    pub cat: &'a Catalog,
    pub me: Option<&'a Account>,
    pub friend: Option<&'a Snapshot>,
    pub matching: Option<&'a Matching>,
    pub my_avail: &'a [Avail],
    pub settings: &'a Settings,
    pub world: &'a WorldState,
}

fn has(q: &str, words: &[&str]) -> bool {
    words.iter().any(|w| q.contains(w))
}

pub const HELP: &str = "Ordis can answer things like:\n• What am I close to completing?\n• What am I missing for Wisp Prime?\n• What should I farm tonight?\n• Does my friend have Wisp Prime Systems?\n• Can my friend help me?\n• What can I build right now?\n• Which relics should I crack?\n• What are my goals?\n• How much Ferrite do I have?\n• Any good fissures / invasions? Where is Baro?\n• What's my mastery progress? What's in the Foundry?";

/// The grounded answer. Always safe to show as-is.
pub fn answer(c: &Ctx, question: &str) -> String {
    let q = question.to_lowercase();
    let Some(me) = c.me else {
        return "Operator, Ordis cannot see your inventory yet. Load your lastData.dat from the Overview tab, and Ordis will have all the answers.".into();
    };
    let friend_word = c.settings.friend_name.to_lowercase();
    let mentions_friend = q.contains("friend") || (!friend_word.is_empty() && friend_word != "friend" && q.contains(&friend_word));
    let mentioned = c.cat.mentioned(&q, 3);

    if has(&q, &["hello", "hi ordis", "hey ordis"]) || q.trim() == "hi" {
        return "Operator! Ordis is online and fully operational. Ask away.".into();
    }
    if has(&q, &["what can you", "help me with", "what do you do"]) || q.trim() == "help" {
        return HELP.into();
    }

    // friends
    if mentions_friend && has(&q, &["help", "need from", "trade", "swap"]) && !has(&q, &["have ", "has "]) {
        return friend_help(c);
    }
    if (mentions_friend || has(&q, &["anyone", "anybody"])) && has(&q, &["have", "has", "got", "spare"]) {
        return match (c.friend, mentioned.first()) {
            (None, _) => "Ordis has no friend snapshot loaded. Open the Friends tab and load one, or set a shared folder in Settings.".into(),
            (Some(f), Some((_, name))) => {
                let hits = friend_has(f, name.trim_end_matches(" Blueprint"));
                if hits.is_empty() {
                    format!("{} has not made anything like {name} available.", f.name)
                } else {
                    let list: Vec<String> = hits.iter().map(|a| format!("{} ×{}", a.name, a.count)).collect();
                    format!("{} has available: {}.", f.name, list.join(", "))
                }
            }
            (Some(_), None) => friend_help(c),
        };
    }
    if has(&q, &["can i help", "what do i have that", "what can i give"]) {
        return friend_help(c);
    }

    // build / missing
    if has(&q, &["missing", "need", "build", "make ", "require", "why can't i", "why cant i"]) && !has(&q, &["what can i build", "can i build"]) {
        if let Some(item) = c.cat.mentioned_item(&q).or_else(|| mentioned.iter().find_map(|(u, _)| c.cat.item(u))) {
            return build_report(c, me, &item.unique, &item.name);
        }
        if let Some((u, name)) = mentioned.first() {
            if c.cat.recipe(u).is_some() {
                return build_report(c, me, u, name);
            }
        }
    }
    if has(&q, &["what can i build", "can i build", "ready to build"]) {
        let list = buildable_now(c.cat, me);
        return if list.is_empty() {
            "Nothing in the Foundry queue is ready: Ordis found no blueprint you hold with all resources and credits available.".into()
        } else {
            format!("You can build right now: {}.", list.iter().map(|u| c.cat.display(u)).collect::<Vec<_>>().join(", "))
        };
    }

    // closest things to finish
    if has(&q, &["close to", "closest", "nearly", "almost", "next to finish", "what can i finish", "what am i close"]) {
        let list = crate::close::close_to_completing(c.cat, me, &c.settings.goals, 4);
        if list.is_empty() {
            return "Ordis cannot see anything you are partway through yet. Add a goal in the Goals tab, or get a Prime part and ask again.".into();
        }
        let detailed = has(&q, &["how", "where", "detail", "exactly", "get"]);
        let lines: Vec<String> = list.iter().enumerate().map(|(i, x)| crate::close::describe(x, detailed || i == 0)).collect();
        let tail = if detailed { "" } else { "
Ask 'how do I get them' for the full list of sources." };
        return format!("You are closest to finishing:\n{}{}", lines.join("\n"), tail);
    }

    // what to do
    if has(&q, &["farm", "tonight", "what should i do", "what should we do", "recommend", "worth doing"]) {
        let recs = recommend(c.cat, me, c.settings, c.world, c.friend, c.matching);
        if recs.is_empty() {
            return "Ordis has nothing urgent. Add some goals in the Goals tab and Ordis will plan around them.".into();
        }
        let lines: Vec<String> = recs.iter().take(6).map(|r| format!("• {} — {}", r.title, r.detail)).collect();
        return format!("Here is what moves your goals forward:\n{}", lines.join("\n"));
    }

    if has(&q, &["relic"]) {
        let (needed, _) = needed_parts(c.cat, me, &c.settings.goals);
        let rows: Vec<_> = relic_rows(c.cat, me, &needed).into_iter().filter(|r| r.useful).collect();
        return if rows.is_empty() {
            "None of your relics hold a part your goals need right now.".into()
        } else {
            let lines: Vec<String> = rows
                .iter()
                .take(8)
                .map(|r| {
                    let useful: Vec<&str> = r.rewards.iter().filter(|x| x.useful).map(|x| x.name.as_str()).collect();
                    format!("• {} ({}) ×{} — {}", r.name, r.refinement, r.count, useful.join(", "))
                })
                .collect();
            format!("Relics worth cracking:\n{}", lines.join("\n"))
        };
    }

    if has(&q, &["goal", "progress"]) && !q.contains("mastery") {
        if c.settings.goals.is_empty() {
            return "You have no goals yet. Add one in the Goals tab, for example 'Wisp Prime'.".into();
        }
        let lines: Vec<String> = c
            .settings
            .goals
            .iter()
            .map(|g| {
                let st = status(c.cat, me, g);
                format!("• {} — {} ({:.0}%)", g.name, st.summary, st.fraction * 100.0)
            })
            .collect();
        return format!("Your goals:\n{}", lines.join("\n"));
    }

    // world
    if has(&q, &["fissure"]) {
        if c.world.fissures.is_empty() {
            return "Ordis has no fissure data yet. Open the World tab to refresh.".into();
        }
        let lines: Vec<String> = c
            .world
            .fissures
            .iter()
            .filter(|f| !f.storm)
            .take(12)
            .map(|f| format!("• {} {} — {}{}", f.tier, f.mission, f.node, if f.hard { " (Steel Path)" } else { "" }))
            .collect();
        return format!("Active fissures:\n{}", lines.join("\n"));
    }
    if has(&q, &["invasion"]) {
        let lines: Vec<String> = c.world.invasions.iter().take(8).map(|i| format!("• {} ({}): {} vs {}", i.node, i.desc, i.attacker, i.defender)).collect();
        return if lines.is_empty() { "No invasions reported.".into() } else { format!("Invasions:\n{}", lines.join("\n")) };
    }
    if has(&q, &["baro", "void trader"]) {
        return c.world.baro.clone().unwrap_or_else(|| "Ordis has no word on Baro yet.".into());
    }
    if has(&q, &["sortie"]) {
        return if c.world.sortie.is_empty() { "No sortie data yet.".into() } else { format!("Today's sortie:\n• {}", c.world.sortie.join("\n• ")) };
    }
    if has(&q, &["cycle", "night", "day ", "cetus", "vallis", "cambion"]) {
        let lines: Vec<String> = c
            .world
            .cycles
            .iter()
            .map(|y| format!("• {}: {} ({} left)", y.name, y.state, fmt_duration(y.expiry_ms - crate::util::now_ms())))
            .collect();
        return if lines.is_empty() { "No cycle data yet.".into() } else { format!("World cycles:\n{}", lines.join("\n")) };
    }

    if has(&q, &["mastery", "mastered", "mr "]) {
        let rows = mastery_rows(c.cat, me);
        let m = rows.iter().filter(|r| r.state == MasteryState::Mastered).count();
        let owned = rows.iter().filter(|r| r.state == MasteryState::Owned).count();
        return format!("You are Mastery Rank {}. {m} items mastered, {owned} owned and waiting to be levelled, {} not yet owned.", me.mr, rows.len() - m - owned);
    }
    if has(&q, &["foundry", "building", "crafting"]) {
        let rows = foundry_rows(c.cat, me);
        return if rows.is_empty() {
            "The Foundry is empty.".into()
        } else {
            let lines: Vec<String> = rows
                .iter()
                .map(|r| if r.remaining_ms <= 0 { format!("• {} — ready to claim", r.name) } else { format!("• {} — {} left", r.name, fmt_duration(r.remaining_ms)) })
                .collect();
            format!("In the Foundry:\n{}", lines.join("\n"))
        };
    }

    // plain inventory questions
    if has(&q, &["how much", "how many", "do i have", "have i got", "count"]) {
        if let Some((u, name)) = mentioned.first() {
            let built = me.count(u);
            let bp = me.count(&crate::util::blueprint_form(u));
            return if bp > 0 && bp != built {
                format!("You have {} {name} built and {} as unbuilt blueprints.", fmt_n(built), fmt_n(bp))
            } else {
                format!("You have {} × {name}.", fmt_n(built))
            };
        }
    }

    // a bare item name: tell them what it needs
    if let Some(item) = c.cat.mentioned_item(&q).filter(|_| q.split_whitespace().count() <= 4) {
        return build_report(c, me, &item.unique, &item.name);
    }

    format!("Ordis did not quite catch that, Operator.\n\n{HELP}")
}

fn friend_help(c: &Ctx) -> String {
    let (Some(f), Some(m)) = (c.friend, c.matching) else {
        return "Ordis has no friend snapshot to compare with. Load one in the Friends tab.".into();
    };
    let mut parts = Vec::new();
    if !m.they_have.is_empty() {
        let l: Vec<String> = m.they_have.iter().map(|(n, k)| format!("{n} ×{k}")).collect();
        parts.push(format!("{} has what you need: {}.", f.name, l.join(", ")));
    }
    if !m.i_have.is_empty() {
        let l: Vec<String> = m.i_have.iter().map(|(n, k)| format!("{n} ×{k}")).collect();
        parts.push(format!("You can help {} with: {}.", f.name, l.join(", ")));
    }
    if m.mutual() {
        parts.push("You can both help each other. That's a swap.".into());
    }
    if !m.shared_farming.is_empty() {
        parts.push(format!("You are both short of {}, so you could farm it together.", m.shared_farming.join(", ")));
    }
    if parts.is_empty() {
        format!("No overlap with {} right now. Neither of you has what the other needs.", f.name)
    } else {
        parts.join("\n")
    }
}

fn build_report(c: &Ctx, me: &Account, unique: &str, name: &str) -> String {
    let Some(res) = resolve(c.cat, me, unique, 1) else {
        return format!("Ordis has no recipe on file for {name}.");
    };
    let mut out = Vec::new();
    let parts = res.parts_missing();
    if parts.is_empty() {
        out.push(format!("For {name}: every part is accounted for."));
    } else {
        out.push(format!("For {name} you still need to obtain:"));
        for p in parts {
            let bp = crate::util::blueprint_form(&p.unique);
            let relics = c.cat.relics_with(&bp);
            let src = if relics.is_empty() {
                String::new()
            } else {
                format!(" — relics: {}", relics.iter().take(3).map(|(r, rar)| format!("{} ({rar})", r.name)).collect::<Vec<_>>().join(", "))
            };
            out.push(format!("• {} ×{}{src}", c.cat.display(&bp), p.to_obtain));
        }
    }
    let short = res.resources_missing();
    if !short.is_empty() {
        out.push("Resources short:".into());
        for r in short.iter().take(12) {
            out.push(format!("• {} — need {}, have {}", c.cat.display(&r.unique), fmt_n(r.need), fmt_n(r.have)));
        }
    }
    if me.credits < res.credits {
        out.push(format!("Credits: {} needed, you have {}.", fmt_n(res.credits), fmt_n(me.credits)));
    }
    if res.buildable(me) {
        out.push("You have everything. Start the build in the Foundry.".into());
    }
    out.join("\n")
}

/// Ordis' voice: a short framing line, never altering the facts below it.
pub fn with_flavor(text: &str, n: usize) -> String {
    const OPENERS: [&str; 4] = ["Operator, Ordis has checked.", "Ordis has consulted the records.", "Right away, Operator.", "Ordis is delighted to help."];
    if text.starts_with("Operator") || text.starts_with("Ordis") || text.contains(HELP) {
        return text.to_string();
    }
    format!("{}\n{}", OPENERS[n % OPENERS.len()], text)
}

pub fn llm_system_prompt() -> &'static str {
    "You are Ordis, the cheerful, slightly eccentric cephalon from Warframe. You are given FACTS computed from the player's real data. \
Rewrite them as a short, friendly reply in Ordis' voice. Use ONLY the facts provided. Never invent items, quantities, names or locations. \
Keep every number and item name exactly as given. Keep bullet lists as lists."
}

pub fn ask_llm(cfg: &LlmCfg, question: &str, facts: &str) -> Result<String, String> {
    let url = format!("{}/chat/completions", cfg.endpoint.trim_end_matches('/'));
    let body = json!({
        "model": cfg.model,
        "temperature": 0.3,
        "messages": [
            {"role": "system", "content": llm_system_prompt()},
            {"role": "user", "content": format!("Question: {question}\n\nFACTS:\n{facts}")}
        ]
    })
    .to_string();
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(60))).build().into();
    let mut req = agent.post(&url).header("Content-Type", "application/json");
    if !cfg.api_key.is_empty() {
        req = req.header("Authorization", &format!("Bearer {}", cfg.api_key));
    }
    let mut resp = req.send(body.as_bytes()).map_err(|e| format!("language model unreachable: {e}"))?;
    let bytes = resp.body_mut().read_to_vec().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    v["choices"][0]["message"]["content"].as_str().map(|s| s.trim().to_string()).ok_or_else(|| "no answer in the model's reply".into())
}

/// Reads text aloud with Windows' built-in voice. Does nothing elsewhere.
pub fn speak(text: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let clean: String = text.chars().filter(|c| !matches!(c, '\'' | '"' | '`' | '$' | '•')).collect();
        let script = format!("Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('{clean}')");
        let _ = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).creation_flags(0x0800_0000).spawn();
    }
    #[cfg(not(windows))]
    let _ = text;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_catalog;
    use crate::settings::GoalKind;
    use serde_json::json;

    fn ask(q: &str, acct: &Account, friend: Option<&Snapshot>, matching: Option<&Matching>, s: &Settings) -> String {
        let cat = test_catalog();
        let world = WorldState::default();
        let ctx = Ctx { cat: &cat, me: Some(acct), friend, matching, my_avail: &[], settings: s, world: &world };
        answer(&ctx, q)
    }

    #[test]
    fn missing_for_item() {
        let acct = Account::from_json(&json!({"MiscItems": [{"ItemType": "/Lotus/Types/Items/MiscItems/OrokinCell", "ItemCount": 2}]}));
        let a = ask("What am I missing for Wisp Prime?", &acct, None, None, &Settings::default());
        assert!(a.contains("Wisp Prime Systems"), "{a}");
        assert!(a.contains("Axi W1") || a.contains("relics"), "{a}");
        assert!(a.contains("need 5, have 2"), "{a}");
    }

    #[test]
    fn counts_and_goals() {
        let acct = Account::from_json(&json!({"MiscItems": [{"ItemType": "/Lotus/Types/Items/MiscItems/Ferrite", "ItemCount": 6750}]}));
        let mut s = Settings::default();
        s.add_goal(GoalKind::Build, "/Lotus/Powersuits/Wisp/WispPrime", "Wisp Prime", 1);
        assert_eq!(ask("how much ferrite do i have", &acct, None, None, &s), "You have 6,750 × Ferrite.");
        assert!(ask("what are my goals", &acct, None, None, &s).contains("Wisp Prime"));
    }

    #[test]
    fn friend_questions() {
        let acct = Account::default();
        let f = Snapshot {
            name: "Alex".into(),
            available: Some(vec![Avail { unique: "/A/WispPrimeSystemsBlueprint".into(), name: "Wisp Prime Systems".into(), count: 1, ..Default::default() }]),
            ..Default::default()
        };
        let m = Matching { they_have: vec![("Wisp Prime Systems".into(), 1)], ..Default::default() };
        let a = ask("does my friend have wisp prime systems", &acct, Some(&f), Some(&m), &Settings::default());
        assert!(a.contains("Alex has available: Wisp Prime Systems ×1"), "{a}");
        let b = ask("can my friend help me", &acct, Some(&f), Some(&m), &Settings::default());
        assert!(b.contains("Alex has what you need"), "{b}");
    }

    #[test]
    fn no_inventory_is_handled() {
        let cat = test_catalog();
        let s = Settings::default();
        let w = WorldState::default();
        let ctx = Ctx { cat: &cat, me: None, friend: None, matching: None, my_avail: &[], settings: &s, world: &w };
        assert!(answer(&ctx, "anything").contains("cannot see your inventory"));
    }
}
