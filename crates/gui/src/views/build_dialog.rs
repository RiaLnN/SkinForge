use eframe::egui::{self, Color32, Vec2};
use std::sync::mpsc::Sender;

use crate::state::{AppState, BuildMethod};
use crate::worker::{BuildSkinTask, WorkerTask};

pub fn render_build_dialog(
    ctx: &egui::Context,
    state: &mut AppState,
    worker_tx: &Sender<WorkerTask>,
) {
    if !state.show_build_dialog {
        return;
    }

    let mut is_open = true;
    let mut close_requested = false;

    let accent_color = Color32::from_rgb(137, 180, 250);
    let inactive_border = Color32::from_rgb(50, 50, 65);
    let text_muted = Color32::from_rgb(166, 173, 200);
    let text_secondary = Color32::from_rgb(140, 145, 170);
    let warning_color = Color32::from_rgb(243, 139, 168);

    egui::Window::new("Build and Install Skin")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .default_width(540.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.style_mut().interaction.selectable_labels = false;

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Skin: {}", state.new_skin_name))
                    .size(15.0)
                    .strong(),
            );
            ui.label(
                egui::RichText::new("Choose an installation method for this skin mix:")
                    .color(text_muted),
            );
            ui.add_space(10.0);

            // 1. OSK Option (Recommended)
            let is_osk = state.selected_build_method == BuildMethod::OskPackage;
            let osk_stroke = egui::Stroke::new(
                if is_osk { 1.5_f32 } else { 1.0_f32 },
                if is_osk { accent_color } else { inactive_border },
            );

            let mut select_osk = false;
            let osk_frame = egui::Frame::group(ui.style())
                .stroke(osk_stroke)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .radio(
                                state.selected_build_method == BuildMethod::OskPackage,
                                "",
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            select_osk = true;
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let label = egui::RichText::new("Export .osk package and open in osu!lazer")
                                    .strong()
                                    .color(Color32::WHITE);
                                if ui
                                    .add(egui::Label::new(label).selectable(false).sense(egui::Sense::click()))
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    select_osk = true;
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.colored_label(
                                        accent_color,
                                        egui::RichText::new("RECOMMENDED").size(10.0).strong(),
                                    );
                                });
                            });

                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    "Creates a standard .osk archive and launches it with osu!lazer. The game safely validates, imports, and activates the skin.",
                                )
                                .size(11.5)
                                .color(text_muted),
                            );

                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("• 100% database-safe   • Works while osu!lazer is running")
                                    .size(11.0)
                                    .color(text_secondary),
                            );
                        });
                    });
                });

            let osk_interact = osk_frame.response.interact(egui::Sense::click());
            if select_osk || osk_interact.clicked() {
                state.selected_build_method = BuildMethod::OskPackage;
            }
            if osk_interact.hovered() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            }

            ui.add_space(10.0);

            // 2. Direct Realm Option (Zero-Copy)
            let is_realm = state.selected_build_method == BuildMethod::DirectRealm;
            let realm_stroke = egui::Stroke::new(
                if is_realm { 1.5_f32 } else { 1.0_f32 },
                if is_realm { accent_color } else { inactive_border },
            );

            let mut select_realm = false;
            let realm_frame = egui::Frame::group(ui.style())
                .stroke(realm_stroke)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .radio(
                                state.selected_build_method == BuildMethod::DirectRealm,
                                "",
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            select_realm = true;
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let label = egui::RichText::new("Direct injection into client.realm")
                                    .strong()
                                    .color(Color32::WHITE);
                                if ui
                                    .add(egui::Label::new(label).selectable(false).sense(egui::Sense::click()))
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    select_realm = true;
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.colored_label(
                                        text_secondary,
                                        egui::RichText::new("ZERO-COPY").size(10.0).strong(),
                                    );
                                });
                            });

                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    "Writes skin manifest references directly into the database without duplicating file storage (zero-copy).",
                                )
                                .size(11.5)
                                .color(text_muted),
                            );

                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "• Danger if game is running: concurrent writes to client.realm can corrupt the database or crash osu!",
                                )
                                .size(11.0)
                                .color(text_secondary),
                            );

                            if state.is_game_running {
                                ui.add_space(3.0);
                                ui.label(
                                    egui::RichText::new(
                                        "⚠ osu! is currently running. Writing to Realm is blocked to prevent database corruption.",
                                    )
                                    .size(11.0)
                                    .color(warning_color),
                                );
                            }
                        });
                    });
                });

            let realm_interact = realm_frame.response.interact(egui::Sense::click());
            if select_realm || realm_interact.clicked() {
                state.selected_build_method = BuildMethod::DirectRealm;
            }
            if realm_interact.hovered() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Action Buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    close_requested = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_submit = match state.selected_build_method {
                        BuildMethod::OskPackage => !state.is_working,
                        BuildMethod::DirectRealm => !state.is_working && !state.is_game_running,
                    };

                    let submit_text = match state.selected_build_method {
                        BuildMethod::OskPackage => "Build & Open Skin",
                        BuildMethod::DirectRealm => "Write to Realm",
                    };

                    ui.add_enabled_ui(can_submit, |ui| {
                        let btn = egui::Button::new(
                            egui::RichText::new(submit_text)
                                .color(Color32::from_rgb(17, 17, 27))
                                .strong(),
                        )
                        .fill(accent_color)
                        .min_size(Vec2::new(180.0, 32.0));

                        let btn_response = ui
                            .add(btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        if btn_response.clicked() {
                            if let (Some(lazer_root), Some(files_dir), Some(client_realm_path)) = (
                                state.lazer_root.clone(),
                                state.files_dir.clone(),
                                state.client_realm_path.clone(),
                            ) {
                                if let Some(plan) = state.build_mix_plan(None) {
                                    state.is_working = true;
                                    state.status_message = Some((
                                        match state.selected_build_method {
                                            BuildMethod::OskPackage => {
                                                "Exporting .osk package and sending to osu!lazer...".to_string()
                                            }
                                            BuildMethod::DirectRealm => {
                                                "Writing skin manifest to client.realm...".to_string()
                                            }
                                        },
                                        false,
                                    ));

                                    let _ = worker_tx.send(WorkerTask::BuildSkin(Box::new(
                                        BuildSkinTask {
                                            method: state.selected_build_method,
                                            plan,
                                            combo_colours: state.combo_colours.clone(),
                                            installed_skins: state.installed_skins.clone(),
                                            files_dir,
                                            lazer_root,
                                            client_realm_path,
                                            set_active_on_build: state.set_active_on_build,
                                            extended_ini: state.extended_ini.clone(),
                                        },
                                    )));

                                    close_requested = true;
                                }
                            }
                        }
                    });
                });
            });
        });

    state.show_build_dialog = is_open && !close_requested;
}
