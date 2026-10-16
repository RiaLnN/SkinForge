use std::collections::HashMap;
use std::path::Path;
use eframe::egui;
use osu_mixer_core::categorize::ComponentSlot;
use osu_mixer_core::mixer::SlotSource;

use crate::state::AppState;

pub struct TextureCache {
    cache: HashMap<String, egui::TextureHandle>,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_or_load(
        &mut self,
        ctx: &egui::Context,
        files_dir: Option<&Path>,
        hash: &str,
    ) -> Option<&egui::TextureHandle> {
        if !self.cache.contains_key(hash) {
            let files_dir = files_dir?;
            let path = osu_mixer_core::paths::hashed_storage_path(files_dir, hash);
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                    let handle = ctx.load_texture(
                        format!("tex_{hash}"),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.cache.insert(hash.to_string(), handle);
                }
            }
        }
        self.cache.get(hash)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Определяет hash файла для отображения указанного слота с учетом выбранного источника
/// и автоматического отката на базу (fallback).
pub fn resolve_slot_file_hash<'a>(
    state: &'a AppState,
    slot: ComponentSlot,
) -> Option<&'a str> {
    let source = state.slot_sources.get(&slot).unwrap_or(&SlotSource::Base);
    match source {
        SlotSource::None => None,
        SlotSource::Base => find_file_hash_for_slot(state.base_skin()?, slot),
        SlotSource::Donor(donor_id) => {
            let donor_skin = state.installed_skins.iter().find(|s| &s.id == donor_id);
            if let Some(donor) = donor_skin {
                if let Some(hash) = find_file_hash_for_slot(donor, slot) {
                    return Some(hash);
                }
            }
            // Fallback на базовый скин, как в osu! lazer
            find_file_hash_for_slot(state.base_skin()?, slot)
        }
    }
}

pub fn resolve_slot_texture_hash<'a>(
    state: &'a AppState,
    slot: ComponentSlot,
) -> Option<&'a str> {
    resolve_slot_file_hash(state, slot)
}

/// Возвращает абсолютный путь к файлу ресурса в хранилище osu! lazer.
pub fn resolve_slot_file_path(
    state: &AppState,
    slot: ComponentSlot,
) -> Option<std::path::PathBuf> {
    let hash = resolve_slot_file_hash(state, slot)?;
    let files_dir = state.files_dir.as_ref()?;
    Some(osu_mixer_core::paths::hashed_storage_path(files_dir, hash))
}

fn find_file_hash_for_slot(skin: &osu_mixer_core::bridge::Skin, slot: ComponentSlot) -> Option<&str> {
    let target_name = slot.primary_filename();
    let name_no_ext = target_name.strip_suffix(".png").unwrap_or(target_name);
    let target_2x = format!("{name_no_ext}@2x.png");

    // Ищем сначала @2x, затем 1x, либо любой совпавший по категории файл
    if let Some(f) = skin
        .files
        .iter()
        .find(|f| f.filename.eq_ignore_ascii_case(&target_2x))
    {
        return Some(&f.file_hash);
    }

    if let Some(f) = skin
        .files
        .iter()
        .find(|f| f.filename.eq_ignore_ascii_case(target_name))
    {
        return Some(&f.file_hash);
    }

    // Дополнительный поиск по matcher'у слота
    skin.files
        .iter()
        .find(|f| slot.matches_filename(&f.filename))
        .map(|f| f.file_hash.as_str())
}
