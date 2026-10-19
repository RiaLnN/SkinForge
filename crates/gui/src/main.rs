#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

pub mod app;
pub mod audio;
pub mod state;
pub mod textures;
pub mod views;
pub mod worker;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1240.0, 760.0])
            .with_min_inner_size([1024.0, 620.0])
            .with_title("SkinForge — osu!lazer & osu!stable Skin Mixer"),
        ..Default::default()
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let (worker_tx, worker_rx) = std::sync::mpsc::channel();

    // Start background worker thread
    let worker_event_tx = event_tx.clone();
    std::thread::spawn(move || {
        worker::run_worker(worker_rx, worker_event_tx);
    });

    // Send initial task to load skins from osu!lazer and osu!stable
    let _ = worker_tx.send(worker::WorkerTask::LoadSkins {
        custom_lazer_root: None,
        custom_stable_root: None,
    });

    let app_worker_tx = worker_tx.clone();
    eframe::run_native(
        "SkinForge",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::MixerApp::new(cc, app_worker_tx, event_rx)))),
    )
}
