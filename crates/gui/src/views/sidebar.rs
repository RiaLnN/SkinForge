use std::sync::mpsc::Sender;

use eframe::egui::{self, Color32};
use skinforge_core::bridge::SkinSource;

use crate::state::{truncate_text, AppState};
use crate::worker::WorkerTask;

pub fn render_sidebar(ui: &mut egui::Ui, state: &mut AppState, worker_tx: &Sender<WorkerTask>) {
    let panel_width = ui.available_width();
    ui.set_max_width(panel_width);
    let field_width = (panel_width - 14.0).max(140.0);

    ui.vertical(|ui| {
        ui.set_max_width(panel_width);

        ui.heading(
            egui::RichText::new(format!(
                "{} 1. Base Skin & Filter",
                egui_phosphor::regular::FOLDER_OPEN
            ))
            .color(Color32::from_rgb(255, 130, 188)),
        );
        ui.add_space(6.0);

        // 1. Base skin selector (strictly truncated + ComboBox::truncate so long names never stretch the panel)
        let (current_base_display, current_base_full) = state
            .base_skin()
            .map(|s| {
                let badge = s.badge_label();
                let short = truncate_text(&s.name, 16);
                (
                    format!("[{badge}] {short}"),
                    format!("{} ({}) — by {}", s.name, badge, s.creator),
                )
            })
            .unwrap_or_else(|| ("Select skin...".to_string(), "No base skin selected".to_string()));

        ui.label(
            egui::RichText::new("Base skin (foundation):")
                .size(12.0)
                .color(Color32::from_rgb(215, 195, 225)),
        );
        let combo_resp = egui::ComboBox::from_id_salt("base_skin_combobox")
            .selected_text(current_base_display)
            .width(field_width)
            .truncate()
            .show_ui(ui, |ui| {
                let skin_items: Vec<(String, String, String, SkinSource)> = state
                    .visible_skins()
                    .into_iter()
                    .map(|s| (s.id.clone(), s.name.clone(), s.creator.clone(), s.source))
                    .collect();

                for (id, name, creator, source) in skin_items {
                    let is_selected = state.selected_base_skin_id.as_deref() == Some(&id);
                    let badge = match source {
                        SkinSource::Lazer => "lazer",
                        SkinSource::Stable => "stable",
                    };
                    let label = format!("[{badge}] {}", truncate_text(&name, 24));
                    let resp = ui
                        .selectable_label(is_selected, label)
                        .on_hover_text(format!("{name}\nCreator: {creator}\nSource: osu!{badge}"));
                    if resp.clicked() {
                        state.select_base_skin(&id);
                    }
                }
            });
        combo_resp.response.on_hover_text(current_base_full);

        ui.add_space(6.0);

        // 2. New skin name (bounded width)
        ui.label(
            egui::RichText::new("New skin name:")
                .size(12.0)
                .color(Color32::from_rgb(215, 195, 225)),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_skin_name)
                .desired_width(field_width)
                .clip_text(true),
        );

        ui.add_space(4.0);

        // 3. Creator (bounded width)
        ui.label(
            egui::RichText::new("Creator:")
                .size(12.0)
                .color(Color32::from_rgb(215, 195, 225)),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_skin_creator)
                .desired_width(field_width)
                .clip_text(true),
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        // 4. Version filters (lazer / stable)
        ui.label(
            egui::RichText::new(format!(
                "{} Game Version Filter:",
                egui_phosphor::regular::FUNNEL
            ))
            .strong()
            .size(12.0)
            .color(Color32::from_rgb(240, 215, 248)),
        );
        ui.add_space(4.0);

        let lazer_count = state.count_by_source(SkinSource::Lazer);
        let stable_count = state.count_by_source(SkinSource::Stable);

        ui.horizontal(|ui| {
            let lazer_icon = if state.show_lazer_skins {
                egui_phosphor::regular::CHECK_SQUARE
            } else {
                egui_phosphor::regular::SQUARE
            };
            let lazer_text = egui::RichText::new(format!("{lazer_icon} lazer ({lazer_count})"))
                .color(if state.show_lazer_skins {
                    Color32::from_rgb(185, 175, 255)
                } else {
                    Color32::from_rgb(135, 120, 148)
                });
            if ui
                .selectable_label(state.show_lazer_skins, lazer_text)
                .on_hover_text("Show or hide osu!lazer skins in selectors")
                .clicked()
            {
                state.show_lazer_skins = !state.show_lazer_skins;
            }

            let stable_icon = if state.show_stable_skins {
                egui_phosphor::regular::CHECK_SQUARE
            } else {
                egui_phosphor::regular::SQUARE
            };
            let stable_text = egui::RichText::new(format!("{stable_icon} stable ({stable_count})"))
                .color(if state.show_stable_skins {
                    Color32::from_rgb(255, 130, 188)
                } else {
                    Color32::from_rgb(135, 120, 148)
                });
            if ui
                .selectable_label(state.show_stable_skins, stable_text)
                .on_hover_text("Show or hide osu!stable skins in selectors")
                .clicked()
            {
                state.show_stable_skins = !state.show_stable_skins;
            }
        });

        ui.add_space(4.0);

        // Custom folder overrides for lazer / stable
        ui.horizontal(|ui| {
            let lazer_tooltip = state
                .lazer_root
                .as_ref()
                .map(|p| {
                    format!(
                        "osu!lazer root: {}\nClick to select a custom folder",
                        p.display()
                    )
                })
                .unwrap_or_else(|| "Select custom osu!lazer data folder".to_string());

            if ui
                .small_button(format!(
                    "{} lazer folder…",
                    egui_phosphor::regular::FOLDER_OPEN
                ))
                .on_hover_text(lazer_tooltip)
                .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title("Select osu!lazer Data Folder")
                    .pick_folder()
                {
                    state.custom_lazer_root = Some(folder);
                    state.is_working = true;
                    let _ = worker_tx.send(WorkerTask::LoadSkins {
                        custom_lazer_root: state.custom_lazer_root.clone(),
                        custom_stable_root: state.custom_stable_root.clone(),
                    });
                }
            }

            let stable_tooltip = state
                .stable_root
                .as_ref()
                .map(|p| {
                    format!(
                        "osu!stable root: {}\nClick to select a custom folder",
                        p.display()
                    )
                })
                .unwrap_or_else(|| {
                    "Select custom osu!stable folder (or Skins subfolder)".to_string()
                });

            if ui
                .small_button(format!(
                    "{} stable folder…",
                    egui_phosphor::regular::FOLDER_OPEN
                ))
                .on_hover_text(stable_tooltip)
                .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title("Select osu!stable Folder (or Skins Directory)")
                    .pick_folder()
                {
                    state.custom_stable_root = Some(folder);
                    state.is_working = true;
                    let _ = worker_tx.send(WorkerTask::LoadSkins {
                        custom_lazer_root: state.custom_lazer_root.clone(),
                        custom_stable_root: state.custom_stable_root.clone(),
                    });
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // 5. Search bar
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                    .color(Color32::from_rgb(255, 130, 188)),
            );
            let search_width = if state.skin_search_query.is_empty() {
                (field_width - 24.0).max(80.0)
            } else {
                (field_width - 50.0).max(70.0)
            };
            let edit = egui::TextEdit::singleline(&mut state.skin_search_query)
                .hint_text("Search skins...")
                .desired_width(search_width);
            ui.add(edit);
            if !state.skin_search_query.is_empty()
                && ui
                    .small_button(egui_phosphor::regular::X)
                    .on_hover_text("Clear search")
                    .clicked()
            {
                state.skin_search_query.clear();
            }
        });

        ui.add_space(6.0);

        // 6. Installed skins list
        let visible_items: Vec<(String, String, String, usize, SkinSource)> = state
            .visible_skins()
            .into_iter()
            .map(|s| {
                (
                    s.id.clone(),
                    s.name.clone(),
                    s.creator.clone(),
                    s.files.len(),
                    s.source,
                )
            })
            .collect();

        ui.label(
            egui::RichText::new(format!(
                "{} Skins ({} shown / {} total)",
                egui_phosphor::regular::STACK,
                visible_items.len(),
                state.installed_skins.len()
            ))
            .size(12.0)
            .color(Color32::from_rgb(215, 195, 225)),
        );

        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .id_salt("sidebar_skins_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(field_width);
                for (id, name, creator, count, source) in visible_items {
                    let is_base = state.selected_base_skin_id.as_deref() == Some(&id);
                    let radio_icon = if is_base {
                        egui_phosphor::regular::RADIO_BUTTON
                    } else {
                        egui_phosphor::regular::CIRCLE
                    };
                    let badge = match source {
                        SkinSource::Lazer => "lazer",
                        SkinSource::Stable => "stable",
                    };
                    let short_name = truncate_text(&name, 18);
                    let text = format!("{radio_icon} [{badge}] {short_name}");
                    let tooltip = format!(
                        "{name}\nCreator: {creator}\nVersion: osu!{badge}\nFiles: {count}\nClick to set as Base Skin"
                    );

                    let resp = ui
                        .add_sized(
                            [field_width, 22.0],
                            egui::SelectableLabel::new(is_base, text),
                        )
                        .on_hover_text(tooltip);
                    if resp.clicked() {
                        state.select_base_skin(&id);
                    }
                }
            });
    });
}
