use std::sync::mpsc::{channel, Receiver, Sender};
use eframe::egui;
use osu_mixer_core::bridge::Skin;

use crate::audio::AudioPlayer;
use crate::state::AppState;
use crate::textures::TextureCache;
use crate::views::{categories, preview, sidebar};

pub enum AppEvent {
    SkinsLoaded(Result<Vec<Skin>, String>),
    MixCompleted(Result<String, String>),
    ProcessCheckResult(bool),
}

pub struct MixerApp {
    pub state: AppState,
    pub textures: TextureCache,
    pub audio: AudioPlayer,
    pub tx: Sender<AppEvent>,
    pub rx: Receiver<AppEvent>,
}

impl MixerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let (tx, rx) = channel();

        Self {
            state: AppState::default(),
            textures: TextureCache::new(),
            audio: AudioPlayer::new(),
            tx,
            rx,
        }
    }
}

impl eframe::App for MixerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
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
                ui.heading("osu! mixer (lazer)");
                if self.state.is_working {
                    ui.spinner();
                    ui.label("Обработка...");
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
                let action = preview::render_preview(
                    ui,
                    &mut self.state,
                    &mut self.textures,
                    &self.audio,
                );
                if action == preview::PreviewAction::ApplyMix {
                    // Будет подключено в worker в Task 6
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            categories::render_categories(ui, &mut self.state);
        });
    }
}
