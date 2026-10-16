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

            // Accordion for Cursor category
            if category == ComponentCategory::Cursor {
                ui.add_space(8.0);
                render_cursor_advanced(ui, state);
            }

            // Accordion for Gameplay category
            if category == ComponentCategory::Gameplay {
                ui.add_space(8.0);
                render_gameplay_advanced(ui, state);
            }
        });
}

fn render_cursor_advanced(ui: &mut egui::Ui, state: &mut AppState) {
    let title = if state.show_advanced_cursor {
        format!(
            "{} Advanced Cursor Settings (skin.ini [General])",
            egui_phosphor::regular::CARET_DOWN
        )
    } else {
        format!(
            "{} Advanced Cursor Settings (skin.ini [General])",
            egui_phosphor::regular::CARET_RIGHT
        )
    };

    if ui.button(egui::RichText::new(title).strong()).clicked() {
        state.show_advanced_cursor = !state.show_advanced_cursor;
    }

    if state.show_advanced_cursor {
        ui.add_space(4.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                let mut rotate = state.extended_ini.cursor_rotate().unwrap_or(true);
                if ui
                    .checkbox(&mut rotate, "Rotate cursor with movement (CursorRotate)")
                    .changed()
                {
                    state.extended_ini.set_cursor_rotate(Some(rotate));
                }
                let mut expand = state.extended_ini.cursor_expand().unwrap_or(true);
                if ui
                    .checkbox(&mut expand, "Expand cursor on click (CursorExpand)")
                    .changed()
                {
                    state.extended_ini.set_cursor_expand(Some(expand));
                }
                let mut trail_rotate = state.extended_ini.cursor_trail_rotate().unwrap_or(true);
                if ui
                    .checkbox(&mut trail_rotate, "Rotate cursor trail (CursorTrailRotate)")
                    .changed()
                {
                    state
                        .extended_ini
                        .set_cursor_trail_rotate(Some(trail_rotate));
                }
                let mut center = state.extended_ini.cursor_center().unwrap_or(true);
                if ui
                    .checkbox(&mut center, "Center cursor origin (CursorCenter)")
                    .changed()
                {
                    state.extended_ini.set_cursor_center(Some(center));
                }
            });
        });
    }
}

fn render_gameplay_advanced(ui: &mut egui::Ui, state: &mut AppState) {
    let title = if state.show_advanced_gameplay {
        format!(
            "{} Advanced Gameplay & Fonts Settings (skin.ini)",
            egui_phosphor::regular::CARET_DOWN
        )
    } else {
        format!(
            "{} Advanced Gameplay & Fonts Settings (skin.ini)",
            egui_phosphor::regular::CARET_RIGHT
        )
    };

    if ui.button(egui::RichText::new(title).strong()).clicked() {
        state.show_advanced_gameplay = !state.show_advanced_gameplay;
    }

    if state.show_advanced_gameplay {
        ui.add_space(4.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                let mut overlay_above = state
                    .extended_ini
                    .hit_circle_overlay_above_number()
                    .unwrap_or(true);
                if ui
                    .checkbox(
                        &mut overlay_above,
                        "HitCircle Overlay above number (HitCircleOverlayAboveNumber)",
                    )
                    .changed()
                {
                    state
                        .extended_ini
                        .set_hit_circle_overlay_above_number(Some(overlay_above));
                }
                let mut ball_tint = state.extended_ini.allow_slider_ball_tint().unwrap_or(false);
                if ui
                    .checkbox(
                        &mut ball_tint,
                        "Allow SliderBall combo tint (AllowSliderBallTint)",
                    )
                    .changed()
                {
                    state
                        .extended_ini
                        .set_allow_slider_ball_tint(Some(ball_tint));
                }
                let mut layered_sounds = state.extended_ini.layered_hit_sounds().unwrap_or(true);
                if ui
                    .checkbox(&mut layered_sounds, "Layered hitsounds (LayeredHitSounds)")
                    .changed()
                {
                    state
                        .extended_ini
                        .set_layered_hit_sounds(Some(layered_sounds));
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Animation Framerate:");
                    let mut fps = state.extended_ini.animation_framerate().unwrap_or(-1);
                    if ui
                        .add(egui::DragValue::new(&mut fps).range(-1..=240))
                        .changed()
                    {
                        state.extended_ini.set_animation_framerate(if fps == -1 {
                            None
                        } else {
                            Some(fps)
                        });
                    }
                    ui.label("(fps, -1 for default/sync)");
                });

                ui.horizontal(|ui| {
                    ui.label("HitCircle Overlap:");
                    let mut overlap = state.extended_ini.hit_circle_overlap().unwrap_or(-2);
                    if ui
                        .add(egui::DragValue::new(&mut overlap).range(-20..=50))
                        .changed()
                    {
                        state.extended_ini.set_hit_circle_overlap(Some(overlap));
                    }
                    ui.label("px");
                });

                ui.horizontal(|ui| {
                    ui.label("Score Overlap:");
                    let mut overlap = state.extended_ini.score_overlap().unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut overlap).range(-20..=50))
                        .changed()
                    {
                        state.extended_ini.set_score_overlap(Some(overlap));
                    }
                    ui.label("px");
                });

                ui.horizontal(|ui| {
                    ui.label("Combo Overlap:");
                    let mut overlap = state.extended_ini.combo_overlap().unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut overlap).range(-20..=50))
                        .changed()
                    {
                        state.extended_ini.set_combo_overlap(Some(overlap));
                    }
                    ui.label("px");
                });
            });
        });
    }
}

fn render_slot_row(ui: &mut egui::Ui, state: &mut AppState, slot: ComponentSlot) {
    let current_source = state
        .slot_sources
        .get(&slot)
        .cloned()
        .unwrap_or(SlotSource::Base);

    let display_text = match &current_source {
        SlotSource::Base => "Base skin (foundation)".to_string(),
        SlotSource::None => {
            if slot == ComponentSlot::HudLayout {
                "Default Lazer HUD (None)".to_string()
            } else {
                "Disable / Remove (None)".to_string()
            }
        }
        SlotSource::CustomFile(path) => {
            let name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("custom");
            format!("Custom file ({name})")
        }
        SlotSource::Donor(id) => state
            .installed_skins
            .iter()
            .find(|s| &s.id == id)
            .map(|s| format!("Donor: {}", s.name))
            .unwrap_or_else(|| "Unknown donor".to_string()),
    };

    let mut new_custom_file = None;

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
                // If custom file is selected, show a clear button
                if matches!(current_source, SlotSource::CustomFile(_))
                    && ui.button("✕").on_hover_text("Reset to base").clicked()
                {
                    state.set_slot_source(slot, SlotSource::Base);
                }

                // File import button
                if slot.supports_single_file_import() {
                    let import_text = format!("{} Import...", egui_phosphor::regular::FOLDER_OPEN);
                    if ui.button(import_text).clicked() {
                        let mut dialog = rfd::FileDialog::new();
                        if slot == ComponentSlot::HudLayout {
                            dialog = dialog.add_filter("osu!lazer HUD Layout", &["json"]);
                        } else if slot.category() == ComponentCategory::Audio
                            || matches!(
                                slot,
                                ComponentSlot::MenuClick
                                    | ComponentSlot::MenuHit
                                    | ComponentSlot::MenuBack
                                    | ComponentSlot::Applause
                                    | ComponentSlot::FailSound
                                    | ComponentSlot::PauseLoop
                            )
                        {
                            dialog = dialog.add_filter("Audio files", &["wav", "ogg", "mp3"]);
                        } else {
                            dialog = dialog.add_filter("Image files", &["png", "jpg", "jpeg"]);
                        }

                        if let Some(path) = dialog.pick_file() {
                            new_custom_file = Some(path);
                        }
                    }
                }

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
                    .width(200.0)
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

                        if slot.can_be_none() {
                            let none_label = if slot == ComponentSlot::HudLayout {
                                "Default Lazer HUD (None)"
                            } else {
                                "Disable / Remove (None)"
                            };
                            if ui
                                .selectable_label(
                                    matches!(current_source, SlotSource::None),
                                    none_label,
                                )
                                .clicked()
                            {
                                state.set_slot_source(slot, SlotSource::None);
                            }
                        }

                        ui.separator();

                        let skin_items: Vec<(String, String)> = state
                            .installed_skins
                            .iter()
                            .filter(|s| {
                                if slot == ComponentSlot::HudLayout {
                                    s.files
                                        .iter()
                                        .any(|f| f.filename.eq_ignore_ascii_case("skin.json"))
                                } else {
                                    true
                                }
                            })
                            .map(|s| (s.id.clone(), s.name.clone()))
                            .collect();

                        if skin_items.is_empty() && slot == ComponentSlot::HudLayout {
                            ui.label(
                                egui::RichText::new("(No donor skins have skin.json)")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(140, 140, 160)),
                            );
                        }

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

    if let Some(path) = new_custom_file {
        state.set_slot_custom_file(slot, path);
    }

    ui.add_space(4.0);
}
