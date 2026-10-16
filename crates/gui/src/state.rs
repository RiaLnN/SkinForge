use std::collections::HashMap;
use std::path::PathBuf;

use skinforge_core::bridge::Skin;
use skinforge_core::categorize::{ComponentCategory, ComponentSlot};
use skinforge_core::mixer::{ManifestMixPlan, SlotAssignment, SlotSource};

use skinforge_core::skin_ini::SkinIni;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Category(ComponentCategory),
    Colors,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub lazer_root: Option<PathBuf>,
    pub files_dir: Option<PathBuf>,
    pub client_realm_path: Option<PathBuf>,
    pub installed_skins: Vec<Skin>,
    pub selected_base_skin_id: Option<String>,
    pub new_skin_name: String,
    pub new_skin_creator: String,
    pub slot_sources: HashMap<ComponentSlot, SlotSource>,
    pub combo_colours: Vec<(u8, u8, u8)>,
    pub auto_backup: bool,
    pub active_tab: ActiveTab,
    pub status_message: Option<(String, bool)>,
    pub is_working: bool,
    pub is_game_running: bool,
    pub set_active_on_build: bool,
    pub show_advanced_cursor: bool,
    pub show_advanced_gameplay: bool,
    pub show_advanced_colors: bool,
    pub extended_ini: SkinIni,
}

impl Default for AppState {
    fn default() -> Self {
        let mut slot_sources = HashMap::new();
        for slot in ComponentSlot::all() {
            slot_sources.insert(*slot, SlotSource::Base);
        }

        Self {
            lazer_root: None,
            files_dir: None,
            client_realm_path: None,
            installed_skins: Vec::new(),
            selected_base_skin_id: None,
            new_skin_name: "Mixed Skin".to_string(),
            new_skin_creator: "SkinForge".to_string(),
            slot_sources,
            combo_colours: vec![
                (255, 102, 171), // #ff66ab
                (51, 187, 238),  // #33bbee
                (136, 255, 68),  // #88ff44
                (255, 170, 34),  // #ffaa22
            ],
            auto_backup: true,
            active_tab: ActiveTab::Category(ComponentCategory::Cursor),
            status_message: None,
            is_working: false,
            is_game_running: false,
            set_active_on_build: true,
            show_advanced_cursor: false,
            show_advanced_gameplay: false,
            show_advanced_colors: false,
            extended_ini: SkinIni::default(),
        }
    }
}

impl AppState {
    /// Loads skin list from database, filtering out built-in protected skins
    /// (Protected == true) and empty skins with zero files.
    pub fn set_installed_skins(&mut self, skins: Vec<Skin>) {
        self.installed_skins = skins
            .into_iter()
            .filter(|s| !s.is_protected && !s.files.is_empty())
            .collect();

        if self.selected_base_skin_id.is_none() && !self.installed_skins.is_empty() {
            self.select_base_skin(&self.installed_skins[0].id.clone());
        }
    }

    /// Selects a base skin and sets the default new skin name.
    pub fn select_base_skin(&mut self, skin_id: &str) {
        if let Some(skin) = self.installed_skins.iter().find(|s| s.id == skin_id) {
            self.selected_base_skin_id = Some(skin.id.clone());
            self.new_skin_name = format!("{} + Mixed", skin.name);

            // Attempt to load base skin's skin.ini into extended_ini
            if let Some(files_dir) = &self.files_dir {
                if let Some(ini_file) = skin
                    .files
                    .iter()
                    .find(|f| f.filename.eq_ignore_ascii_case("skin.ini"))
                {
                    let path =
                        skinforge_core::paths::hashed_storage_path(files_dir, &ini_file.file_hash);
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        self.extended_ini = SkinIni::parse(&text);
                        let combos = self.extended_ini.combo_colours();
                        if !combos.is_empty() {
                            self.combo_colours = combos.into_iter().map(|(_, rgb)| rgb).collect();
                        }
                    }
                }
            }
        }
    }

    pub fn set_slot_custom_file(&mut self, slot: ComponentSlot, path: PathBuf) {
        self.slot_sources.insert(slot, SlotSource::CustomFile(path));
    }

    /// Returns a reference to the currently selected base skin, if any.
    pub fn base_skin(&self) -> Option<&Skin> {
        let id = self.selected_base_skin_id.as_ref()?;
        self.installed_skins.iter().find(|s| &s.id == id)
    }

    /// Sets the source for an individual component slot.
    pub fn set_slot_source(&mut self, slot: ComponentSlot, source: SlotSource) {
        self.slot_sources.insert(slot, source);
    }

    /// Batch-assigns all slots in a category to a donor skin.
    pub fn batch_set_category(&mut self, category: ComponentCategory, donor_id: &str) {
        for slot in ComponentSlot::all() {
            if slot.category() == category {
                self.slot_sources
                    .insert(*slot, SlotSource::Donor(donor_id.to_string()));
            }
        }
    }

    /// Resets all slots in a category to the base skin.
    pub fn reset_category_to_base(&mut self, category: ComponentCategory) {
        for slot in ComponentSlot::all() {
            if slot.category() == category {
                self.slot_sources.insert(*slot, SlotSource::Base);
            }
        }
    }

    /// Assembles a `ManifestMixPlan` from the current UI configuration.
    pub fn build_mix_plan(&self, custom_skin_ini_hash: Option<String>) -> Option<ManifestMixPlan> {
        let base_id = self.selected_base_skin_id.clone()?;
        let slots = self
            .slot_sources
            .iter()
            .map(|(slot, source)| SlotAssignment {
                slot: *slot,
                source: source.clone(),
            })
            .collect();

        Some(ManifestMixPlan {
            base_skin_id: base_id,
            new_skin_name: self.new_skin_name.clone(),
            new_skin_creator: self.new_skin_creator.clone(),
            slots,
            custom_skin_ini_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skinforge_core::bridge::SkinFile;

    fn make_skin(id: &str, name: &str, is_protected: bool, files: &[&str]) -> Skin {
        Skin {
            id: id.to_string(),
            name: name.to_string(),
            creator: "creator".to_string(),
            is_protected,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: files
                .iter()
                .map(|f| SkinFile {
                    filename: f.to_string(),
                    file_hash: "hash".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn filtering_skins_excludes_protected_and_empty() {
        let mut state = AppState::default();
        let s1 = make_skin("1", "Custom", false, &["cursor.png"]);
        let s2 = make_skin("2", "Argon (Protected)", true, &["cursor.png"]);
        let s3 = make_skin("3", "Empty", false, &[]);

        state.set_installed_skins(vec![s1, s2, s3]);
        assert_eq!(state.installed_skins.len(), 1);
        assert_eq!(state.installed_skins[0].name, "Custom");
        assert_eq!(state.selected_base_skin_id.as_deref(), Some("1"));
    }

    #[test]
    fn changing_base_skin_updates_default_slots_and_name() {
        let mut state = AppState::default();
        let s1 = make_skin("1", "Aristia", false, &["cursor.png"]);
        let s2 = make_skin("2", "WhiteCat", false, &["cursor.png"]);

        state.set_installed_skins(vec![s1, s2]);
        state.select_base_skin("2");

        assert_eq!(state.selected_base_skin_id.as_deref(), Some("2"));
        assert_eq!(state.new_skin_name, "WhiteCat + Mixed");
        assert_eq!(state.base_skin().unwrap().name, "WhiteCat");
    }

    #[test]
    fn batch_category_assignment_updates_all_slots_in_category() {
        let mut state = AppState::default();
        state.batch_set_category(ComponentCategory::Cursor, "donor_cat");

        assert_eq!(
            state.slot_sources.get(&ComponentSlot::Cursor),
            Some(&SlotSource::Donor("donor_cat".to_string()))
        );
        assert_eq!(
            state.slot_sources.get(&ComponentSlot::CursorTrail),
            Some(&SlotSource::Donor("donor_cat".to_string()))
        );
        // Gameplay unaffected
        assert_eq!(
            state.slot_sources.get(&ComponentSlot::Hitcircle),
            Some(&SlotSource::Base)
        );
    }

    #[test]
    fn custom_file_assignment_updates_slot_sources() {
        let mut state = AppState::default();
        let file = PathBuf::from("/tmp/custom_cursor.png");
        state.set_slot_custom_file(ComponentSlot::Cursor, file.clone());
        assert_eq!(
            state.slot_sources.get(&ComponentSlot::Cursor),
            Some(&SlotSource::CustomFile(file))
        );
    }
}

