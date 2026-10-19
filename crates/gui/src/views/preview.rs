use eframe::egui::{self, pos2, vec2, Color32, Rect};
use skinforge_core::categorize::ComponentSlot;
use skinforge_core::mixer::{LaunchTarget, SlotSource};

use crate::audio::AudioPlayer;
use crate::state::{truncate_text, AppState, HitJudgementKind};
use crate::textures::{resolve_slot_file_path, TextureCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAction {
    None,
    ExportOsk,
    PlayHitSound(ComponentSlot),
}

pub fn render_preview(
    ui: &mut egui::Ui,
    state: &mut AppState,
    textures: &mut TextureCache,
    audio: &AudioPlayer,
) -> PreviewAction {
    let mut action = PreviewAction::None;
    let panel_width = ui.available_width();
    ui.set_max_width(panel_width);

    egui::ScrollArea::vertical()
        .id_salt("preview_panel_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(panel_width);
            ui.vertical(|ui| {
                ui.heading(
                    egui::RichText::new(format!(
                        "{} Preview & Export",
                        egui_phosphor::regular::EYE
                    ))
                    .color(Color32::from_rgb(255, 130, 188)),
                );
                ui.add_space(6.0);

                // 1. Interactive playfield canvas (Hitcircle + ApproachCircle + Cursor + Natural Hit Judgements on click)
                if let Some(act) = render_playfield_canvas(ui, state, textures) {
                    action = act;
                }

                ui.add_space(6.0);

                // 2. Compact Selected Element Inspector (previews non-playfield elements on hover/click)
                render_element_inspector(ui, state, textures);

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // 3. Mix summary
                render_mix_summary(ui, state);

                ui.add_space(8.0);

                // 4. Game version selector for auto-opening the exported .osk file
                ui.label(
                    egui::RichText::new(format!(
                        "{} Auto-open exported .osk in:",
                        egui_phosphor::regular::ROCKET_LAUNCH
                    ))
                    .strong()
                    .size(12.0)
                    .color(Color32::from_rgb(245, 220, 252)),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    for (target, label, tooltip) in [
                        (
                            LaunchTarget::Lazer,
                            "osu!lazer",
                            "Automatically launch the exported .osk package in osu!lazer",
                        ),
                        (
                            LaunchTarget::Stable,
                            "osu!stable",
                            "Automatically open the exported .osk package in osu!stable (uses Wine automatically on Linux)",
                        ),
                        (
                            LaunchTarget::None,
                            "Save only",
                            "Save the .osk file to disk without launching osu!",
                        ),
                    ] {
                        let active = state.launch_target == target;
                        let (bg_fill, stroke, text_color) = if active {
                            (
                                Color32::from_rgb(175, 42, 108),
                                egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 130, 190)),
                                Color32::WHITE,
                            )
                        } else {
                            (
                                Color32::from_rgb(24, 17, 30),
                                egui::Stroke::new(1.0_f32, Color32::from_rgb(56, 38, 68)),
                                Color32::from_rgb(165, 148, 180),
                            )
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new(label)
                                .size(12.0)
                                .strong()
                                .color(text_color),
                        )
                        .fill(bg_fill)
                        .stroke(stroke)
                        .rounding(6.0);

                        if ui.add(btn).on_hover_text(tooltip).clicked() {
                            state.launch_target = target;
                        }
                    }
                });

                ui.add_space(8.0);

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
                    .fill(Color32::from_rgb(228, 64, 142))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 138, 195)))
                    .rounding(8.0)
                    .min_size(vec2(ui.available_width(), 42.0));

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

    if state.show_texture_viewer {
        render_texture_viewer_window(ui.ctx(), state, textures);
    }

    match action {
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
    let canvas_size = vec2(ui.available_width().max(240.0), 210.0);
    let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
    let rect = response.rect;

    // Canvas background (deep osu! plum playfield)
    painter.rect_filled(rect, 8.0, Color32::from_rgb(16, 11, 21));
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 44, 84)),
    );

    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

    let center = rect.center() + vec2(-22.0, 2.0);
    let circle_radius = 42.0;

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
                Color32::from_rgba_unmultiplied(255, 195, 225, alpha),
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

            // Pop-in scale (1.28 -> 1.0) and slight upward drift
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
                        Color32::from_rgba_unmultiplied(255, 165, 210, (alpha as f32 * 0.45) as u8),
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
        painter.circle_filled(cursor_pos, 14.0, Color32::from_rgb(255, 102, 171));
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

fn render_element_inspector(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    let slot = state.inspected_slot;
    let source = state.slot_sources.get(&slot).unwrap_or(&SlotSource::Base);

    let source_job = match source {
        SlotSource::Base => state
            .base_skin()
            .map(|s| {
                crate::views::sidebar::build_skin_badged_job(
                    s.source,
                    &truncate_text(&s.name, 18),
                    false,
                )
            })
            .unwrap_or_else(|| {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    "Base skin",
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(11.0),
                        color: Color32::from_rgb(255, 130, 188),
                        ..Default::default()
                    },
                );
                job
            }),
        SlotSource::None => {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                "Disabled (None)",
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(11.0),
                    color: Color32::from_rgb(255, 185, 75),
                    ..Default::default()
                },
            );
            job
        }
        SlotSource::CustomFile(p) => {
            let fname = p.file_name().and_then(|f| f.to_str()).unwrap_or("custom");
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &format!("Custom: {}", truncate_text(fname, 20)),
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(11.0),
                    color: Color32::from_rgb(125, 215, 255),
                    ..Default::default()
                },
            );
            job
        }
        SlotSource::Donor(id) => state
            .installed_skins
            .iter()
            .find(|s| &s.id == id)
            .map(|s| {
                crate::views::sidebar::build_skin_badged_job(
                    s.source,
                    &truncate_text(&s.name, 18),
                    false,
                )
            })
            .unwrap_or_else(|| {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    "Donor skin",
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(11.0),
                        color: Color32::from_rgb(255, 130, 188),
                        ..Default::default()
                    },
                );
                job
            }),
    };

    let opt_tex = textures.get_or_load_slot(ui.ctx(), state, slot);
    let has_texture = opt_tex.is_some();
    let dims_text = if let Some(tex) = opt_tex {
        let tex_size = tex.size_vec2();
        format!("{}×{} px", tex_size.x as u32, tex_size.y as u32)
    } else {
        "No texture in skin".to_string()
    };

    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(29, 20, 36))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 44, 84)))
        .rounding(7.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (box_rect, thumb_resp) =
                    ui.allocate_exact_size(vec2(52.0, 52.0), egui::Sense::click());
                let hovered = thumb_resp.hovered();

                let (bg_color, border_stroke) = if hovered && has_texture {
                    (
                        Color32::from_rgb(24, 15, 32),
                        egui::Stroke::new(1.5_f32, Color32::from_rgb(255, 102, 171)),
                    )
                } else {
                    (
                        Color32::from_rgb(16, 11, 21),
                        egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 44, 84)),
                    )
                };

                ui.painter().rect_filled(box_rect, 6.0, bg_color);
                ui.painter().rect_stroke(box_rect, 6.0, border_stroke);

                if let Some(tex) = opt_tex {
                    let tex_size = tex.size_vec2();
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

                    if hovered {
                        let zoom_rect = Rect::from_min_size(
                            pos2(box_rect.max.x - 17.0, box_rect.max.y - 17.0),
                            vec2(15.0, 15.0),
                        );
                        ui.painter().rect_filled(
                            zoom_rect,
                            3.0,
                            Color32::from_rgba_unmultiplied(20, 12, 26, 220),
                        );
                        ui.painter().text(
                            zoom_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::MAGNIFYING_GLASS_PLUS,
                            egui::FontId::proportional(10.0),
                            Color32::from_rgb(255, 130, 188),
                        );
                    }
                } else {
                    ui.painter().text(
                        box_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::IMAGE,
                        egui::FontId::proportional(18.0),
                        Color32::from_rgb(115, 95, 130),
                    );
                }

                if has_texture {
                    if thumb_resp.clicked() {
                        state.show_texture_viewer = true;
                    }
                    thumb_resp.on_hover_text(format!(
                        "Click to view {} in full native resolution ({})",
                        slot.primary_filename(),
                        dims_text
                    ));
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
                        .size(12.5)
                        .color(Color32::from_rgb(250, 228, 255)),
                    );

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}  •  {}",
                                slot.primary_filename(),
                                dims_text
                            ))
                            .size(11.0)
                            .color(Color32::from_rgb(180, 160, 198)),
                        );

                        if has_texture
                            && ui
                                .small_button(format!(
                                    "{} View",
                                    egui_phosphor::regular::MAGNIFYING_GLASS_PLUS
                                ))
                                .on_hover_text("Open full resolution texture viewer")
                                .clicked()
                        {
                            state.show_texture_viewer = true;
                        }
                    });

                    ui.label(source_job);
                });
            });
        });
}

fn render_texture_viewer_window(
    ctx: &egui::Context,
    state: &mut AppState,
    textures: &mut TextureCache,
) {
    let mut is_open = state.show_texture_viewer;
    let slot = state.inspected_slot;
    let title = format!(
        "{} {} ({}) — Texture Viewer",
        egui_phosphor::regular::MAGNIFYING_GLASS_PLUS,
        slot.display_name(),
        slot.primary_filename()
    );

    egui::Window::new(title)
        .open(&mut is_open)
        .default_size(vec2(520.0, 480.0))
        .min_size(vec2(340.0, 300.0))
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            let opt_tex = textures.get_or_load_slot(ctx, state, slot);
            let Some(tex) = opt_tex else {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(
                        egui::RichText::new(egui_phosphor::regular::IMAGE)
                            .size(36.0)
                            .color(Color32::from_rgb(160, 140, 175)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "No texture found for slot \"{}\" in the current skin.",
                            slot.display_name()
                        ))
                        .color(Color32::from_rgb(215, 195, 225)),
                    );
                });
                return;
            };

            let tex_size = tex.size_vec2();
            let native_w = tex_size.x;
            let native_h = tex_size.y;

            // Top control bar
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Size: {}×{} px",
                        native_w as u32, native_h as u32
                    ))
                    .strong()
                    .color(Color32::from_rgb(250, 228, 255)),
                );

                ui.separator();

                let is_fit = state.texture_viewer_scale <= 0.001;
                if ui
                    .selectable_label(is_fit, "Fit")
                    .on_hover_text("Fit texture inside the viewer window")
                    .clicked()
                {
                    state.texture_viewer_scale = 0.0;
                }

                let is_100 = (state.texture_viewer_scale - 1.0).abs() < 0.01;
                if ui
                    .selectable_label(is_100, "100% (1:1)")
                    .on_hover_text("Show at exact 1:1 native resolution")
                    .clicked()
                {
                    state.texture_viewer_scale = 1.0;
                }

                let is_200 = (state.texture_viewer_scale - 2.0).abs() < 0.01;
                if ui
                    .selectable_label(is_200, "200%")
                    .on_hover_text("Zoom in 2x")
                    .clicked()
                {
                    state.texture_viewer_scale = 2.0;
                }

                let mut zoom_pct = if state.texture_viewer_scale <= 0.001 {
                    100.0
                } else {
                    (state.texture_viewer_scale * 100.0).round()
                };
                let slider = egui::Slider::new(&mut zoom_pct, 20.0..=400.0)
                    .suffix("%")
                    .show_value(true);
                if ui
                    .add_sized([95.0, 18.0], slider)
                    .on_hover_text("Adjust zoom scale")
                    .changed()
                {
                    state.texture_viewer_scale = zoom_pct / 100.0;
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Compute draw dimensions
            let avail_size = ui.available_size();
            let draw_size = if state.texture_viewer_scale <= 0.001 {
                let padding = 16.0;
                let max_w = (avail_size.x - padding).max(48.0);
                let max_h = (avail_size.y - padding).max(48.0);
                let scale_w = max_w / native_w;
                let scale_h = max_h / native_h;
                let fit_scale = scale_w.min(scale_h);
                tex_size * fit_scale
            } else {
                tex_size * state.texture_viewer_scale
            };

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        let (rect, _) = ui.allocate_exact_size(draw_size, egui::Sense::hover());
                        if ui.is_rect_visible(rect) {
                            let painter = ui.painter();

                            // Checkerboard transparency background
                            let check = 16.0;
                            let color_a = Color32::from_rgb(25, 17, 33);
                            let color_b = Color32::from_rgb(33, 23, 43);
                            let cols = (rect.width() / check).ceil() as usize;
                            let rows = (rect.height() / check).ceil() as usize;
                            for r in 0..rows {
                                for c in 0..cols {
                                    let col = if (r + c) % 2 == 0 { color_a } else { color_b };
                                    let cell_rect = Rect::from_min_max(
                                        pos2(
                                            rect.min.x + c as f32 * check,
                                            rect.min.y + r as f32 * check,
                                        ),
                                        pos2(
                                            (rect.min.x + (c + 1) as f32 * check).min(rect.max.x),
                                            (rect.min.y + (r + 1) as f32 * check).min(rect.max.y),
                                        ),
                                    );
                                    painter.rect_filled(cell_rect, 0.0, col);
                                }
                            }

                            // Full-res texture
                            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                            painter.image(tex.id(), rect, uv, Color32::WHITE);
                            painter.rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 44, 84)),
                            );
                        }
                    });
                });
        });

    state.show_texture_viewer = is_open;
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

    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(29, 20, 36))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 44, 84)))
        .rounding(7.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} Mix Summary ({} slots):",
                        egui_phosphor::regular::CHART_PIE,
                        ComponentSlot::all().len()
                    ))
                    .strong()
                    .size(12.5)
                    .color(Color32::from_rgb(250, 228, 255)),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(format!("{} Base:", egui_phosphor::regular::FOLDER));
                    ui.colored_label(Color32::from_rgb(210, 195, 225), format!("{base_count}"));
                    ui.separator();
                    ui.label(format!("{} Replaced:", egui_phosphor::regular::SWAP));
                    ui.colored_label(Color32::from_rgb(255, 130, 188), format!("{donor_count}"));
                    ui.separator();
                    ui.label(format!("{} Disabled:", egui_phosphor::regular::PROHIBIT));
                    ui.colored_label(Color32::from_rgb(255, 185, 75), format!("{none_count}"));
                });
            });
        });
}
