use eframe::egui;
use skinforge_core::bridge::Skin;
use skinforge_core::paths::{LazerInstallation, StableInstallation};
use std::sync::mpsc::{Receiver, Sender};

use crate::audio::AudioPlayer;
use crate::state::AppState;
use crate::textures::TextureCache;
use crate::views::{categories, preview, sidebar};
use crate::worker::{sanitize_skin_filename, ExportOskTask, WorkerTask};

pub enum AppEvent {
    LazerInstallationFound(LazerInstallation),
    StableInstallationFound(StableInstallation),
    SkinsLoaded(Result<Vec<Skin>, String>),
    MixCompleted(Result<String, String>),
}

pub struct MixerApp {
    pub state: AppState,
    pub textures: TextureCache,
    pub audio: AudioPlayer,
    pub worker_tx: Sender<WorkerTask>,
    pub rx: Receiver<AppEvent>,
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

        cc.egui_ctx.style_mut(|style| {
            style.interaction.selectable_labels = false;
        });

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
        }
    }

    fn dispatch_export_osk(&mut self) {
        let Some(plan) = self.state.build_mix_plan(None) else {
            return;
        };

        let safe_name = sanitize_skin_filename(&plan.new_skin_name);
        let default_filename = format!("{safe_name}.osk");

        let mut dialog = rfd::FileDialog::new()
            .set_title("Export Mixed .osk Skin")
            .add_filter("osu! Skin Package (.osk)", &["osk"])
            .set_file_name(&default_filename);

        if let Some(lazer_root) = &self.state.lazer_root {
            let exports_dir = lazer_root.join("exports");
            let _ = std::fs::create_dir_all(&exports_dir);
            if exports_dir.is_dir() {
                dialog = dialog.set_directory(&exports_dir);
            }
        } else if let Some(stable_root) = &self.state.stable_root {
            if stable_root.is_dir() {
                dialog = dialog.set_directory(stable_root);
            }
        }

        if let Some(mut output_path) = dialog.save_file() {
            if output_path.extension().is_none() {
                output_path.set_extension("osk");
            }
            self.state.is_working = true;
            self.state.status_message = None;

            let task = ExportOskTask {
                plan,
                combo_colours: self.state.combo_colours.clone(),
                installed_skins: self.state.installed_skins.clone(),
                files_dir: self.state.files_dir.clone(),
                output_path,
                open_after_export: self.state.open_after_export,
                extended_ini: self.state.extended_ini.clone(),
            };
            let _ = self.worker_tx.send(WorkerTask::ExportOsk(Box::new(task)));
        }
    }
}

impl eframe::App for MixerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::LazerInstallationFound(inst) => {
                    self.state.lazer_root = Some(inst.root);
                    self.state.files_dir = Some(inst.files_dir);
                    self.state.client_realm_path = Some(inst.client_realm);
                }
                AppEvent::StableInstallationFound(inst) => {
                    self.state.stable_root = Some(inst.root);
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
                }
                AppEvent::MixCompleted(Err(err)) => {
                    self.state.status_message = Some((err, true));
                    self.state.is_working = false;
                }
            }
        }

        egui::TopBottomPanel::top("top_status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("SkinForge");

                if ui
                    .button(format!(
                        "{} Refresh Skins",
                        egui_phosphor::regular::ARROWS_CLOCKWISE
                    ))
                    .on_hover_text("Reload skins from osu!lazer and osu!stable")
                    .clicked()
                {
                    self.state.is_working = true;
                    let _ = self.worker_tx.send(WorkerTask::LoadSkins {
                        custom_lazer_root: self.state.custom_lazer_root.clone(),
                        custom_stable_root: self.state.custom_stable_root.clone(),
                    });
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
            .default_width(285.0)
            .width_range(250.0..=360.0)
            .show(ctx, |ui| {
                sidebar::render_sidebar(ui, &mut self.state, &self.worker_tx);
            });

        egui::SidePanel::right("preview_panel")
            .default_width(370.0)
            .width_range(310.0..=460.0)
            .show(ctx, |ui| {
                let action =
                    preview::render_preview(ui, &mut self.state, &mut self.textures, &self.audio);
                if action == preview::PreviewAction::ExportOsk {
                    self.dispatch_export_osk();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            categories::render_categories(ui, &mut self.state, &mut self.textures, &self.audio);
        });
    }
}
