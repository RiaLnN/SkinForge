use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;
use skinforge_core::bridge::Skin;
use skinforge_core::categorize::{ComponentCategory, ComponentSlot};
use skinforge_core::mixer::{ManifestMixPlan, SlotAssignment, SlotSource};

use skinforge_core::skin_ini::SkinIni;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Category(ComponentCategory),
    Colors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildMethod {
    #[default]
    OskPackage,
    DirectRealm,
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
    pub cursor_trail_history: Vec<(egui::Pos2, f64)>,
    pub last_hit_time: Option<f64>,
    pub demo_mode: bool,
    pub real_installed_skins: Vec<Skin>,
    pub show_build_dialog: bool,
    pub selected_build_method: BuildMethod,
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
            cursor_trail_history: Vec::new(),
            last_hit_time: None,
            demo_mode: false,
            real_installed_skins: Vec::new(),
            show_build_dialog: false,
            selected_build_method: BuildMethod::OskPackage,
        }
    }
}

const DEMO_SKIN_CATALOG: &[(&str, &str)] = &[
    ("WhiteCat 2.1 (CK)", "CYPERDARK"),
    ("Yugen", "Garin"),
    ("Aristia (Edit)", "Aristia"),
    ("Bacon Boy 1.0", "Vaxei"),
    ("Seoul v9 Personal", "cookiezi"),
    ("Rafis 2018 HDDT", "Rafis"),
    ("FlyingTuna Selyu", "FlyingTuna"),
    ("BTMC Freedom Dive", "BeasttrollMC"),
    ("Kamui [Remake]", "ReeK"),
    ("Chocopafe 1.0", "Chocopafe"),
    ("Miraie 2.0", "Miraie"),
    ("Monko 2020", "Monko2k"),
];

fn apply_demo_names(skins: &[Skin]) -> Vec<Skin> {
    skins
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut demo_skin = s.clone();
            let (demo_name, demo_creator) = DEMO_SKIN_CATALOG[i % DEMO_SKIN_CATALOG.len()];
            demo_skin.name = demo_name.to_string();
            demo_skin.creator = demo_creator.to_string();
            demo_skin
        })
        .collect()
}

pub fn generate_mock_demo_skins() -> Vec<Skin> {
    DEMO_SKIN_CATALOG
        .iter()
        .take(6)
        .enumerate()
        .map(|(i, (name, creator))| {
            let id = format!("demo-skin-{i}");
            let files = ComponentSlot::all()
                .iter()
                .map(|slot| skinforge_core::bridge::SkinFile {
                    filename: slot.primary_filename().to_string(),
                    file_hash: format!("mock_hash_{i}_{}", slot.primary_filename()),
                })
                .collect();
            Skin {
                id,
                name: name.to_string(),
                creator: creator.to_string(),
                is_protected: false,
                instantiation_info: String::new(),
                hash: format!("demo_hash_{i}"),
                delete_pending: false,
                files,
            }
        })
        .collect()
}

impl AppState {
    pub fn enable_demo_mode(&mut self) {
        self.demo_mode = true;
        if self.real_installed_skins.is_empty() && !self.installed_skins.is_empty() {
            self.real_installed_skins = self.installed_skins.clone();
        }

        if !self.real_installed_skins.is_empty() {
            self.installed_skins = apply_demo_names(&self.real_installed_skins);
        } else {
            self.installed_skins = generate_mock_demo_skins();
            if self.installed_skins.len() >= 3 {
                self.selected_base_skin_id = Some(self.installed_skins[0].id.clone());
                self.slot_sources.insert(
                    ComponentSlot::Cursor,
                    SlotSource::Donor(self.installed_skins[1].id.clone()),
                );
                self.slot_sources.insert(
                    ComponentSlot::CursorTrail,
                    SlotSource::Donor(self.installed_skins[1].id.clone()),
                );
                self.slot_sources.insert(
                    ComponentSlot::Hitnormal,
                    SlotSource::Donor(self.installed_skins[2].id.clone()),
                );
                self.slot_sources.insert(
                    ComponentSlot::Hitclap,
                    SlotSource::Donor(self.installed_skins[2].id.clone()),
                );
            }
        }

        self.new_skin_name = "WhiteCat + Yugen (Mixed)".to_string();
        self.new_skin_creator = "SkinForge".to_string();
    }

    pub fn disable_demo_mode(&mut self) {
        self.demo_mode = false;
        if !self.real_installed_skins.is_empty() {
            self.installed_skins = self.real_installed_skins.clone();
            if let Some(base_id) = &self.selected_base_skin_id {
                if let Some(base) = self.installed_skins.iter().find(|s| &s.id == base_id) {
                    self.new_skin_name = format!("{} + Mixed", base.name);
                }
            }
        } else {
            self.installed_skins.clear();
            self.selected_base_skin_id = None;
            self.slot_sources.clear();
        }
    }

    pub fn record_cursor_pos(&mut self, pos: egui::Pos2, time: f64) {
        self.cursor_trail_history.retain(|(_, t)| time - *t <= 0.25);
        if let Some((last_pos, _)) = self.cursor_trail_history.last() {
            if (*last_pos - pos).length() < 1.0 {
                return;
            }
        }
        if self.cursor_trail_history.len() >= 32 {
            self.cursor_trail_history.remove(0);
        }
        self.cursor_trail_history.push((pos, time));
    }

    pub fn trigger_hit(&mut self, time: f64) {
        self.last_hit_time = Some(time);
    }

    /// Loads skin list from database, filtering out built-in protected skins
    /// (Protected == true) and empty skins with zero files.
    pub fn set_installed_skins(&mut self, skins: Vec<Skin>) {
        let filtered: Vec<Skin> = skins
            .into_iter()
            .filter(|s| !s.is_protected && !s.files.is_empty())
            .collect();

        self.real_installed_skins = filtered.clone();
        if self.demo_mode {
            if !filtered.is_empty() {
                self.installed_skins = apply_demo_names(&filtered);
            } else if self.installed_skins.is_empty() {
                self.installed_skins = generate_mock_demo_skins();
            }
        } else {
            self.installed_skins = filtered;
        }

        if !self.installed_skins.is_empty() {
            let base_valid = self
                .selected_base_skin_id
                .as_ref()
                .is_some_and(|id| self.installed_skins.iter().any(|s| &s.id == id));
            if !base_valid {
                self.select_base_skin(&self.installed_skins[0].id.clone());
            }
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

    #[test]
    fn cursor_trail_history_and_hit_trigger() {
        let mut state = AppState::default();
        assert!(state.cursor_trail_history.is_empty());
        assert_eq!(state.last_hit_time, None);

        state.record_cursor_pos(egui::pos2(10.0, 10.0), 1.0);
        state.record_cursor_pos(egui::pos2(15.0, 15.0), 1.1);
        assert_eq!(state.cursor_trail_history.len(), 2);

        // Advance time past 0.25s: old point at 1.0 should be pruned
        state.record_cursor_pos(egui::pos2(20.0, 20.0), 1.3);
        assert_eq!(state.cursor_trail_history.len(), 2);
        assert_eq!(state.cursor_trail_history[0].1, 1.1);
        assert_eq!(state.cursor_trail_history[1].1, 1.3);

        state.trigger_hit(2.5);
        assert_eq!(state.last_hit_time, Some(2.5));
    }

    #[test]
    fn demo_mode_masks_and_restores_names() {
        let mut state = AppState::default();
        let s1 = make_skin("1", "UglyCustomName_xXx_1337", false, &["cursor.png"]);
        let s2 = make_skin("2", "Another_Weird_Folder_Name", false, &["hitcircle.png"]);
        state.set_installed_skins(vec![s1, s2]);

        assert_eq!(state.installed_skins[0].name, "UglyCustomName_xXx_1337");
        assert_eq!(state.installed_skins[1].name, "Another_Weird_Folder_Name");
        assert!(!state.demo_mode);

        state.enable_demo_mode();
        assert!(state.demo_mode);
        assert_eq!(state.installed_skins[0].name, "WhiteCat 2.1 (CK)");
        assert_eq!(state.installed_skins[1].name, "Yugen");
        assert_eq!(state.new_skin_name, "WhiteCat + Yugen (Mixed)");

        state.disable_demo_mode();
        assert!(!state.demo_mode);
        assert_eq!(state.installed_skins[0].name, "UglyCustomName_xXx_1337");
        assert_eq!(state.installed_skins[1].name, "Another_Weird_Folder_Name");
    }

    #[test]
    fn demo_mode_mock_fallback_when_empty() {
        let mut state = AppState::default();
        state.enable_demo_mode();
        assert!(state.demo_mode);
        assert!(!state.installed_skins.is_empty());
        assert_eq!(state.installed_skins[0].name, "WhiteCat 2.1 (CK)");
        assert!(state.selected_base_skin_id.is_some());

        state.disable_demo_mode();
        assert!(!state.demo_mode);
        assert!(state.installed_skins.is_empty());
    }

    #[test]
    fn build_dialog_defaults_to_closed_and_osk_method() {
        let state = AppState::default();
        assert!(!state.show_build_dialog);
        assert_eq!(state.selected_build_method, BuildMethod::OskPackage);
    }
}
