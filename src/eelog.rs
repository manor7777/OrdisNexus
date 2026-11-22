//! Live game awareness: follows Warframe's `EE.log` and turns interesting lines
//! into events. The patterns are editable in Settings because the game's log
//! wording can change between updates.

use crate::settings::EePattern;
use crate::util::now_ms;
use eframe::egui;
use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct EeEvent {
    pub ts_ms: i64,
    pub label: String,
    /// `mission_start`, `mission_end` or `note`.
    pub kind: String,
    pub line: String,
}

pub struct EeWatcher {
    pub rx: Receiver<EeEvent>,
    pub lines: Arc<Mutex<VecDeque<String>>>,
    pub status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
}

impl Drop for EeWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub fn classify(line: &str, patterns: &[EePattern]) -> Option<(String, String)> {
    patterns
        .iter()
        .find(|p| !p.contains.is_empty() && line.contains(&p.contains))
        .map(|p| (p.label.clone(), p.kind.clone()))
}

/// Text after `marker` in `line`, trimmed.
pub fn tail_after(line: &str, marker: &str) -> Option<String> {
    let i = line.find(marker)? + marker.len();
    let t = line[i..].trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// The integer that follows `marker`.
pub fn number_after(line: &str, marker: &str) -> Option<u64> {
    let t = tail_after(line, marker)?;
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

const KEEP_LINES: usize = 400;

pub fn start(path: PathBuf, patterns: Vec<EePattern>, ctx: &egui::Context) -> EeWatcher {
    let (tx, rx) = channel();
    let lines = Arc::new(Mutex::new(VecDeque::new()));
    let status = Arc::new(Mutex::new(format!("Waiting for {}", path.display())));
    let stop = Arc::new(AtomicBool::new(false));
    let (l2, s2, stop2, ctx) = (lines.clone(), status.clone(), stop.clone(), ctx.clone());
    std::thread::spawn(move || {
        let mut file: Option<std::fs::File> = None;
        let mut pos: u64 = 0;
        let mut pending: Vec<u8> = Vec::new();
        while !stop2.load(Ordering::SeqCst) {
            if file.is_none() {
                match std::fs::File::open(&path) {
                    Ok(mut f) => {
                        // start at the end: only new activity matters
                        pos = f.seek(SeekFrom::End(0)).unwrap_or(0);
                        *s2.lock().unwrap() = format!("Watching {}", path.display());
                        file = Some(f);
                    }
                    Err(_) => {
                        *s2.lock().unwrap() = format!("Waiting for {} (start Warframe)", path.display());
                        std::thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                }
            }
            let f = file.as_mut().unwrap();
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            if len < pos {
                // the game restarted and began a fresh log
                pos = 0;
                let _ = f.seek(SeekFrom::Start(0));
                pending.clear();
            }
            if len > pos {
                let mut buf = Vec::new();
                let _ = f.seek(SeekFrom::Start(pos));
                if f.take(len - pos).read_to_end(&mut buf).is_ok() {
                    pos += buf.len() as u64;
                    pending.extend_from_slice(&buf);
                }
                while let Some(i) = pending.iter().position(|&b| b == b'\n') {
                    let raw: Vec<u8> = pending.drain(..=i).collect();
                    let line = String::from_utf8_lossy(&raw).trim_end().to_string();
                    {
                        let mut g = l2.lock().unwrap();
                        g.push_back(line.clone());
                        while g.len() > KEEP_LINES {
                            g.pop_front();
                        }
                    }
                    if let Some((label, kind)) = classify(&line, &patterns) {
                        let _ = tx.send(EeEvent { ts_ms: now_ms(), label, kind, line });
                    }
                }
                ctx.request_repaint();
            }
            std::thread::sleep(Duration::from_millis(400));
        }
    });
    EeWatcher { rx, lines, status, stop }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::default_patterns;

    #[test]
    fn classifies_real_log_lines() {
        let p = default_patterns();
        let k = |l: &str| classify(l, &p).map(|(_, k)| k);
        assert_eq!(k("3701:86.803 Game [Info]: OnStateStarted, mission type=MT_SURVIVAL").as_deref(), Some("mission_start"));
        assert_eq!(k("445.717 Script [Info]: EndOfMatch.lua: Mission Succeeded").as_deref(), Some("mission_end"));
        assert_eq!(k("2204.861 Net [Info]: GameRulesImpl - changing state from SS_STARTED to SS_ENDING").as_deref(), Some("mission_end"));
        assert_eq!(k("81.006 Script [Info]: ThemedSquadOverlay.lua: Mission name: Cameria (Jupiter)").as_deref(), Some("mission_name"));
        assert_eq!(k("445.995 Script [Info]: EndOfMatch.lua: NotifyTagMultiple(MISSION_REWARD_CREDITS): 16900").as_deref(), Some("credits"));
        assert!(k("445.738 Sys [Info]: nothing interesting").is_none());
        assert!(classify("anything", &[EePattern { label: "x".into(), contains: String::new(), kind: "note".into() }]).is_none());
    }

    #[test]
    fn extracts_values() {
        assert_eq!(tail_after("x: Mission name: Cameria (Jupiter)", "Mission name: ").as_deref(), Some("Cameria (Jupiter)"));
        assert_eq!(number_after("NotifyTagMultiple(MISSION_REWARD_CREDITS): 16900", "): "), Some(16900));
        assert_eq!(number_after("nothing", "): "), None);
    }

    #[test]
    fn follows_a_growing_file() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("ordis-ee-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("EE.log");
        std::fs::write(&path, "old line\n").unwrap();
        let ctx = egui::Context::default();
        let w = start(path.clone(), default_patterns(), &ctx);
        std::thread::sleep(Duration::from_millis(900));
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "Net [Info]: GameRulesImpl - changing state from SS_STARTED to SS_ENDING").unwrap();
        let ev = w.rx.recv_timeout(Duration::from_secs(5)).expect("event arrives");
        assert_eq!(ev.kind, "mission_end");
        assert!(!w.lines.lock().unwrap().iter().any(|l| l == "old line"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
