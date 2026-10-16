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
            .with_inner_size([1180.0, 720.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("osu! mixer (lazer)"),
        ..Default::default()
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let (worker_tx, worker_rx) = std::sync::mpsc::channel();

    // Запуск фонового рабочего потока
    let worker_event_tx = event_tx.clone();
    std::thread::spawn(move || {
        worker::run_worker(worker_rx, worker_event_tx);
    });

    // Отправка первичных задач на проверку процессов и чтение скинов
    let _ = worker_tx.send(worker::WorkerTask::CheckProcess);
    let _ = worker_tx.send(worker::WorkerTask::LoadSkins);

    let app_worker_tx = worker_tx.clone();
    eframe::run_native(
        "osu! mixer (lazer)",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(app::MixerApp::new(cc, app_worker_tx, event_rx)))
        }),
    )
}
