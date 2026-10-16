use std::sync::mpsc::{channel, Receiver, Sender};
use eframe::egui;
use osu_mixer_core::bridge::Skin;

use crate::state::AppState;

pub enum AppEvent {
    SkinsLoaded(Result<Vec<Skin>, String>),
    MixCompleted(Result<String, String>),
    ProcessCheckResult(bool),
}

pub struct MixerApp {
    pub state: AppState,
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("osu! mixer (lazer)");
        });
    }
}
