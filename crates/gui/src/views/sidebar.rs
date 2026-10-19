use std::sync::mpsc::Sender;

use eframe::egui::{self, pos2, vec2, Color32, FontId, Rect, Stroke};
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
                "{} Base Skin & Filter",
                egui_phosphor::regular::FOLDER_OPEN
            ))
            .color(Color32::from_rgb(255, 130, 188)),
        );
        ui.add_space(6.0);

        // 1. Base skin selector (strictly truncated + ComboBox::truncate so long names never stretch the panel)
        let (current_base_job, current_base_full) = state
            .base_skin()
            .map(|s| {
                let short = truncate_text(&s.name, 15);
                let job = build_skin_badged_job(s.source, &short, false);
                (
                    job,
                    format!("{} ({}) — by {}", s.name, s.badge_label(), s.creator),
                )
            })
            .unwrap_or_else(|| {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    "Select skin...",
                    0.0,
                    egui::TextFormat {
                        font_id: FontId::proportional(12.5),
                        color: Color32::from_rgb(215, 195, 225),
                        ..Default::default()
                    },
                );
                (job, "No base skin selected".to_string())
            });

        ui.label(
            egui::RichText::new("Base skin (foundation):")
                .size(12.0)
                .color(Color32::from_rgb(215, 195, 225)),
        );
        let combo_resp = egui::ComboBox::from_id_salt("base_skin_combobox")
            .selected_text(current_base_job)
            .width(field_width)
            .truncate()
            .show_ui(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
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
                    let tooltip = format!("{name}\nCreator: {creator}\nSource: osu!{badge}");
                    if render_skin_list_row(ui, 230.0, is_selected, source, &name, &tooltip) {
                        state.select_base_skin(&id);
                        ui.close_menu();
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

        // 4. Version filters (lazer / stable) — high-contrast dark-rose active toggle with bright border & white text
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
            if render_version_filter_toggle(
                ui,
                state.show_lazer_skins,
                &format!("lazer ({lazer_count})"),
                "Show or hide osu!lazer skins in selectors",
            ) {
                state.show_lazer_skins = !state.show_lazer_skins;
            }

            if render_version_filter_toggle(
                ui,
                state.show_stable_skins,
                &format!("stable ({stable_count})"),
                "Show or hide osu!stable skins in selectors",
            ) {
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

        // 6. Installed skins list (left-aligned rows, distinct version badge pill, background-only active highlight)
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
                ui.spacing_mut().item_spacing.y = 4.0;

                for (id, name, creator, count, source) in visible_items {
                    let is_base = state.selected_base_skin_id.as_deref() == Some(&id);
                    let badge_lower = match source {
                        SkinSource::Lazer => "lazer",
                        SkinSource::Stable => "stable",
                    };
                    let tooltip = format!(
                        "{name}\nCreator: {creator}\nVersion: osu!{badge_lower}\nFiles: {count}\nClick to set as Base Skin"
                    );

                    if render_skin_list_row(ui, field_width, is_base, source, &name, &tooltip) {
                        state.select_base_skin(&id);
                    }
                }
            });
    });
}

/// Renders a high-contrast toggle button for the Game Version Filter without clashing colors.
fn render_version_filter_toggle(
    ui: &mut egui::Ui,
    active: bool,
    label: &str,
    tooltip: &str,
) -> bool {
    let icon = if active {
        egui_phosphor::regular::CHECK_SQUARE
    } else {
        egui_phosphor::regular::SQUARE
    };

    let (bg_fill, stroke, text_color) = if active {
        (
            Color32::from_rgb(76, 26, 56),
            Stroke::new(1.5_f32, Color32::from_rgb(255, 102, 171)),
            Color32::WHITE,
        )
    } else {
        (
            Color32::from_rgb(24, 17, 30),
            Stroke::new(1.0_f32, Color32::from_rgb(56, 38, 68)),
            Color32::from_rgb(150, 134, 165),
        )
    };

    let btn = egui::Button::new(
        egui::RichText::new(format!("{icon} {label}"))
            .size(12.0)
            .strong()
            .color(text_color),
    )
    .fill(bg_fill)
    .stroke(stroke)
    .rounding(6.0);

    ui.add(btn).on_hover_text(tooltip).clicked()
}

/// Renders a single left-aligned skin row (used in both the bottom sidebar list and all base/donor dropdowns):
/// - No radio circle icon (active state is indicated by row background & border)
/// - Distinct pill badge for `LAZER` / `STABLE` on the left so it never merges with the skin name
/// - Left-aligned truncated skin name
pub fn render_skin_list_row(
    ui: &mut egui::Ui,
    width: f32,
    is_active: bool,
    source: SkinSource,
    name: &str,
    tooltip: &str,
) -> bool {
    let row_height = 26.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, row_height), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let (row_fill, row_stroke) = if is_active {
            (
                Color32::from_rgb(175, 42, 108),
                Stroke::new(1.0_f32, Color32::from_rgb(255, 130, 190)),
            )
        } else if hovered {
            (
                Color32::from_rgb(48, 30, 62),
                Stroke::new(1.0_f32, Color32::from_rgb(98, 60, 120)),
            )
        } else {
            (
                Color32::from_rgb(27, 19, 34),
                Stroke::new(1.0_f32, Color32::from_rgb(46, 31, 56)),
            )
        };

        ui.painter().rect(rect, 5.0, row_fill, row_stroke);

        // Distinct version pill badge on the left edge
        let pill_w = 44.0;
        let pill_h = 16.0;
        let pill_rect = Rect::from_min_size(
            pos2(rect.min.x + 6.0, rect.center().y - pill_h * 0.5),
            vec2(pill_w, pill_h),
        );
        let (pill_label, pill_fill, pill_border, pill_text_color) = match source {
            SkinSource::Stable => (
                "STABLE",
                Color32::from_rgb(38, 14, 30),
                Stroke::new(1.0_f32, Color32::from_rgb(255, 120, 182)),
                Color32::from_rgb(255, 195, 225),
            ),
            SkinSource::Lazer => (
                "LAZER",
                Color32::from_rgb(22, 16, 38),
                Stroke::new(1.0_f32, Color32::from_rgb(190, 150, 255)),
                Color32::from_rgb(220, 200, 255),
            ),
        };

        ui.painter().rect(pill_rect, 4.0, pill_fill, pill_border);
        ui.painter().text(
            pill_rect.center(),
            egui::Align2::CENTER_CENTER,
            pill_label,
            FontId::proportional(9.5),
            pill_text_color,
        );

        // Left-aligned skin name starting after the badge pill
        let text_x = pill_rect.max.x + 8.0;
        let avail_text_w = (rect.max.x - text_x - 6.0).max(20.0);
        let max_chars = ((avail_text_w / 6.8) as usize).clamp(10, 26);
        let short_name = truncate_text(name, max_chars);
        let name_color = if is_active {
            Color32::WHITE
        } else {
            Color32::from_rgb(238, 226, 246)
        };

        ui.painter().text(
            pos2(text_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            short_name,
            FontId::proportional(12.5),
            name_color,
        );
    }

    resp.on_hover_text(tooltip).clicked()
}

/// Renders a left-aligned non-skin option row inside donor dropdowns (e.g. "Base skin (foundation)" or "Disable / Remove (None)").
pub fn render_plain_option_row(
    ui: &mut egui::Ui,
    width: f32,
    is_active: bool,
    label: &str,
    text_color: Color32,
) -> bool {
    let row_height = 25.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, row_height), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let (row_fill, row_stroke) = if is_active {
            (
                Color32::from_rgb(175, 42, 108),
                Stroke::new(1.0_f32, Color32::from_rgb(255, 130, 190)),
            )
        } else if hovered {
            (
                Color32::from_rgb(48, 30, 62),
                Stroke::new(1.0_f32, Color32::from_rgb(98, 60, 120)),
            )
        } else {
            (
                Color32::from_rgb(27, 19, 34),
                Stroke::new(1.0_f32, Color32::from_rgb(46, 31, 56)),
            )
        };

        ui.painter().rect(rect, 5.0, row_fill, row_stroke);
        ui.painter().text(
            pos2(rect.min.x + 8.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.0),
            if is_active { Color32::WHITE } else { text_color },
        );
    }

    resp.clicked()
}

/// Builds a `LayoutJob` with a styled `STABLE` / `LAZER` badge pill for closed `ComboBox` selected text.
pub fn build_skin_badged_job(
    source: SkinSource,
    short_name: &str,
    is_selected: bool,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let (badge_text, badge_color, badge_bg) = match source {
        SkinSource::Stable => (
            " STABLE ",
            Color32::from_rgb(255, 195, 225),
            Color32::from_rgb(42, 16, 34),
        ),
        SkinSource::Lazer => (
            " LAZER ",
            Color32::from_rgb(220, 200, 255),
            Color32::from_rgb(25, 18, 42),
        ),
    };

    job.append(
        badge_text,
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(10.0),
            color: badge_color,
            background: badge_bg,
            ..Default::default()
        },
    );
    job.append(
        &format!("  {short_name}"),
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(12.5),
            color: if is_selected {
                Color32::WHITE
            } else {
                Color32::from_rgb(238, 226, 246)
            },
            ..Default::default()
        },
    );
    job
}

