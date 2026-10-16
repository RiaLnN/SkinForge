use eframe::egui;

use crate::state::AppState;

pub fn render_sidebar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.heading(format!(
            "{} 1. Базовый скин и имя",
            egui_phosphor::regular::FOLDER_OPEN
        ));
        ui.add_space(8.0);

        // 1. Выбор базы
        let current_base_name = state
            .base_skin()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Выберите скин...".to_string());

        ui.label("Базовый скин (основа):");
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

        // 2. Имя нового скина
        ui.label("Название нового скина:");
        ui.text_edit_singleline(&mut state.new_skin_name);

        ui.add_space(4.0);

        // 3. Автор
        ui.label("Автор (Creator):");
        ui.text_edit_singleline(&mut state.new_skin_creator);

        ui.add_space(8.0);

        // 4. Авто-бэкап
        ui.checkbox(
            &mut state.auto_backup,
            "Авто-бэкап client.realm",
        );

        ui.add_space(10.0);
        ui.separator();

        // 5. Статус процесса
        if state.is_game_running {
            ui.colored_label(
                egui::Color32::from_rgb(255, 100, 100),
                format!(
                    "{} osu! запущен (запись закрыта)",
                    egui_phosphor::regular::SHIELD_WARNING
                ),
            );
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(56, 239, 125),
                format!(
                    "{} osu! закрыт (Realm Bridge готов)",
                    egui_phosphor::regular::SHIELD_CHECK
                ),
            );
        }

        ui.separator();
        ui.add_space(6.0);

        // 6. Список установленных скинов
        ui.label(format!(
            "{} Установленные скины ({})",
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
                        format!("● {} ({} файлов)", name, count)
                    } else {
                        format!("  {} ({} файлов)", name, count)
                    };

                    if ui.selectable_label(is_base, text).clicked() {
                        state.select_base_skin(&id);
                    }
                }
            });
    });
}
