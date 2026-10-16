use eframe::egui::{self, pos2, vec2, Color32, Rect};
use skinforge_core::categorize::ComponentSlot;
use skinforge_core::mixer::SlotSource;

use crate::audio::AudioPlayer;
use crate::state::AppState;
use crate::textures::{resolve_slot_file_path, resolve_slot_texture_hash, TextureCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAction {
    None,
    ApplyMix,
    PlaySlotSound(ComponentSlot),
}

pub fn render_preview(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
) -> PreviewAction {
    let mut action = PreviewAction::None;

    ui.vertical(|ui| {
        ui.heading(format!(
            "{} 3. Preview & Build",
            egui_phosphor::regular::EYE
        ));
        ui.add_space(8.0);

        // 1. Interactive preview canvas
        render_playfield_canvas(ui, state, textures);

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // 2. Hitsound preview
        ui.label(
            egui::RichText::new(format!(
                "{} Hitsound preview:",
                egui_phosphor::regular::SPEAKER_SIMPLE_HIGH
            ))
            .strong(),
        );
        ui.add_space(4.0);

        ui.horizontal_wrapped(|ui| {
            if ui
                .button(format!("{} Normal", egui_phosphor::regular::PLAY))
                .clicked()
            {
                action = PreviewAction::PlaySlotSound(ComponentSlot::Hitnormal);
            }
            if ui
                .button(format!("{} Whistle", egui_phosphor::regular::PLAY))
                .clicked()
            {
                action = PreviewAction::PlaySlotSound(ComponentSlot::Hitwhistle);
            }
            if ui
                .button(format!("{} Finish", egui_phosphor::regular::PLAY))
                .clicked()
            {
                action = PreviewAction::PlaySlotSound(ComponentSlot::Hitfinish);
            }
            if ui
                .button(format!("{} Clap", egui_phosphor::regular::PLAY))
                .clicked()
            {
                action = PreviewAction::PlaySlotSound(ComponentSlot::Hitclap);
            }
            if ui
                .button(format!("{} Break", egui_phosphor::regular::PLAY))
                .clicked()
            {
                action = PreviewAction::PlaySlotSound(ComponentSlot::Combobreak);
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        // 3. Mix summary
        render_mix_summary(ui, state);

        ui.add_space(12.0);

        // 4. Options
        ui.checkbox(
            &mut state.set_active_on_build,
            "Set as active skin in osu!lazer",
        );
        ui.add_space(8.0);

        // 5. Build button
        let can_apply = !state.is_working && !state.is_game_running && state.base_skin().is_some();

        ui.add_enabled_ui(can_apply, |ui| {
            let button_text = format!(
                "{} Build skin into osu!lazer",
                egui_phosphor::regular::HAMMER
            );
            let btn = egui::Button::new(egui::RichText::new(button_text).size(15.0).strong())
                .min_size(vec2(ui.available_width(), 42.0));

            if ui.add(btn).clicked() {
                action = PreviewAction::ApplyMix;
            }
        });

        if state.is_game_running {
            ui.add_space(4.0);
            ui.colored_label(
                Color32::from_rgb(255, 100, 100),
                "Close osu!lazer before writing to database",
            );
        }
    });

    if let PreviewAction::PlaySlotSound(slot) = action {
        if let Some(path) = resolve_slot_file_path(state, slot) {
            audio.play_from_file(&path);
        }
        return PreviewAction::None;
    }

    action
}

fn render_playfield_canvas(ui: &mut egui::Ui, state: &AppState, textures: &mut TextureCache) {
    let canvas_size = vec2(ui.available_width().max(260.0), 220.0);
    let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::hover());
    let rect = response.rect;

    // Canvas background
    painter.rect_filled(rect, 8.0, Color32::from_rgb(18, 18, 26));
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 62)),
    );

    let center = rect.center() - vec2(30.0, 0.0);
    let circle_radius = 42.0;

    let time = ui.input(|i| i.time);
    let cycle = (time % 1.2) as f32 / 1.2;
    let scale = 2.4 - cycle * 1.4;
    let alpha = (cycle * 220.0 + 35.0).min(255.0) as u8;
    let approach_radius = circle_radius * scale;

    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

    // 1. Approach Circle
    let approach_hash = resolve_slot_texture_hash(state, ComponentSlot::ApproachCircle);
    if let Some(tex) =
        approach_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(center, vec2(approach_radius * 2.0, approach_radius * 2.0)),
            uv,
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
    } else {
        painter.circle_stroke(
            center,
            approach_radius,
            egui::Stroke::new(
                2.5_f32,
                Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
            ),
        );
    }

    // 2. Hitcircle
    let combo_color = state
        .combo_colours
        .first()
        .copied()
        .unwrap_or((255, 102, 171));
    let combo_tint = Color32::from_rgb(combo_color.0, combo_color.1, combo_color.2);
    let hitcircle_hash = resolve_slot_texture_hash(state, ComponentSlot::Hitcircle);
    if let Some(tex) =
        hitcircle_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(center, vec2(circle_radius * 2.0, circle_radius * 2.0)),
            uv,
            combo_tint,
        );
    } else {
        painter.circle_filled(center, circle_radius, combo_tint);
    }

    // 3. Hitcircle Overlay
    let overlay_hash = resolve_slot_texture_hash(state, ComponentSlot::HitcircleOverlay);
    if let Some(tex) =
        overlay_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(center, vec2(circle_radius * 2.0, circle_radius * 2.0)),
            uv,
            Color32::WHITE,
        );
    } else {
        painter.circle_stroke(
            center,
            circle_radius,
            egui::Stroke::new(3.0_f32, Color32::WHITE),
        );
    }

    // 4. Default number "1"
    let num_hash = find_default_1_hash(state);
    if let Some(tex) =
        num_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(center, vec2(20.0, 32.0)),
            uv,
            Color32::WHITE,
        );
    } else {
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "1",
            egui::FontId::proportional(28.0),
            Color32::WHITE,
        );
    }

    // 5. Cursor
    let cursor_pos = if let Some(hover) = response.hover_pos() {
        hover
    } else {
        center
            + vec2(
                70.0 + (time * 1.6).cos() as f32 * 35.0,
                (time * 1.6).sin() as f32 * 35.0,
            )
    };

    // Cursor Trail
    let trail_hash = resolve_slot_texture_hash(state, ComponentSlot::CursorTrail);
    if let Some(tex) =
        trail_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(cursor_pos - vec2(8.0, 8.0), vec2(24.0, 24.0)),
            uv,
            Color32::from_rgba_unmultiplied(255, 255, 255, 140),
        );
    } else {
        painter.circle_filled(
            cursor_pos - vec2(6.0, 6.0),
            10.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 60),
        );
    }

    // Cursor Body
    let cursor_hash = resolve_slot_texture_hash(state, ComponentSlot::Cursor);
    if let Some(tex) =
        cursor_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(cursor_pos, vec2(36.0, 36.0)),
            uv,
            Color32::WHITE,
        );
    } else {
        painter.circle_filled(cursor_pos, 14.0, Color32::from_rgb(255, 190, 40));
        painter.circle_stroke(cursor_pos, 14.0, egui::Stroke::new(2.0_f32, Color32::WHITE));
    }

    // Cursor Middle (if not None)
    let middle_hash = resolve_slot_texture_hash(state, ComponentSlot::CursorMiddle);
    if let Some(tex) =
        middle_hash.and_then(|h| textures.get_or_load(ui.ctx(), state.files_dir.as_deref(), h))
    {
        painter.image(
            tex.id(),
            Rect::from_center_size(cursor_pos, vec2(10.0, 10.0)),
            uv,
            Color32::WHITE,
        );
    }

    ui.ctx().request_repaint();
}

fn render_mix_summary(ui: &mut egui::Ui, state: &AppState) {
    let mut base_count = 0;
    let mut donor_count = 0;
    let mut none_count = 0;

    for slot in ComponentSlot::all() {
        match state.slot_sources.get(slot).unwrap_or(&SlotSource::Base) {
            SlotSource::Base => base_count += 1,
            SlotSource::Donor(_) | SlotSource::CustomFile(_) => donor_count += 1,
            SlotSource::None => none_count += 1,
        }
    }

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} Mix Summary:",
                    egui_phosphor::regular::CHART_PIE
                ))
                .strong(),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(format!("{} From Base:", egui_phosphor::regular::FOLDER));
                ui.colored_label(Color32::from_rgb(180, 180, 200), format!("{base_count}"));
            });

            ui.horizontal(|ui| {
                ui.label(format!("{} From Donors:", egui_phosphor::regular::SWAP));
                ui.colored_label(Color32::from_rgb(56, 239, 125), format!("{donor_count}"));
            });

            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} Disabled (None):",
                    egui_phosphor::regular::PROHIBIT
                ));
                ui.colored_label(Color32::from_rgb(255, 180, 50), format!("{none_count}"));
            });
        });
    });
}

fn find_default_1_hash(state: &AppState) -> Option<&str> {
    let base = state.base_skin()?;
    base.files
        .iter()
        .find(|f| {
            f.filename.eq_ignore_ascii_case("default-1@2x.png")
                || f.filename.eq_ignore_ascii_case("default-1.png")
        })
        .map(|f| f.file_hash.as_str())
}
