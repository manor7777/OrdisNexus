#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod account;
mod app;
mod assistant;
mod catalog;
mod compare;
mod decode;
mod eelog;
mod goals;
mod icons;
mod insight;
mod market;
mod recipes;
mod recommend;
mod settings;
mod share;
mod theme;
mod trade;
mod update;
mod util;
mod views;
mod worldstate;

use app::App;
use eframe::egui;
use std::sync::Arc;
use theme::apply_fluent_theme;

fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?.to_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1000.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title("Ordis Nexus")
        .with_drag_and_drop(true);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    eframe::run_native(
        "Ordis Nexus",
        eframe::NativeOptions { viewport, ..Default::default() },
        Box::new(|cc| {
            apply_fluent_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}
