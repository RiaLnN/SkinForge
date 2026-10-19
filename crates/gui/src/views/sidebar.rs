use std::sync::mpsc::Sender;

use eframe::egui;
use skinforge_core::bridge::SkinSource;

use crate::state::{truncate_text, AppState};
use crate::worker::WorkerTask;

pub fn render_sidebar(ui: &mut egui::Ui, state: &mut AppState, worker_tx: &Sender<WorkerTask>) {
    ui.vertical(|ui| {
        ui.heading(format!(
            "{} 1. Base Skin & Filter",
            egui_phosphor::regular::FOLDER_OPEN
        ));
        ui.add_space(6.0);

        // 1. Base skin selector
        let (current_base_display, current_base_full) = state
            .base_skin()
            .map(|s| {
                let badge = s.badge_label();
                let short = truncate_text(&s.name, 22);
                (
                    format!("[{badge}] {short}"),
                    format!("{} ({}) — by {}", s.name, badge, s.creator),
                )
            })
            .unwrap_or_else(|| ("Select skin...".to_string(), "No base skin selected".to_string()));

        ui.label("Base skin (foundation):");
        let combo_resp = egui::ComboBox::from_id_salt("base_skin_combobox")
            .selected_text(current_base_display)
            .width((ui.available_width() - 8.0).max(160.0))
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
                    let label = format!("[{badge}] {}", truncate_text(&name, 28));
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

        // 2. New skin name
        ui.label("New skin name:");
        ui.text_edit_singleline(&mut state.new_skin_name);

        ui.add_space(4.0);

        // 3. Creator
        ui.label("Creator:");
        ui.text_edit_singleline(&mut state.new_skin_creator);

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
            .size(12.0),
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
                    egui::Color32::from_rgb(162, 155, 254)
                } else {
                    egui::Color32::from_rgb(130, 130, 150)
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
                    egui::Color32::from_rgb(255, 126, 179)
                } else {
                    egui::Color32::from_rgb(130, 130, 150)
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
                .map(|p| format!("osu!lazer root: {}\nClick to select a custom folder", p.display()))
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
                .map(|p| format!("osu!stable root: {}\nClick to select a custom folder", p.display()))
                .unwrap_or_else(|| "Select custom osu!stable folder (or Skins subfolder)".to_string());

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
            ui.label(egui_phosphor::regular::MAGNIFYING_GLASS);
            let edit = egui::TextEdit::singleline(&mut state.skin_search_query)
                .hint_text("Search skins...")
                .desired_width((ui.available_width() - 28.0).max(80.0));
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
            .size(12.0),
        );

        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .id_salt("sidebar_skins_scroll")
            .show(ui, |ui| {
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
                    let short_name = truncate_text(&name, 20);
                    let text = format!("{radio_icon} [{badge}] {short_name} ({count})");
                    let tooltip = format!(
                        "{name}\nCreator: {creator}\nVersion: osu!{badge}\nFiles: {count}\nClick to set as Base Skin"
                    );

                    if ui
                        .selectable_label(is_base, text)
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        state.select_base_skin(&id);
                    }
                }
            });
    });
}
