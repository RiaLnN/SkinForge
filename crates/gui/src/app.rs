use eframe::egui::{self, Color32, Margin, Rounding, Stroke};
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

/// Applies an osu!-inspired warm dark-plum & pink theme across the application.
fn apply_osu_pink_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.interaction.selectable_labels = false;
    style.spacing.button_padding = egui::vec2(8.0, 5.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);

    let mut visuals = egui::Visuals::dark();

    // Base surfaces with subtle osu! warm dark-magenta/plum undertone
    visuals.panel_fill = Color32::from_rgb(22, 16, 27);
    visuals.window_fill = Color32::from_rgb(27, 19, 33);
    visuals.extreme_bg_color = Color32::from_rgb(15, 10, 19);
    visuals.faint_bg_color = Color32::from_rgb(33, 23, 41);
    visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_rgb(76, 48, 92));
    visuals.window_rounding = Rounding::same(8.0);

    // Non-interactive frames, separators & labels
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(29, 20, 36);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(29, 20, 36);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(64, 42, 78));
    visuals.widgets.noninteractive.fg_stroke =
        Stroke::new(1.0_f32, Color32::from_rgb(240, 226, 246));
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    // Inactive buttons, ComboBoxes & inputs
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(42, 28, 52);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 25, 47);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(82, 52, 98));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(238, 222, 245));
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    // Hovered widgets: warm rose-plum fill + signature osu! pink border (#ff66ab)
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(72, 38, 76);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(62, 33, 66);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 102, 171));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    // Active / pressed widgets
    visuals.widgets.active.bg_fill = Color32::from_rgb(228, 64, 142);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(205, 56, 128);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 165, 210));
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    // Open ComboBox / menu header
    visuals.widgets.open.bg_fill = Color32::from_rgb(58, 32, 66);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(52, 28, 60);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 102, 171));
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.open.rounding = Rounding::same(6.0);

    // Selected items (tabs, active skin, checkboxes, sliders)
    visuals.selection.bg_fill = Color32::from_rgb(185, 48, 115);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 225, 240));

    style.visuals = visuals;
    ctx.set_style(style);
}

impl MixerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        worker_tx: Sender<WorkerTask>,
        rx: Receiver<AppEvent>,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        apply_osu_pink_theme(&cc.egui_ctx);

        Self {
            state: AppState::default(),
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
                stable_root: self.state.stable_root.clone(),
                output_path,
                launch_target: self.state.launch_target,
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

        let top_frame = egui::Frame::side_top_panel(&ctx.style())
            .inner_margin(Margin::symmetric(16.0, 9.0))
            .fill(Color32::from_rgb(25, 17, 31));

        egui::TopBottomPanel::top("top_status_panel")
            .frame(top_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("SkinForge")
                            .strong()
                            .color(Color32::from_rgb(255, 115, 180)),
                    );

                    ui.add_space(4.0);

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

                    if self.state.is_working {
                        ui.spinner();
                        ui.label("Processing...");
                    }
                    if let Some((msg, is_err)) = &self.state.status_message {
                        let color = if *is_err {
                            Color32::from_rgb(255, 110, 130)
                        } else {
                            Color32::from_rgb(100, 240, 160)
                        };
                        ui.colored_label(color, msg);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let playing = self.audio.is_playing();
                        ui.add_enabled_ui(playing, |ui| {
                            let stop_text = egui::RichText::new(format!(
                                "{} Stop Audio",
                                egui_phosphor::regular::STOP
                            ))
                            .color(if playing {
                                Color32::from_rgb(255, 102, 171)
                            } else {
                                Color32::GRAY
                            });
                            if ui
                                .button(stop_text)
                                .on_hover_text("Stop currently playing audio immediately")
                                .clicked()
                            {
                                self.audio.stop();
                            }
                        });

                        let mut vol_pct = (self.audio.volume() * 100.0).round();
                        let slider = egui::Slider::new(&mut vol_pct, 0.0..=100.0)
                            .suffix("%")
                            .show_value(true);
                        if ui
                            .add_sized([110.0, 18.0], slider)
                            .on_hover_text("Master audio preview volume")
                            .changed()
                        {
                            self.audio.set_volume(vol_pct / 100.0);
                        }
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::SPEAKER_HIGH)
                                .color(Color32::from_rgb(255, 130, 188)),
                        );
                    });
                });
            });

        let side_frame = egui::Frame::side_top_panel(&ctx.style())
            .inner_margin(Margin::symmetric(14.0, 12.0))
            .fill(Color32::from_rgb(22, 16, 27));

        egui::SidePanel::left("sidebar_panel")
            .frame(side_frame)
            .default_width(290.0)
            .width_range(255.0..=350.0)
            .show(ctx, |ui| {
                sidebar::render_sidebar(ui, &mut self.state, &self.worker_tx);
            });

        egui::SidePanel::right("preview_panel")
            .frame(side_frame)
            .default_width(375.0)
            .width_range(320.0..=450.0)
            .show(ctx, |ui| {
                let action =
                    preview::render_preview(ui, &mut self.state, &mut self.textures, &self.audio);
                if action == preview::PreviewAction::ExportOsk {
                    self.dispatch_export_osk();
                }
            });

        let central_frame = egui::Frame::central_panel(&ctx.style())
            .inner_margin(Margin::symmetric(16.0, 12.0))
            .fill(Color32::from_rgb(20, 14, 25));

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                categories::render_categories(
                    ui,
                    &mut self.state,
                    &mut self.textures,
                    &self.audio,
                );
            });
    }
}
