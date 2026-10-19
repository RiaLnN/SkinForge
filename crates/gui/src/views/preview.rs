use eframe::egui::{self, pos2, vec2, Color32, Rect};
use skinforge_core::categorize::ComponentSlot;
use skinforge_core::mixer::SlotSource;

use crate::audio::AudioPlayer;
use crate::state::{truncate_text, AppState, HitJudgementKind};
use crate::textures::{resolve_slot_file_path, TextureCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAction {
    None,
    ExportOsk,
    ToggleSlotSound(ComponentSlot),
    PlayHitSound(ComponentSlot),
}

pub fn render_preview(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
) -> PreviewAction {
    let mut action = PreviewAction::None;

    egui::ScrollArea::vertical()
        .id_salt("preview_panel_scroll")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading(format!(
                    "{} 3. Preview & Export",
                    egui_phosphor::regular::EYE
                ));
                ui.add_space(6.0);

                // 1. Interactive playfield canvas (with Scorebar + Hit Judgements on click)
                if let Some(act) = render_playfield_canvas(ui, state, textures) {
                    action = act;
                }

                ui.add_space(6.0);

                // 2. Compact Selected Element Inspector (previews non-playfield elements on hover/click)
                render_element_inspector(ui, state, textures);

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // 3. Audio Controls (Volume + Stop + Quick Toggle Buttons)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} Audio Preview",
                            egui_phosphor::regular::SPEAKER_SIMPLE_HIGH
                        ))
                        .strong()
                        .size(12.5),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let playing = audio.is_playing();
                        ui.add_enabled_ui(playing, |ui| {
                            let stop_btn = egui::Button::new(
                                egui::RichText::new(format!(
                                    "{} Stop",
                                    egui_phosphor::regular::STOP
                                ))
                                .size(11.5)
                                .color(if playing {
                                    Color32::from_rgb(255, 110, 140)
                                } else {
                                    Color32::GRAY
                                }),
                            );
                            if ui
                                .add(stop_btn)
                                .on_hover_text("Stop currently playing audio immediately")
                                .clicked()
                            {
                                audio.stop();
                            }
                        });

                        let mut vol_pct = (audio.volume() * 100.0).round();
                        let slider = egui::Slider::new(&mut vol_pct, 0.0..=100.0)
                            .suffix("%")
                            .show_value(true);
                        if ui
                            .add_sized([110.0, 18.0], slider)
                            .on_hover_text("Audio preview volume")
                            .changed()
                        {
                            audio.set_volume(vol_pct / 100.0);
                        }
                    });
                });

                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    for (slot, label) in [
                        (ComponentSlot::Hitnormal, "Normal"),
                        (ComponentSlot::Hitwhistle, "Whistle"),
                        (ComponentSlot::Hitfinish, "Finish"),
                        (ComponentSlot::Hitclap, "Clap"),
                        (ComponentSlot::Combobreak, "Break"),
                        (ComponentSlot::SectionPass, "Pass"),
                        (ComponentSlot::Applause, "Applause"),
                        (ComponentSlot::MenuClick, "Click"),
                    ] {
                        let is_playing = audio.is_playing_slot(slot);
                        let icon = if is_playing {
                            egui_phosphor::regular::STOP
                        } else {
                            egui_phosphor::regular::PLAY
                        };
                        let text = if is_playing {
                            egui::RichText::new(format!("{icon} {label}"))
                                .color(Color32::from_rgb(255, 110, 140))
                        } else {
                            egui::RichText::new(format!("{icon} {label}"))
                        };
                        if ui
                            .button(text)
                            .on_hover_text(format!(
                                "{} {} ({})",
                                if is_playing { "Stop" } else { "Play" },
                                slot.display_name(),
                                slot.primary_filename()
                            ))
                            .clicked()
                        {
                            action = PreviewAction::ToggleSlotSound(slot);
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // 4. Mix summary
                render_mix_summary(ui, state);

                ui.add_space(8.0);

                // 5. Export options & primary Export .osk button
                ui.checkbox(
                    &mut state.open_after_export,
                    "Open exported .osk file in osu! automatically",
                );
                ui.add_space(6.0);

                let can_export = !state.is_working && state.base_skin().is_some();

                ui.add_enabled_ui(can_export, |ui| {
                    let button_text = format!(
                        "{} Export .osk Skin",
                        egui_phosphor::regular::DOWNLOAD_SIMPLE
                    );
                    let btn = egui::Button::new(
                        egui::RichText::new(button_text)
                            .size(15.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(95, 75, 220))
                    .min_size(vec2(ui.available_width(), 40.0));

                    if ui
                        .add(btn)
                        .on_hover_text(
                            "Package the mixed skin into a universal .osk file compatible with both osu!lazer and osu!stable",
                        )
                        .clicked()
                    {
                        action = PreviewAction::ExportOsk;
                    }
                });
            });
        });

    match action {
        PreviewAction::ToggleSlotSound(slot) => {
            let path = resolve_slot_file_path(state, slot);
            audio.toggle_slot(slot, path.as_deref());
            PreviewAction::None
        }
        PreviewAction::PlayHitSound(slot) => {
            let path = resolve_slot_file_path(state, slot);
            audio.play_slot(slot, path.as_deref());
            PreviewAction::None
        }
        other => other,
    }
}

fn render_playfield_canvas(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
) -> Option<PreviewAction> {
    let canvas_size = vec2(ui.available_width().max(260.0), 215.0);
    let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
    let rect = response.rect;

    // Canvas background
    painter.rect_filled(rect, 8.0, Color32::from_rgb(16, 16, 24));
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 62)),
    );

    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

    // Top HUD: Scorebar / Health Bar (`scorebar-bg`, `scorebar-colour`, `scorebar-ki`)
    let bar_rect = Rect::from_min_size(
        rect.min + vec2(12.0, 10.0),
        vec2((rect.width() * 0.58).min(210.0), 14.0),
    );
    if let Some(bg_tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::ScorebarBg) {
        painter.image(bg_tex.id(), bar_rect, uv, Color32::WHITE);
    } else {
        painter.rect_filled(bar_rect, 4.0, Color32::from_rgb(32, 32, 48));
    }

    let fill_rect = Rect::from_min_size(
        bar_rect.min + vec2(2.0, 2.0),
        vec2((bar_rect.width() - 4.0) * 0.82, bar_rect.height() - 4.0),
    );
    if let Some(col_tex) =
        textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::ScorebarColour)
    {
        painter.image(col_tex.id(), fill_rect, uv, Color32::WHITE);
    } else {
        painter.rect_filled(fill_rect, 3.0, Color32::from_rgb(86, 225, 145));
    }

    let ki_center = pos2(fill_rect.max.x, fill_rect.center().y);
    if let Some(ki_tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::ScorebarKi) {
        painter.image(
            ki_tex.id(),
            Rect::from_center_size(ki_center, vec2(18.0, 18.0)),
            uv,
            Color32::WHITE,
        );
    }

    let center = rect.center() + vec2(-24.0, 8.0);
    let circle_radius = 40.0;

    let time = ui.input(|i| i.time);

    let cycle = if let Some(hit_time) = state.last_hit_time {
        let elapsed = (time - hit_time).max(0.0);
        (elapsed % 1.2) as f32 / 1.2
    } else {
        (time % 1.2) as f32 / 1.2
    };

    // Click interaction: natural hit judgements based on timing or missing the circle
    let mut hit_action = None;
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let dist = (pos - center).length();
            let judgement = if dist <= circle_radius * 1.35 {
                state.record_hit_judgement(cycle, time)
            } else {
                state.last_hit_time = Some(time);
                state.last_hit_judgement = Some((HitJudgementKind::Miss, time));
                HitJudgementKind::Miss
            };

            let sound_slot = if judgement == HitJudgementKind::Miss {
                ComponentSlot::Combobreak
            } else {
                ComponentSlot::Hitnormal
            };
            hit_action = Some(PreviewAction::PlayHitSound(sound_slot));
        }
    }

    let scale = 2.4 - cycle * 1.4;
    let alpha = (cycle * 220.0 + 35.0).min(255.0) as u8;
    let approach_radius = circle_radius * scale;

    // 1. Approach Circle
    if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::ApproachCircle) {
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
    if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::Hitcircle) {
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
    if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::HitcircleOverlay) {
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

    // 4. Combo number "1" inside Hitcircle
    if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::ComboNumbers) {
        painter.image(
            tex.id(),
            Rect::from_center_size(center, vec2(22.0, 32.0)),
            uv,
            Color32::WHITE,
        );
    } else {
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "1",
            egui::FontId::proportional(26.0),
            Color32::WHITE,
        );
    }

    // 4b. Natural Hit Judgement Sprite Animation (300 / 100 / 50 / Miss) on Hit
    if let Some((judgement, hit_time)) = state.last_hit_judgement {
        let age = (time - hit_time) as f32;
        let duration = 0.48;
        if (0.0..duration).contains(&age) {
            let progress = age / duration;
            let burst_radius = circle_radius + progress * 26.0;
            let burst_alpha = ((1.0 - progress) * 240.0) as u8;
            let ring_color = judgement.fallback_color();
            painter.circle_stroke(
                center,
                burst_radius,
                egui::Stroke::new(
                    2.8 * (1.0 - progress),
                    Color32::from_rgba_unmultiplied(
                        ring_color.r(),
                        ring_color.g(),
                        ring_color.b(),
                        burst_alpha,
                    ),
                ),
            );

            // Pop-in scale (1.25 -> 1.0) and slight upward drift
            let pop_scale = if progress < 0.2 {
                1.0 + (1.0 - progress / 0.2) * 0.28
            } else {
                1.0
            };
            let judge_pos = center - vec2(0.0, 14.0 + progress * 24.0);
            let judge_slot = judgement.slot();

            if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, judge_slot) {
                let size = vec2(58.0, 32.0) * pop_scale;
                painter.image(
                    tex.id(),
                    Rect::from_center_size(judge_pos, size),
                    uv,
                    Color32::from_rgba_unmultiplied(255, 255, 255, burst_alpha),
                );
            } else {
                painter.text(
                    judge_pos,
                    egui::Align2::CENTER_CENTER,
                    judgement.fallback_text(),
                    egui::FontId::proportional(22.0 * pop_scale),
                    Color32::from_rgba_unmultiplied(
                        ring_color.r(),
                        ring_color.g(),
                        ring_color.b(),
                        burst_alpha,
                    ),
                );
            }
        }
    }

    // 5. Cursor & Trail
    let cursor_pos = response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos())
        .unwrap_or_else(|| {
            center
                + vec2(
                    68.0 + (time * 1.6).cos() as f32 * 32.0,
                    (time * 1.6).sin() as f32 * 32.0,
                )
        });

    let is_trail_none = matches!(
        state.slot_sources.get(&ComponentSlot::CursorTrail),
        Some(SlotSource::None)
    );

    if !is_trail_none {
        state.record_cursor_pos(cursor_pos, time);
        let trail_tex = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::CursorTrail);

        for &(trail_pos, t) in &state.cursor_trail_history {
            let age = ((time - t) / 0.25).clamp(0.0, 1.0) as f32;
            if age > 0.04 {
                let alpha = ((1.0 - age) * 160.0) as u8;
                let scale = 1.0 - age * 0.35;
                if let Some(tex) = trail_tex {
                    painter.image(
                        tex.id(),
                        Rect::from_center_size(trail_pos, vec2(24.0, 24.0) * scale),
                        uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                    );
                } else {
                    painter.circle_filled(
                        trail_pos,
                        9.0 * scale,
                        Color32::from_rgba_unmultiplied(255, 255, 255, (alpha as f32 * 0.4) as u8),
                    );
                }
            }
        }
    }

    // Cursor Body
    if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::Cursor) {
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

    // Cursor Middle
    if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, ComponentSlot::CursorMiddle) {
        painter.image(
            tex.id(),
            Rect::from_center_size(cursor_pos, vec2(10.0, 10.0)),
            uv,
            Color32::WHITE,
        );
    }

    response.on_hover_text(
        "Click the hitcircle as the approach circle closes in to test 300 / 100 / 50 / Miss judgements!",
    );

    ui.ctx().request_repaint();

    hit_action
}

fn render_element_inspector(
    ui: &mut egui::Ui,
    state: &AppState,
    textures: &mut TextureCache,
) {
    let slot = state.inspected_slot;
    let source = state.slot_sources.get(&slot).unwrap_or(&SlotSource::Base);

    let source_desc = match source {
        SlotSource::Base => state
            .base_skin()
            .map(|s| format!("Base: [{}] {}", s.badge_label(), truncate_text(&s.name, 22)))
            .unwrap_or_else(|| "Base skin".to_string()),
        SlotSource::None => "Disabled (None)".to_string(),
        SlotSource::CustomFile(p) => {
            let fname = p.file_name().and_then(|f| f.to_str()).unwrap_or("custom");
            format!("Custom: {}", truncate_text(fname, 22))
        }
        SlotSource::Donor(id) => state
            .installed_skins
            .iter()
            .find(|s| &s.id == id)
            .map(|s| format!("Donor: [{}] {}", s.badge_label(), truncate_text(&s.name, 22)))
            .unwrap_or_else(|| "Donor skin".to_string()),
    };

    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(20, 20, 30))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (box_rect, _) =
                    ui.allocate_exact_size(vec2(52.0, 52.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(box_rect, 6.0, Color32::from_rgb(12, 12, 18));
                ui.painter().rect_stroke(
                    box_rect,
                    6.0,
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(48, 48, 68)),
                );

                let mut dims_text = "No texture in skin".to_string();
                if let Some(tex) = textures.get_or_load_slot(ui.ctx(), state, slot) {
                    let tex_size = tex.size_vec2();
                    dims_text = format!("{}×{} px", tex_size.x as u32, tex_size.y as u32);
                    let max_dim = tex_size.x.max(tex_size.y).max(1.0);
                    let scale = (46.0 / max_dim).min(2.0);
                    let draw_size = (tex_size * scale).clamp(vec2(8.0, 8.0), vec2(46.0, 46.0));
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    ui.painter().image(
                        tex.id(),
                        Rect::from_center_size(box_rect.center(), draw_size),
                        uv,
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().text(
                        box_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::IMAGE,
                        egui::FontId::proportional(18.0),
                        Color32::from_rgb(90, 90, 115),
                    );
                }

                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} Inspector: {}",
                            egui_phosphor::regular::MAGNIFYING_GLASS_PLUS,
                            slot.display_name()
                        ))
                        .strong()
                        .size(12.5),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{}  •  {}",
                            slot.primary_filename(),
                            dims_text
                        ))
                        .size(11.0)
                        .color(Color32::from_rgb(150, 150, 175)),
                    );
                    ui.label(
                        egui::RichText::new(source_desc)
                            .size(11.0)
                            .color(Color32::from_rgb(162, 155, 254)),
                    );
                });
            });
        });
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
                    "{} Mix Summary ({} slots):",
                    egui_phosphor::regular::CHART_PIE,
                    ComponentSlot::all().len()
                ))
                .strong()
                .size(12.5),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(format!("{} Base:", egui_phosphor::regular::FOLDER));
                ui.colored_label(Color32::from_rgb(180, 180, 200), format!("{base_count}"));
                ui.separator();
                ui.label(format!("{} Replaced:", egui_phosphor::regular::SWAP));
                ui.colored_label(Color32::from_rgb(56, 239, 125), format!("{donor_count}"));
                ui.separator();
                ui.label(format!("{} Disabled:", egui_phosphor::regular::PROHIBIT));
                ui.colored_label(Color32::from_rgb(255, 180, 50), format!("{none_count}"));
            });
        });
    });
}
