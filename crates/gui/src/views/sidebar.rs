use eframe::egui;

use crate::state::AppState;

pub fn render_sidebar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.heading(format!(
            "{} 1. Base Skin & Metadata",
            egui_phosphor::regular::FOLDER_OPEN
        ));
        ui.add_space(8.0);

        // 1. Base skin selector
        let current_base_name = state
            .base_skin()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Select skin...".to_string());

        ui.label("Base skin (foundation):");
        egui::ComboBox::from_id_salt("base_skin_combobox")
            .selected_text(current_base_name)
            .width(ui.available_width() - 16.0)
            .show_ui(ui, |ui| {
                let skin_ids: Vec<(String, String)> = state
                    .installed_skins
                    .iter()
                    .map(|s| (s.id.clone(), s.name.clone()))
                    .collect();

                for (id, name) in skin_ids {
                    let is_selected = state.selected_base_skin_id.as_deref() == Some(&id);
                    if ui.selectable_label(is_selected, &name).clicked() {
                        state.select_base_skin(&id);
                    }
                }
            });

        ui.add_space(8.0);

        // 2. New skin name
        ui.label("New skin name:");
        ui.text_edit_singleline(&mut state.new_skin_name);

        ui.add_space(4.0);

        // 3. Creator
        ui.label("Creator:");
        ui.text_edit_singleline(&mut state.new_skin_creator);

        ui.add_space(8.0);

        // 4. Auto-backup
        ui.checkbox(&mut state.auto_backup, "Auto-backup client.realm");

        ui.add_space(10.0);
        ui.separator();

        // 5. Game process status
        if state.is_game_running {
            ui.colored_label(
                egui::Color32::from_rgb(255, 100, 100),
                format!(
                    "{} osu! is running (locked)",
                    egui_phosphor::regular::SHIELD_WARNING
                ),
            );
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(56, 239, 125),
                format!(
                    "{} osu! closed (Realm Bridge ready)",
                    egui_phosphor::regular::SHIELD_CHECK
                ),
            );
        }

        ui.separator();
        ui.add_space(6.0);

        // 6. Installed skins list
        ui.label(format!(
            "{} Installed Skins ({})",
            egui_phosphor::regular::STACK,
            state.installed_skins.len()
        ));

        ui.add_space(4.0);

        let skin_items: Vec<(String, String, usize)> = state
            .installed_skins
            .iter()
            .map(|s| (s.id.clone(), s.name.clone(), s.files.len()))
            .collect();

        egui::ScrollArea::vertical()
            .id_salt("sidebar_skins_scroll")
            .show(ui, |ui| {
                for (id, name, count) in skin_items {
                    let is_base = state.selected_base_skin_id.as_deref() == Some(&id);
                    let text = if is_base {
                        format!("● {} ({} files)", name, count)
                    } else {
                        format!("  {} ({} files)", name, count)
                    };

                    if ui.selectable_label(is_base, text).clicked() {
                        state.select_base_skin(&id);
                    }
                }
            });
    });
}
