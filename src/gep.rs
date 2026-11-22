//! Receiver for the Ordis Nexus Bridge, a small Overwolf app that listens to
//! Warframe's Game Events Provider (GEP) and forwards what it sees.
//!
//! Transport: a tiny HTTP server bound to 127.0.0.1 only, so the Overwolf app
//! needs no file permissions. Only requests whose `Origin` is an
//! `overwolf-extension://` page (or that have no Origin at all, such as curl)
//! are accepted, so an ordinary web page cannot post to it.
//!
//! The inventory carried by GEP (`match_info.inventory`) is the same JSON
//! document AlecaFrame stores, so it is written to a plain file that the normal
//! inventory loader already understands.

use crate::util::{data_dir, now_ms};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const PORT: u16 = 17642;
const MAX_BODY: usize = 64 * 1024 * 1024;

#[derive(Default, Clone)]
pub struct GepStatus {
    pub listening: bool,
    pub error: Option<String>,
    /// Last request of any kind from the bridge.
    pub last_contact_ms: i64,
    pub last_update_ms: i64,
    pub last_inventory_ms: i64,
    pub updates: u64,
    pub inventory_updates: u64,
    pub username: Option<String>,
    pub bridge_version: Option<String>,
    pub game_running: bool,
    pub recent: Vec<String>,
}

pub type Shared = Arc<Mutex<GepStatus>>;

pub fn dir() -> PathBuf {
    data_dir().join("overwolf")
}

pub fn inventory_path() -> PathBuf {
    dir().join("inventory.json")
}

/// Find the inventory inside whatever shape Overwolf delivered: the document
/// can be a string or an object, nested under `info`, `match_info`, etc.
pub fn find_inventory(v: &Value) -> Option<String> {
    match v {
        Value::Object(m) => {
            if let Some(inv) = m.get("inventory") {
                match inv {
                    Value::String(s) if s.trim_start().starts_with('{') => return Some(s.clone()),
                    Value::Object(_) => return serde_json::to_string(inv).ok(),
                    _ => {}
                }
            }
            m.values().find_map(find_inventory)
        }
        Value::Array(a) => a.iter().find_map(find_inventory),
        Value::String(s) if s.len() > 2 && s.trim_start().starts_with('{') && s.contains("\"inventory\"") => {
            serde_json::from_str::<Value>(s).ok().and_then(|x| find_inventory(&x))
        }
        _ => None,
    }
}

pub fn find_string(v: &Value, key: &str) -> Option<String> {
    match v {
        Value::Object(m) => {
            if let Some(Value::String(s)) = m.get(key) {
                return Some(s.clone());
            }
            m.values().find_map(|x| find_string(x, key))
        }
        Value::Array(a) => a.iter().find_map(|x| find_string(x, key)),
        _ => None,
    }
}

fn note(st: &mut GepStatus, text: String) {
    st.recent.push(text);
    if st.recent.len() > 60 {
        st.recent.remove(0);
    }
}

/// Apply one message from the bridge. Returns true if a new inventory was stored.
pub fn handle_message(body: &str, status: &Shared, write_to: &std::path::Path) -> Result<bool, String> {
    let msg: Value = serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;
    let kind = msg.get("kind").and_then(Value::as_str).unwrap_or("info").to_string();
    let data = msg.get("data").cloned().unwrap_or(Value::Null);
    let now = now_ms();
    let mut stored = false;

    match kind.as_str() {
        "status" => {
            let mut st = status.lock().unwrap();
            st.last_contact_ms = now;
            st.game_running = data.get("gameRunning").and_then(Value::as_bool).unwrap_or(st.game_running);
            if let Some(v) = data.get("version").and_then(Value::as_str) {
                st.bridge_version = Some(v.to_string());
            }
        }
        "event" => {
            let mut st = status.lock().unwrap();
            st.last_contact_ms = now;
            st.last_update_ms = now;
            st.updates += 1;
            let text = serde_json::to_string(&data).unwrap_or_default();
            note(&mut st, format!("event {}", text.chars().take(160).collect::<String>()));
        }
        _ => {
            let inv = find_inventory(&data);
            let user = find_string(&data, "username");
            if let Some(inv) = &inv {
                // validate before replacing a good file
                let v: Value = serde_json::from_str(inv).map_err(|e| format!("inventory is not json: {e}"))?;
                if !v.is_object() {
                    return Err("inventory is not an object".into());
                }
                if let Some(parent) = write_to.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let tmp = write_to.with_extension("tmp");
                std::fs::write(&tmp, inv).map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, write_to).map_err(|e| e.to_string())?;
                stored = true;
            }
            let mut st = status.lock().unwrap();
            st.last_contact_ms = now;
            st.last_update_ms = now;
            st.updates += 1;
            if user.is_some() {
                st.username = user;
            }
            if stored {
                st.last_inventory_ms = now;
                st.inventory_updates += 1;
                note(&mut st, "info update WITH inventory".into());
            } else {
                note(&mut st, "info update (no inventory in this one)".into());
            }
        }
    }
    Ok(stored)
}

fn respond(s: &mut TcpStream, code: &str, origin: Option<&str>, body: &str) {
    let allow = origin.map(|o| format!("Access-Control-Allow-Origin: {o}\r\nVary: Origin\r\n")).unwrap_or_default();
    let _ = write!(
        s,
        "HTTP/1.1 {code}\r\n{allow}Access-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn origin_ok(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(o) => o.starts_with("overwolf-extension://"),
    }
}

fn serve(mut s: TcpStream, status: Shared, path: PathBuf) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(15)));
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 16384];
    let header_end;
    loop {
        match s.read(&mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = i + 4;
            break;
        }
        if buf.len() > 64 * 1024 {
            return;
        }
    }
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request = lines.next().unwrap_or("").to_string();
    let mut origin: Option<String> = None;
    let mut len = 0usize;
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "origin" => origin = Some(v.trim().to_string()),
                "content-length" => len = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    if !origin_ok(origin.as_deref()) {
        respond(&mut s, "403 Forbidden", None, "forbidden");
        return;
    }
    let o = origin.as_deref();
    let mut parts = request.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    match (method, target) {
        ("OPTIONS", _) => respond(&mut s, "204 No Content", o, ""),
        ("GET", "/ping") => {
            status.lock().unwrap().last_contact_ms = now_ms();
            respond(&mut s, "200 OK", o, "ordis-nexus");
        }
        ("POST", "/gep") => {
            if len > MAX_BODY {
                respond(&mut s, "413 Payload Too Large", o, "too large");
                return;
            }
            let mut body: Vec<u8> = buf[header_end..].to_vec();
            while body.len() < len {
                match s.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => body.extend_from_slice(&tmp[..n]),
                }
            }
            let text = String::from_utf8_lossy(&body).to_string();
            match handle_message(&text, &status, &path) {
                Ok(stored) => respond(&mut s, "200 OK", o, if stored { "stored" } else { "ok" }),
                Err(e) => {
                    status.lock().unwrap().error = Some(e.clone());
                    respond(&mut s, "400 Bad Request", o, &e)
                }
            }
        }
        _ => respond(&mut s, "404 Not Found", o, "not found"),
    }
}

/// Start the listener. Never panics: if the port is taken the status says so.
pub fn start(ctx: eframe::egui::Context) -> Shared {
    let status: Shared = Arc::new(Mutex::new(GepStatus::default()));
    let st = status.clone();
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
            Ok(l) => l,
            Err(e) => {
                let mut g = st.lock().unwrap();
                g.error = Some(format!("Cannot listen on 127.0.0.1:{PORT}: {e}. Is another copy of Ordis Nexus running?"));
                return;
            }
        };
        st.lock().unwrap().listening = true;
        for conn in listener.incoming().flatten() {
            let (st2, ctx2, path) = (st.clone(), ctx.clone(), inventory_path());
            std::thread::spawn(move || {
                serve(conn, st2, path);
                ctx2.request_repaint();
            });
        }
    });
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ordis-gep-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.join("inventory.json")
    }

    #[test]
    fn finds_inventory_in_the_getinfo_shape() {
        let inner = r#"{"RegularCredits":5,"MiscItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite","ItemCount":9}]}"#;
        let msg = serde_json::json!({"info": {"match_info": {"inventory": inner}}});
        assert_eq!(find_inventory(&msg).as_deref(), Some(inner));
    }

    #[test]
    fn finds_inventory_in_the_update_shape() {
        let inner = r#"{"RegularCredits":5}"#;
        let msg = serde_json::json!({"feature":"match_info","info":{"match_info":{"inventory":inner}}});
        assert!(find_inventory(&msg).is_some());
        assert!(find_inventory(&serde_json::json!({"info":{"game_info":{"username":"x"}}})).is_none());
    }

    #[test]
    fn stores_inventory_and_username() {
        let status: Shared = Arc::new(Mutex::new(GepStatus::default()));
        let p = tmp("store");
        let inner = r#"{"RegularCredits":5,"MiscItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite","ItemCount":9}]}"#;
        let body = serde_json::json!({"kind":"info","ts":1,"data":{"info":{"match_info":{"inventory":inner},"game_info":{"username":"Tenno"}}}}).to_string();
        assert!(handle_message(&body, &status, &p).unwrap());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), inner);
        let st = status.lock().unwrap();
        assert_eq!(st.inventory_updates, 1);
        assert_eq!(st.username.as_deref(), Some("Tenno"));
    }

    #[test]
    fn updates_without_inventory_leave_the_file_alone() {
        let status: Shared = Arc::new(Mutex::new(GepStatus::default()));
        let p = tmp("noinv");
        let body = serde_json::json!({"kind":"info","data":{"info":{"game_info":{"username":"Tenno"}}}}).to_string();
        assert!(!handle_message(&body, &status, &p).unwrap());
        assert!(!p.exists());
        assert_eq!(status.lock().unwrap().updates, 1);
    }

    #[test]
    fn rejects_bad_input_and_foreign_origins() {
        let status: Shared = Arc::new(Mutex::new(GepStatus::default()));
        assert!(handle_message("not json", &status, &tmp("bad")).is_err());
        assert!(origin_ok(None));
        assert!(origin_ok(Some("overwolf-extension://abcd")));
        assert!(!origin_ok(Some("https://evil.example")));
    }

    #[test]
    fn real_http_roundtrip() {
        let status: Shared = Arc::new(Mutex::new(GepStatus::default()));
        let p = tmp("http");
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let (st, pp) = (status.clone(), p.clone());
        let h = std::thread::spawn(move || {
            let (c, _) = l.accept().unwrap();
            serve(c, st, pp);
        });
        let inner = r#"{"RegularCredits":7}"#;
        let body = serde_json::json!({"kind":"info","data":{"match_info":{"inventory":inner}}}).to_string();
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(c, "POST /gep HTTP/1.1\r\nHost: x\r\nOrigin: overwolf-extension://abc\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).unwrap();
        h.join().unwrap();
        assert!(out.starts_with("HTTP/1.1 200"), "{out}");
        assert!(out.contains("Access-Control-Allow-Origin: overwolf-extension://abc"));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), inner);
    }
}
