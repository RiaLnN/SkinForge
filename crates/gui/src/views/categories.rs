use eframe::egui;
use skinforge_core::categorize::{ComponentCategory, ComponentSlot};
use skinforge_core::mixer::SlotSource;

use crate::state::{ActiveTab, AppState};
use crate::views::colors::render_colors;

pub fn render_categories(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        // Category tabs
        ui.horizontal(|ui| {
            render_tab_button(
                ui,
                state,
                ActiveTab::Category(ComponentCategory::Cursor),
                egui_phosphor::regular::CURSOR,
                "Cursor",
            );
            render_tab_button(
                ui,
                state,
                ActiveTab::Category(ComponentCategory::Gameplay),
                egui_phosphor::regular::CIRCLE_NOTCH,
                "Gameplay",
            );
            render_tab_button(
                ui,
                state,
                ActiveTab::Category(ComponentCategory::Audio),
                egui_phosphor::regular::SPEAKER_HIGH,
                "Audio",
            );
            render_tab_button(
                ui,
                state,
                ActiveTab::Category(ComponentCategory::Interface),
                egui_phosphor::regular::LAYOUT,
                "Interface",
            );
            render_tab_button(
                ui,
                state,
                ActiveTab::Colors,
                egui_phosphor::regular::PALETTE,
                "Colors",
            );
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Active tab contents
        match state.active_tab {
            ActiveTab::Category(cat) => render_category_content(ui, state, cat),
            ActiveTab::Colors => render_colors(ui, state),
        }
    });
}

fn render_tab_button(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tab: ActiveTab,
    icon: &str,
    label: &str,
) {
    let is_active = state.active_tab == tab;
    let text = format!("{icon} {label}");
    if ui.selectable_label(is_active, text).clicked() {
        state.active_tab = tab;
    }
}

fn render_category_content(ui: &mut egui::Ui, state: &mut AppState, category: ComponentCategory) {
    // 1. Batch action toolbar
    ui.horizontal(|ui| {
        ui.label(format!(
            "{} Batch replace:",
            egui_phosphor::regular::LIGHTNING
        ));

        egui::ComboBox::from_id_salt(format!("batch_donor_{:?}", category))
            .selected_text("Select donor for entire category...")
            .show_ui(ui, |ui| {
                let skin_items: Vec<(String, String)> = state
                    .installed_skins
                    .iter()
                    .map(|s| (s.id.clone(), s.name.clone()))
                    .collect();

                for (id, name) in skin_items {
                    if ui.button(&name).clicked() {
                        state.batch_set_category(category, &id);
                    }
                }
            });

        if ui
            .button(format!(
                "{} Reset to base",
                egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE
            ))
            .clicked()
        {
            state.reset_category_to_base(category);
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // 2. Slot list for current category
    let slots_in_cat: Vec<ComponentSlot> = ComponentSlot::all()
        .iter()
        .filter(|s| s.category() == category)
        .copied()
        .collect();

    egui::ScrollArea::vertical()
        .id_salt(format!("slots_scroll_{:?}", category))
        .show(ui, |ui| {
            for slot in slots_in_cat {
                render_slot_row(ui, state, slot);
            }
        });
}

fn render_slot_row(ui: &mut egui::Ui, state: &mut AppState, slot: ComponentSlot) {
    let current_source = state
        .slot_sources
        .get(&slot)
        .cloned()
        .unwrap_or(SlotSource::Base);

    let display_text = match &current_source {
        SlotSource::Base => "Base skin (foundation)".to_string(),
        SlotSource::None => "Disable / Remove (None)".to_string(),
        SlotSource::CustomFile(path) => {
            let name = path.file_name().and_then(|f| f.to_str()).unwrap_or("custom");
            format!("Custom file ({name})")
        }
        SlotSource::Donor(id) => state
            .installed_skins
            .iter()
            .find(|s| &s.id == id)
            .map(|s| format!("Donor: {}", s.name))
            .unwrap_or_else(|| "Unknown donor".to_string()),
    };

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            // Slot label and primary filename
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(slot.display_name()).strong().size(13.0));
                ui.label(
                    egui::RichText::new(slot.primary_filename())
                        .size(10.0)
                        .color(egui::Color32::from_rgb(160, 160, 180)),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Status badge
                match &current_source {
                    SlotSource::Base => {
                        ui.label(
                            egui::RichText::new("Base")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(160, 160, 180)),
                        );
                    }
                    SlotSource::None => {
                        ui.label(
                            egui::RichText::new("Removed")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(255, 180, 50)),
                        );
                    }
                    SlotSource::CustomFile(_) => {
                        ui.label(
                            egui::RichText::new("Custom")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 200, 255)),
                        );
                    }
                    SlotSource::Donor(_) => {
                        ui.label(
                            egui::RichText::new("Replaced")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(56, 239, 125)),
                        );
                    }
                }

                // Source dropdown
                egui::ComboBox::from_id_salt(format!("slot_combo_{:?}", slot))
                    .selected_text(display_text)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                matches!(current_source, SlotSource::Base),
                                "Base skin (foundation)",
                            )
                            .clicked()
                        {
                            state.set_slot_source(slot, SlotSource::Base);
                        }

                        if slot.can_be_none()
                            && ui
                                .selectable_label(
                                    matches!(current_source, SlotSource::None),
                                    "Disable / Remove (None)",
                                )
                                .clicked()
                        {
                            state.set_slot_source(slot, SlotSource::None);
                        }

                        ui.separator();

                        let skin_items: Vec<(String, String)> = state
                            .installed_skins
                            .iter()
                            .map(|s| (s.id.clone(), s.name.clone()))
                            .collect();

                        for (id, name) in skin_items {
                            let is_this_donor = match &current_source {
                                SlotSource::Donor(d_id) => d_id == &id,
                                _ => false,
                            };
                            if ui.selectable_label(is_this_donor, &name).clicked() {
                                state.set_slot_source(slot, SlotSource::Donor(id));
                            }
                        }
                    });
            });
        });
    });

    ui.add_space(4.0);
}
