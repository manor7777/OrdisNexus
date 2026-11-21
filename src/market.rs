//! warframe.market price lookups (background, rate limited, cached on disk).

use crate::util::{data_dir, now_ms};
use eframe::egui;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TTL_MS: i64 = 6 * 3600 * 1000;

#[derive(Clone)]
enum Entry {
    Loading,
    Done(f32, i64),
    Failed(i64),
}

pub struct Market {
    state: Arc<Mutex<HashMap<String, Entry>>>,
    tx: Sender<String>,
}

fn cache_path() -> std::path::PathBuf {
    data_dir().join("market_cache.json")
}

/// Median of the cheapest sell orders in a `/v2/orders/item/{slug}/top` response.
pub fn parse_top(v: &Value) -> Option<f32> {
    let mut prices: Vec<f32> = v
        .get("data")?
        .get("sell")?
        .as_array()?
        .iter()
        .filter_map(|o| o.get("platinum").and_then(Value::as_f64))
        .map(|p| p as f32)
        .collect();
    if prices.is_empty() {
        return None;
    }
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    prices.truncate(5);
    Some(prices[prices.len() / 2])
}

impl Market {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut map: HashMap<String, Entry> = HashMap::new();
        if let Some(v) = std::fs::read_to_string(cache_path()).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
            for (k, e) in v.as_object().into_iter().flatten() {
                if let (Some(p), Some(t)) = (e.get(0).and_then(Value::as_f64), e.get(1).and_then(Value::as_i64)) {
                    if now_ms() - t < TTL_MS {
                        map.insert(k.clone(), Entry::Done(p as f32, t));
                    }
                }
            }
        }
        let state = Arc::new(Mutex::new(map));
        let (tx, rx) = channel::<String>();
        let (st, ctx) = (state.clone(), ctx.clone());
        std::thread::spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().into();
            while let Ok(slug) = rx.recv() {
                std::thread::sleep(Duration::from_millis(400)); // the API allows ~3 requests/second
                let url = format!("https://api.warframe.market/v2/orders/item/{slug}/top");
                let result = agent
                    .get(&url)
                    .header("Accept", "application/json")
                    .header("Language", "en")
                    .header("Platform", "pc")
                    .call()
                    .ok()
                    .and_then(|mut r| r.body_mut().read_to_vec().ok())
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                    .and_then(|v| parse_top(&v));
                let now = now_ms();
                let mut g = st.lock().unwrap();
                g.insert(slug, match result {
                    Some(p) => Entry::Done(p, now),
                    None => Entry::Failed(now),
                });
                // persist what we know
                let out: serde_json::Map<String, Value> = g
                    .iter()
                    .filter_map(|(k, e)| match e {
                        Entry::Done(p, t) => Some((k.clone(), serde_json::json!([p, t]))),
                        _ => None,
                    })
                    .collect();
                drop(g);
                let _ = std::fs::create_dir_all(data_dir());
                let _ = std::fs::write(cache_path(), Value::Object(out).to_string());
                ctx.request_repaint();
            }
        });
        Self { state, tx }
    }

    /// Cached platinum price for a slug; queues a lookup the first time it is asked.
    pub fn price(&self, slug: &str) -> Option<f32> {
        let mut g = self.state.lock().unwrap();
        match g.get(slug) {
            Some(Entry::Done(p, _)) => Some(*p),
            Some(Entry::Failed(t)) if now_ms() - *t < 600_000 => None,
            Some(Entry::Loading) => None,
            _ => {
                g.insert(slug.to_string(), Entry::Loading);
                let _ = self.tx.send(slug.to_string());
                None
            }
        }
    }

    pub fn pending(&self) -> usize {
        self.state.lock().unwrap().values().filter(|e| matches!(e, Entry::Loading)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn median_of_cheapest() {
        let v = json!({"data": {"sell": [{"platinum": 4}, {"platinum": 4}, {"platinum": 5}, {"platinum": 9}, {"platinum": 30}]}});
        assert_eq!(parse_top(&v), Some(5.0));
        assert_eq!(parse_top(&json!({"data": {"sell": []}})), None);
        assert_eq!(parse_top(&json!({})), None);
    }
}
