//! Background icon loader: downloads item images on demand, caches them on
//! disk, and exposes them as egui textures.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CDN: &str = "https://content.warframe.com/PublicExport/Manifest";
const SIZE: u32 = 64;

enum Slot {
    Loading,
    Ready(TextureHandle),
    Failed,
}

type Decoded = (String, Option<([usize; 2], Vec<u8>)>);

pub struct Icons {
    slots: HashMap<String, Slot>,
    req: Sender<String>,
    done: Receiver<Decoded>,
}

pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("OrdisNexus").join("icons")
}

/// Stable, short cache file name for a texture path (FNV-1a).
fn safe(name: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in name.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}.png")
}

fn encode(name: &str) -> String {
    name.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'/' | b'!' | b'+' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn fetch(agent: &ureq::Agent, name: &str, dir: &PathBuf) -> Option<Vec<u8>> {
    let path = dir.join(safe(name));
    if let Ok(b) = std::fs::read(&path) {
        return Some(b);
    }
    let mut resp = agent.get(&format!("{CDN}{}", encode(name))).call().ok()?;
    let bytes = resp.body_mut().read_to_vec().ok()?;
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&path, &bytes);
    Some(bytes)
}

fn decode(bytes: &[u8]) -> Option<([usize; 2], Vec<u8>)> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = img.resize(SIZE, SIZE, image::imageops::FilterType::Triangle).to_rgba8();
    Some(([img.width() as usize, img.height() as usize], img.into_raw()))
}

impl Icons {
    pub fn new(ctx: &egui::Context) -> Self {
        let (req, req_rx) = channel::<String>();
        let (done_tx, done) = channel::<Decoded>();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let dir = cache_dir();
        for _ in 0..4 {
            let (rx, tx, ctx, dir) = (req_rx.clone(), done_tx.clone(), ctx.clone(), dir.clone());
            std::thread::spawn(move || {
                let agent: ureq::Agent =
                    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).build().into();
                loop {
                    let Ok(name) = rx.lock().unwrap().recv() else { return };
                    let img = fetch(&agent, &name, &dir).and_then(|b| decode(&b));
                    let _ = tx.send((name, img));
                    ctx.request_repaint();
                }
            });
        }
        Self { slots: HashMap::new(), req, done }
    }

    /// Texture for `image` if it has loaded; otherwise queues a download.
    pub fn get(&mut self, ctx: &egui::Context, image: &str) -> Option<&TextureHandle> {
        while let Ok((name, img)) = self.done.try_recv() {
            let slot = match img {
                Some((size, rgba)) => Slot::Ready(ctx.load_texture(
                    format!("icon/{name}"),
                    ColorImage::from_rgba_unmultiplied(size, &rgba),
                    TextureOptions::LINEAR,
                )),
                None => Slot::Failed,
            };
            self.slots.insert(name, slot);
        }
        if !self.slots.contains_key(image) {
            self.slots.insert(image.to_string(), Slot::Loading);
            let _ = self.req.send(image.to_string());
        }
        match self.slots.get(image) {
            Some(Slot::Ready(t)) => Some(t),
            _ => None,
        }
    }
}
