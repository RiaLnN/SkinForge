use eframe::egui;
use skinforge_core::bridge::Skin;
use skinforge_core::paths::LazerInstallation;
use std::sync::mpsc::{Receiver, Sender};

use crate::audio::AudioPlayer;
use crate::state::AppState;
use crate::textures::TextureCache;
use crate::views::{categories, preview, sidebar};
use crate::worker::WorkerTask;

pub enum AppEvent {
    InstallationFound(LazerInstallation),
    SkinsLoaded(Result<Vec<Skin>, String>),
    MixCompleted(Result<String, String>),
    ProcessCheckResult(bool),
}

pub struct MixerApp {
    pub state: AppState,
    pub textures: TextureCache,
    pub audio: AudioPlayer,
    pub worker_tx: Sender<WorkerTask>,
    pub rx: Receiver<AppEvent>,
    pub last_process_check: f64,
}

impl MixerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        worker_tx: Sender<WorkerTask>,
        rx: Receiver<AppEvent>,
        demo_mode: bool,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let mut state = AppState::default();
        if demo_mode {
            state.enable_demo_mode();
        }

        Self {
            state,
            textures: TextureCache::new(),
            audio: AudioPlayer::new(),
            worker_tx,
            rx,
            last_process_check: 0.0,
        }
    }
}

impl eframe::App for MixerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Periodic game process check every 2 seconds
        let time = ctx.input(|i| i.time);
        if time - self.last_process_check > 2.0 {
            self.last_process_check = time;
            let _ = self.worker_tx.send(WorkerTask::CheckProcess);
        }

        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::InstallationFound(inst) => {
                    self.state.lazer_root = Some(inst.root);
                    self.state.files_dir = Some(inst.files_dir);
                    self.state.client_realm_path = Some(inst.client_realm);
                }
                AppEvent::SkinsLoaded(Ok(skins)) => {
                    self.state.set_installed_skins(skins);
                    self.state.is_working = false;
                }
                AppEvent::SkinsLoaded(Err(err)) => {
                    self.state.status_message = Some((err, true));
                    self.state.is_working = false;
                }
                AppEvent::MixCompleted(Ok(msg)) => {
                    self.state.status_message = Some((msg, false));
                    self.state.is_working = false;
                    // Automatically reload skins so the newly created skin appears in the list
                    let _ = self.worker_tx.send(WorkerTask::LoadSkins);
                }
                AppEvent::MixCompleted(Err(err)) => {
                    self.state.status_message = Some((err, true));
                    self.state.is_working = false;
                }
                AppEvent::ProcessCheckResult(running) => {
                    self.state.is_game_running = running;
                }
            }
        }

        egui::TopBottomPanel::top("top_status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("SkinForge");

                if ui
                    .button(format!(
                        "{} Refresh",
                        egui_phosphor::regular::ARROWS_CLOCKWISE
                    ))
                    .clicked()
                {
                    self.state.is_working = true;
                    let _ = self.worker_tx.send(WorkerTask::LoadSkins);
                }

                ui.separator();

                let demo_label = if self.state.demo_mode {
                    format!("{} Clean Names: ON", egui_phosphor::regular::SPARKLE)
                } else {
                    format!("{} Clean Names: OFF", egui_phosphor::regular::SPARKLE)
                };
                if ui
                    .selectable_label(self.state.demo_mode, demo_label)
                    .on_hover_text(
                        "Mask skin names with clean canonical names for screenshots/demos",
                    )
                    .clicked()
                {
                    if self.state.demo_mode {
                        self.state.disable_demo_mode();
                    } else {
                        self.state.enable_demo_mode();
                    }
                }

                if self.state.is_working {
                    ui.spinner();
                    ui.label("Processing...");
                }
                if let Some((msg, is_err)) = &self.state.status_message {
                    let color = if *is_err {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::from_rgb(56, 239, 125)
                    };
                    ui.colored_label(color, msg);
                }
            });
        });

        egui::SidePanel::left("sidebar_panel")
            .default_width(280.0)
            .width_range(240.0..=360.0)
            .show(ctx, |ui| {
                sidebar::render_sidebar(ui, &mut self.state);
            });

        egui::SidePanel::right("preview_panel")
            .default_width(380.0)
            .width_range(300.0..=480.0)
            .show(ctx, |ui| {
                let action =
                    preview::render_preview(ui, &mut self.state, &mut self.textures, &self.audio);
                if action == preview::PreviewAction::ApplyMix {
                    if let (Some(files_dir), Some(client_realm_path)) = (
                        self.state.files_dir.clone(),
                        self.state.client_realm_path.clone(),
                    ) {
                        if let Some(plan) = self.state.build_mix_plan(None) {
                            self.state.is_working = true;
                            self.state.status_message =
                                Some(("Building and saving skin to Realm...".to_string(), false));
                            let _ = self.worker_tx.send(WorkerTask::ApplyMix(Box::new(
                                crate::worker::ApplyMixTask {
                                    plan,
                                    combo_colours: self.state.combo_colours.clone(),
                                    installed_skins: self.state.installed_skins.clone(),
                                    files_dir,
                                    client_realm_path,
                                    set_active_on_build: self.state.set_active_on_build,
                                    extended_ini: self.state.extended_ini.clone(),
                                },
                            )));
                        }
                    } else {
                        self.state.status_message =
                            Some(("osu!lazer data directory not found".to_string(), true));
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            categories::render_categories(ui, &mut self.state, &self.audio);
        });
    }
}
