use eframe::egui;

use crate::state::AppState;

pub fn render_colors(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(
                "Combo colours are written to the [Colours] section of the new skin.ini:",
            )
            .size(12.0)
            .color(egui::Color32::from_rgb(170, 170, 190)),
        );
        ui.add_space(8.0);

        let mut to_remove = None;
        let colors_count = state.combo_colours.len();

        egui::ScrollArea::vertical()
            .id_salt("colors_scroll")
            .show(ui, |ui| {
                for (i, rgb) in state.combo_colours.iter_mut().enumerate() {
                    let mut srgb = [rgb.0, rgb.1, rgb.2];

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Combo {}:", i + 1))
                                    .strong()
                                    .size(13.0),
                            );

                            if ui.color_edit_button_srgb(&mut srgb).changed() {
                                *rgb = (srgb[0], srgb[1], srgb[2]);
                            }

                            ui.label(format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2));

                            if colors_count > 1 {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(format!(
                                                "{} Delete",
                                                egui_phosphor::regular::TRASH
                                            ))
                                            .clicked()
                                        {
                                            to_remove = Some(i);
                                        }
                                    },
                                );
                            }
                        });
                    });

                    ui.add_space(4.0);
                }
            });

        if let Some(idx) = to_remove {
            state.combo_colours.remove(idx);
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if state.combo_colours.len() < 8
                && ui
                    .button(format!("{} Add combo colour", egui_phosphor::regular::PLUS))
                    .clicked()
            {
                state.combo_colours.push((255, 255, 255));
            }

            if ui
                .button(format!(
                    "{} Reset to default palette",
                    egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE
                ))
                .clicked()
            {
                state.combo_colours = vec![
                    (255, 102, 171),
                    (51, 187, 238),
                    (136, 255, 68),
                    (255, 170, 34),
                ];
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        let adv_title = if state.show_advanced_colors {
            format!(
                "{} Advanced Slider & Interface Colors (skin.ini [Colours])",
                egui_phosphor::regular::CARET_DOWN
            )
        } else {
            format!(
                "{} Advanced Slider & Interface Colors (skin.ini [Colours])",
                egui_phosphor::regular::CARET_RIGHT
            )
        };

        if ui.button(egui::RichText::new(adv_title).strong()).clicked() {
            state.show_advanced_colors = !state.show_advanced_colors;
        }

        if state.show_advanced_colors {
            ui.add_space(6.0);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.vertical(|ui| {
                    render_color_override_row(
                        ui,
                        "SliderTrackOverride:",
                        state.extended_ini.slider_track_override(),
                        (0, 0, 0),
                        |val| state.extended_ini.set_slider_track_override(val),
                    );
                    render_color_override_row(
                        ui,
                        "SliderBorder:",
                        state.extended_ini.slider_border(),
                        (255, 255, 255),
                        |val| state.extended_ini.set_slider_border(val),
                    );
                    render_color_override_row(
                        ui,
                        "SliderBall:",
                        state.extended_ini.slider_ball(),
                        (255, 255, 255),
                        |val| state.extended_ini.set_slider_ball(val),
                    );
                    render_color_override_row(
                        ui,
                        "MenuGlow:",
                        state.extended_ini.menu_glow(),
                        (0, 78, 155),
                        |val| state.extended_ini.set_menu_glow(val),
                    );
                    render_color_override_row(
                        ui,
                        "SpinnerApproachCircle:",
                        state.extended_ini.spinner_approach_circle(),
                        (77, 139, 217),
                        |val| state.extended_ini.set_spinner_approach_circle(val),
                    );
                    render_color_override_row(
                        ui,
                        "SongSelectActiveText:",
                        state.extended_ini.song_select_active_text(),
                        (255, 255, 255),
                        |val| state.extended_ini.set_song_select_active_text(val),
                    );
                    render_color_override_row(
                        ui,
                        "SongSelectInactiveText:",
                        state.extended_ini.song_select_inactive_text(),
                        (192, 192, 192),
                        |val| state.extended_ini.set_song_select_inactive_text(val),
                    );
                });
            });
        }
    });
}

fn render_color_override_row(
    ui: &mut egui::Ui,
    label: &str,
    current: Option<(u8, u8, u8)>,
    default_color: (u8, u8, u8),
    mut on_change: impl FnMut(Option<(u8, u8, u8)>),
) {
    ui.horizontal(|ui| {
        let mut is_enabled = current.is_some();
        if ui.checkbox(&mut is_enabled, label).changed() {
            if is_enabled {
                on_change(Some(default_color));
            } else {
                on_change(None);
            }
        }

        if let Some(rgb) = current {
            let mut srgb = [rgb.0, rgb.1, rgb.2];
            if ui.color_edit_button_srgb(&mut srgb).changed() {
                on_change(Some((srgb[0], srgb[1], srgb[2])));
            }
            ui.label(format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2));
        } else {
            ui.label(
                egui::RichText::new("(Default / dynamic tint)")
                    .color(egui::Color32::from_rgb(140, 140, 160))
                    .size(11.0),
            );
        }
    });
}
