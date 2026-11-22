#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod account;
mod app;
mod assistant;
mod catalog;
mod close;
mod compare;
mod decode;
mod eelog;
mod gep;
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

/// Where problems are written so a silent start-up failure can be diagnosed.
pub fn log_path() -> std::path::PathBuf {
    util::data_dir().join("startup.log")
}

pub fn log(msg: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(util::data_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = writeln!(f, "[{}] {}", util::now_ms(), msg);
    }
}

fn fail_dialog(text: &str) {
    log(text);
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Ordis Nexus could not start")
        .set_description(format!("{text}\n\nDetails were saved to:\n{}\n\nPlease send that file to the developer.", log_path().display()))
        .show();
}

fn main() -> eframe::Result {
    let _ = std::fs::remove_file(log_path());
    log(&format!("Ordis Nexus {} starting", update::current_version()));
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        fail_dialog(&format!("Unexpected error: {info}\n{bt}"));
    }));
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1000.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title("Ordis Nexus")
        .with_drag_and_drop(true);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    log("opening window");
    let result = eframe::run_native(
        "Ordis Nexus",
        eframe::NativeOptions { viewport, ..Default::default() },
        Box::new(|cc| {
            log("window created, loading app");
            apply_fluent_theme(&cc.egui_ctx);
            let app = App::new(&cc.egui_ctx);
            log("app ready");
            Ok(Box::new(app))
        }),
    );
    if let Err(e) = &result {
        fail_dialog(&format!("The window could not be created: {e}\n\nThis is usually a graphics-driver problem (OpenGL)."));
    }
    log("exited");
    result
}
