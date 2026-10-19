use eframe::egui::{self, pos2, vec2, Color32, FontId, Rect};
use skinforge_core::bridge::SkinSource;
use skinforge_core::categorize::{ComponentCategory, ComponentSlot, ComponentSubgroup};
use skinforge_core::mixer::SlotSource;

use crate::audio::AudioPlayer;
use crate::state::{truncate_text, ActiveTab, AppState};
use crate::textures::{resolve_slot_file_path, TextureCache};
use crate::views::colors::render_colors;
use crate::views::sidebar::{
    build_skin_badged_job, render_plain_option_row, render_skin_list_row,
};

pub fn render_categories(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
) {
    let avail_w = ui.available_width();
    ui.set_max_width(avail_w);

    ui.vertical(|ui| {
        ui.set_max_width(avail_w);

        // Category tabs (5 component categories + Colors & Settings)
        ui.horizontal_wrapped(|ui| {
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
                ActiveTab::Category(ComponentCategory::Hud),
                egui_phosphor::regular::GAUGE,
                "HUD & Playfield",
            );
            render_tab_button(
                ui,
                state,
                ActiveTab::Category(ComponentCategory::Interface),
                egui_phosphor::regular::LAYOUT,
                "Menus & Screens",
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
                ActiveTab::Colors,
                egui_phosphor::regular::PALETTE,
                "Colors & INI",
            );
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // Active tab contents
        match state.active_tab {
            ActiveTab::Category(cat) => render_category_content(ui, state, textures, audio, cat),
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
    let text = egui::RichText::new(format!("{icon} {label}")).color(if is_active {
        Color32::WHITE
    } else {
        Color32::from_rgb(225, 205, 238)
    });
    if ui.selectable_label(is_active, text).clicked() {
        state.active_tab = tab;
    }
}

fn render_category_content(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
    category: ComponentCategory,
) {
    let donor_candidates: Vec<(String, String, SkinSource)> = state
        .visible_skins()
        .into_iter()
        .map(|s| (s.id.clone(), s.name.clone(), s.source))
        .collect();

    // 1. Category-wide batch action toolbar
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} Category batch:",
                egui_phosphor::regular::LIGHTNING
            ))
            .color(Color32::from_rgb(255, 130, 188)),
        );

        egui::ComboBox::from_id_salt(format!("batch_donor_{:?}", category))
            .selected_text("Apply donor to category...")
            .width(195.0)
            .truncate()
            .show_ui(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for (id, name, source) in &donor_candidates {
                    if render_skin_list_row(ui, 230.0, false, *source, name, name) {
                        state.batch_set_category(category, id);
                        ui.close_menu();
                    }
                }
            });

        if ui
            .button(format!(
                "{} Reset All",
                egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE
            ))
            .on_hover_text("Reset all slots in this category to Base skin")
            .clicked()
        {
            state.reset_category_to_base(category);
        }
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);

    // 2. Collapsible Subgroups within the active category
    let subgroups = ComponentSubgroup::subgroups_for_category(category);
    let scroll_w = ui.available_width();

    egui::ScrollArea::vertical()
        .id_salt(format!("slots_scroll_{:?}", category))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(scroll_w);
            for &subgroup in subgroups {
                render_subgroup_section(ui, state, textures, audio, subgroup, &donor_candidates);
                ui.add_space(6.0);
            }

            if category == ComponentCategory::Cursor {
                ui.add_space(4.0);
                render_cursor_advanced(ui, state);
            }

            if category == ComponentCategory::Gameplay {
                ui.add_space(4.0);
                render_gameplay_advanced(ui, state);
            }
        });
}

fn render_subgroup_section(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
    subgroup: ComponentSubgroup,
    donor_candidates: &[(String, String, SkinSource)],
) {
    let slots = subgroup.slots();
    let total_count = slots.len();
    let overridden_count = slots
        .iter()
        .filter(|s| {
            !matches!(
                state.slot_sources.get(s).unwrap_or(&SlotSource::Base),
                SlotSource::Base
            )
        })
        .count();

    let is_expanded = state.is_subgroup_expanded(subgroup);
    let caret = if is_expanded {
        egui_phosphor::regular::CARET_DOWN
    } else {
        egui_phosphor::regular::CARET_RIGHT
    };

    // Subgroup header bar (osu! warm plum surface)
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(33, 22, 41))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(72, 46, 88)))
        .rounding(7.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let header_label = format!(
                    "{caret} {} ({overridden_count}/{total_count} modified)",
                    subgroup.display_name()
                );
                if ui
                    .selectable_label(
                        is_expanded,
                        egui::RichText::new(header_label)
                            .strong()
                            .size(13.0)
                            .color(Color32::from_rgb(250, 228, 255)),
                    )
                    .on_hover_text("Click to expand or collapse this sub-group")
                    .clicked()
                {
                    state.toggle_subgroup(subgroup);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if overridden_count > 0
                        && ui
                            .small_button(egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE)
                            .on_hover_text("Reset this sub-group to Base skin")
                            .clicked()
                    {
                        state.reset_subgroup_to_base(subgroup);
                    }

                    egui::ComboBox::from_id_salt(format!("subgroup_batch_{:?}", subgroup))
                        .selected_text("Batch sub-group...")
                        .width(135.0)
                        .truncate()
                        .show_ui(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            for (id, name, source) in donor_candidates {
                                if render_skin_list_row(ui, 230.0, false, *source, name, name) {
                                    state.batch_set_subgroup(subgroup, id);
                                    ui.close_menu();
                                }
                            }
                        });
                });
            });

            if is_expanded {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                for slot in slots {
                    render_slot_row(ui, state, textures, audio, slot, donor_candidates);
                }
            }
        });
}

fn make_plain_combo_job(text: &str, color: Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(12.0),
            color,
            ..Default::default()
        },
    );
    job
}

fn render_slot_row(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
    slot: ComponentSlot,
    donor_candidates: &[(String, String, SkinSource)],
) {
    let current_source = state
        .slot_sources
        .get(&slot)
        .cloned()
        .unwrap_or(SlotSource::Base);

    let (selected_job, full_source_tooltip, source_color) = match &current_source {
        SlotSource::Base => (
            make_plain_combo_job("Base skin", Color32::from_rgb(205, 190, 218)),
            "Inherited from Base skin (foundation)".to_string(),
            Color32::from_rgb(205, 190, 218),
        ),
        SlotSource::None => {
            if slot == ComponentSlot::HudLayout {
                (
                    make_plain_combo_job("Default HUD (None)", Color32::from_rgb(255, 185, 75)),
                    "Uses default osu!lazer HUD (no custom skin.json)".to_string(),
                    Color32::from_rgb(255, 185, 75),
                )
            } else {
                (
                    make_plain_combo_job("Disabled (None)", Color32::from_rgb(255, 185, 75)),
                    "Component is disabled / replaced with blank asset".to_string(),
                    Color32::from_rgb(255, 185, 75),
                )
            }
        }
        SlotSource::CustomFile(path) => {
            let name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("custom");
            (
                make_plain_combo_job(
                    &format!("File: {}", truncate_text(name, 13)),
                    Color32::from_rgb(125, 215, 255),
                ),
                format!("Custom file from disk:\n{}", path.display()),
                Color32::from_rgb(125, 215, 255),
            )
        }
        SlotSource::Donor(id) => state
            .installed_skins
            .iter()
            .find(|s| &s.id == id)
            .map(|s| {
                let badge = s.badge_label();
                (
                    build_skin_badged_job(s.source, &truncate_text(&s.name, 12), false),
                    format!("Donor skin: {} (osu!{badge})", s.name),
                    Color32::from_rgb(255, 130, 188),
                )
            })
            .unwrap_or_else(|| {
                (
                    make_plain_combo_job("Unknown donor", Color32::from_rgb(255, 120, 120)),
                    "Selected donor skin is not loaded".to_string(),
                    Color32::from_rgb(255, 120, 120),
                )
            }),
    };

    let mut new_custom_file = None;

    egui::Frame::none()
        .fill(Color32::from_rgb(24, 17, 30))
        .rounding(6.0)
        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(56, 38, 68)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Column 1: 28x28 Mini Preview OR Audio Play/Stop Toggle
                if slot.is_audio() {
                    let is_none = matches!(current_source, SlotSource::None);
                    let is_playing = audio.is_playing_slot(slot);
                    ui.add_enabled_ui(!is_none, |ui| {
                        let (icon, tint, tip) = if is_playing {
                            (
                                egui_phosphor::regular::STOP,
                                Color32::from_rgb(255, 102, 171),
                                "Stop audio playback",
                            )
                        } else {
                            (
                                egui_phosphor::regular::PLAY,
                                Color32::from_rgb(255, 150, 200),
                                "Play sound preview (uses default osu! sound if skin omits this file)",
                            )
                        };
                        let btn = egui::Button::new(egui::RichText::new(icon).color(tint))
                            .min_size(vec2(28.0, 28.0));
                        if ui.add(btn).on_hover_text(tip).clicked() {
                            let path = resolve_slot_file_path(state, slot);
                            audio.toggle_slot(slot, path.as_deref());
                        }
                    });
                } else {
                    let (thumb_rect, thumb_resp) =
                        ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::click());
                    ui.painter()
                        .rect_filled(thumb_rect, 4.0, Color32::from_rgb(16, 11, 20));
                    ui.painter().rect_stroke(
                        thumb_rect,
                        4.0,
                        egui::Stroke::new(1.0_f32, Color32::from_rgb(66, 44, 80)),
                    );

                    if slot == ComponentSlot::HudLayout {
                        ui.painter().text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::CODE,
                            egui::FontId::proportional(14.0),
                            Color32::from_rgb(255, 130, 188),
                        );
                    } else if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, slot) {
                        let tex_size = tex.size_vec2();
                        let max_dim = tex_size.x.max(tex_size.y).max(1.0);
                        let fit_scale = (24.0 / max_dim).min(2.0);
                        let draw_size =
                            (tex_size * fit_scale).clamp(vec2(6.0, 6.0), vec2(24.0, 24.0));
                        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                        ui.painter().image(
                            tex.id(),
                            Rect::from_center_size(thumb_rect.center(), draw_size),
                            uv,
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::IMAGE,
                            egui::FontId::proportional(13.0),
                            Color32::from_rgb(115, 95, 130),
                        );
                    }

                    if thumb_resp.clicked() || thumb_resp.hovered() {
                        state.inspected_slot = slot;
                    }
                    thumb_resp.on_hover_text(format!(
                        "{} ({})\nHover or click to inspect in Preview panel",
                        slot.display_name(),
                        slot.primary_filename()
                    ));
                }

                ui.add_space(4.0);

                // Column 2: Fixed-width Slot Name & Primary Filename (never overlapped by donor selector)
                let label_width = 148.0;
                let label_resp = ui.allocate_ui_with_layout(
                    vec2(label_width, 28.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_max_width(label_width);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(truncate_text(slot.display_name(), 20))
                                    .strong()
                                    .size(12.5)
                                    .color(source_color),
                            )
                            .truncate(),
                        );
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(truncate_text(slot.primary_filename(), 23))
                                    .size(10.0)
                                    .color(Color32::from_rgb(160, 140, 175)),
                            )
                            .truncate(),
                        );
                    },
                );
                if !slot.is_audio()
                    && slot != ComponentSlot::HudLayout
                    && (label_resp.response.clicked() || label_resp.response.hovered())
                {
                    state.inspected_slot = slot;
                }
                label_resp.response.on_hover_text(format!(
                    "{}\nFile: {}\n{}",
                    slot.display_name(),
                    slot.primary_filename(),
                    full_source_tooltip
                ));

                // Column 3: Bounded Donor ComboBox with .truncate() and unified STABLE / LAZER badges
                let action_buttons_width = 62.0;
                let combo_width =
                    (ui.available_width() - action_buttons_width).clamp(110.0, 200.0);

                let combo_resp = egui::ComboBox::from_id_salt(format!("slot_combo_{:?}", slot))
                    .selected_text(selected_job)
                    .width(combo_width)
                    .truncate()
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        let popup_row_w = 230.0;

                        if render_plain_option_row(
                            ui,
                            popup_row_w,
                            matches!(current_source, SlotSource::Base),
                            "Base skin (foundation)",
                            Color32::from_rgb(235, 222, 245),
                        ) {
                            state.set_slot_source(slot, SlotSource::Base);
                            ui.close_menu();
                        }

                        if slot.can_be_none() {
                            let none_label = if slot == ComponentSlot::HudLayout {
                                "Default Lazer HUD (None)"
                            } else {
                                "Disable / Remove (None)"
                            };
                            if render_plain_option_row(
                                ui,
                                popup_row_w,
                                matches!(current_source, SlotSource::None),
                                none_label,
                                Color32::from_rgb(255, 195, 100),
                            ) {
                                state.set_slot_source(slot, SlotSource::None);
                                ui.close_menu();
                            }
                        }

                        ui.separator();

                        let filtered_donors: Vec<&(String, String, SkinSource)> = donor_candidates
                            .iter()
                            .filter(|(id, _, _)| {
                                if slot == ComponentSlot::HudLayout {
                                    state
                                        .installed_skins
                                        .iter()
                                        .find(|s| &s.id == id)
                                        .is_some_and(|s| {
                                            s.files.iter().any(|f| {
                                                f.filename.eq_ignore_ascii_case("skin.json")
                                            })
                                        })
                                } else {
                                    true
                                }
                            })
                            .collect();

                        if filtered_donors.is_empty() && slot == ComponentSlot::HudLayout {
                            ui.label(
                                egui::RichText::new("(No visible skins have skin.json)")
                                    .size(11.0)
                                    .color(Color32::from_rgb(160, 140, 175)),
                            );
                        }

                        for (id, name, source) in filtered_donors {
                            let is_this_donor = match &current_source {
                                SlotSource::Donor(d_id) => d_id == id,
                                _ => false,
                            };
                            if render_skin_list_row(
                                ui,
                                popup_row_w,
                                is_this_donor,
                                *source,
                                name,
                                name,
                            ) {
                                state.set_slot_source(slot, SlotSource::Donor(id.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                combo_resp.response.on_hover_text(&full_source_tooltip);

                // Column 4: Compact Action Buttons (Import file + Reset to Base)
                if slot.supports_single_file_import()
                    && ui
                        .button(egui_phosphor::regular::FOLDER_OPEN)
                        .on_hover_text("Import custom file from disk...")
                        .clicked()
                {
                    let mut dialog = rfd::FileDialog::new();
                    if slot == ComponentSlot::HudLayout {
                        dialog = dialog.add_filter("osu!lazer HUD Layout", &["json"]);
                    } else if slot.is_audio() {
                        dialog = dialog.add_filter("Audio files", &["wav", "ogg", "mp3"]);
                    } else {
                        dialog = dialog.add_filter("Image files", &["png", "jpg", "jpeg"]);
                    }

                    if let Some(path) = dialog.pick_file() {
                        new_custom_file = Some(path);
                    }
                }

                let is_modified = !matches!(current_source, SlotSource::Base);
                ui.add_enabled_ui(is_modified, |ui| {
                    if ui
                        .button(egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE)
                        .on_hover_text("Reset slot to Base skin")
                        .clicked()
                    {
                        state.set_slot_source(slot, SlotSource::Base);
                    }
                });
            });
        });

    if let Some(path) = new_custom_file {
        state.set_slot_custom_file(slot, path);
    }

    ui.add_space(3.0);
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
