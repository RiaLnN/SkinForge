use eframe::egui;
use skinforge_core::categorize::ComponentSlot;
use skinforge_core::mixer::SlotSource;
use std::collections::HashMap;
use std::path::Path;

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
            let path = skinforge_core::paths::hashed_storage_path(files_dir, hash);
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

    pub fn get_or_load_path(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
    ) -> Option<&egui::TextureHandle> {
        let key = path.to_string_lossy().to_string();
        if !self.cache.contains_key(&key) {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                    let handle = ctx.load_texture(
                        format!("tex_custom_{key}"),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.cache.insert(key.clone(), handle);
                }
            }
        }
        self.cache.get(&key)
    }

    pub fn get_or_load_slot(
        &mut self,
        ctx: &egui::Context,
        state: &AppState,
        slot: ComponentSlot,
    ) -> Option<&egui::TextureHandle> {
        let source = state.slot_sources.get(&slot).unwrap_or(&SlotSource::Base);
        if let SlotSource::CustomFile(path) = source {
            return self.get_or_load_path(ctx, path);
        }
        let hash = resolve_slot_file_hash(state, slot)?;
        self.get_or_load(ctx, state.files_dir.as_deref(), hash)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Resolves the file hash to display for a slot considering the selected source
/// and automatic fallback to base skin.
pub fn resolve_slot_file_hash(state: &AppState, slot: ComponentSlot) -> Option<&str> {
    let source = state.slot_sources.get(&slot).unwrap_or(&SlotSource::Base);
    match source {
        SlotSource::None => None,
        SlotSource::CustomFile(_) => None,
        SlotSource::Base => find_file_hash_for_slot(state.base_skin()?, slot),
        SlotSource::Donor(donor_id) => {
            let donor_skin = state.installed_skins.iter().find(|s| &s.id == donor_id);
            if let Some(donor) = donor_skin {
                if let Some(hash) = find_file_hash_for_slot(donor, slot) {
                    return Some(hash);
                }
            }
            // Fallback to base skin, matching osu!lazer behavior
            find_file_hash_for_slot(state.base_skin()?, slot)
        }
    }
}

pub fn resolve_slot_texture_hash(state: &AppState, slot: ComponentSlot) -> Option<&str> {
    resolve_slot_file_hash(state, slot)
}

/// Returns the absolute path to a resource file in osu!lazer content-addressed storage.
pub fn resolve_slot_file_path(state: &AppState, slot: ComponentSlot) -> Option<std::path::PathBuf> {
    let source = state.slot_sources.get(&slot).unwrap_or(&SlotSource::Base);
    if let SlotSource::CustomFile(path) = source {
        return Some(path.clone());
    }
    let hash = resolve_slot_file_hash(state, slot)?;
    let files_dir = state.files_dir.as_ref()?;
    Some(skinforge_core::paths::hashed_storage_path(files_dir, hash))
}

fn find_file_hash_for_slot(
    skin: &skinforge_core::bridge::Skin,
    slot: ComponentSlot,
) -> Option<&str> {
    let target_name = slot.primary_filename();
    let name_no_ext = target_name.strip_suffix(".png").unwrap_or(target_name);
    let target_2x = format!("{name_no_ext}@2x.png");

    // Look for @2x first, then 1x, then any matching file for slot
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

    skin.files
        .iter()
        .find(|f| slot.matches_filename(&f.filename))
        .map(|f| f.file_hash.as_str())
}
