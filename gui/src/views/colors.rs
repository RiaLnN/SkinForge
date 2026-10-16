use eframe::egui;

use crate::state::AppState;

pub fn render_colors(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("Комбо-цвета записываются в секцию [Colours] нового skin.ini:")
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
                                                "{} Удалить",
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
                    .button(format!(
                        "{} Добавить комбо-цвет",
                        egui_phosphor::regular::PLUS
                    ))
                    .clicked()
            {
                state.combo_colours.push((255, 255, 255));
            }

            if ui
                .button(format!(
                    "{} Сбросить на стандартную палитру",
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
    });
}
