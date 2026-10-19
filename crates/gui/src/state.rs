use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use eframe::egui;
use skinforge_core::bridge::{Skin, SkinSource};
use skinforge_core::categorize::{ComponentCategory, ComponentSlot, ComponentSubgroup};
use skinforge_core::mixer::{LaunchTarget, ManifestMixPlan, SlotAssignment, SlotSource};
use skinforge_core::skin_ini::SkinIni;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Category(ComponentCategory),
    Colors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitJudgementKind {
    Hit300,
    Hit100,
    Hit50,
    Miss,
}

impl HitJudgementKind {
    pub fn slot(&self) -> ComponentSlot {
        match self {
            Self::Hit300 => ComponentSlot::Hit300,
            Self::Hit100 => ComponentSlot::Hit100,
            Self::Hit50 => ComponentSlot::Hit50,
            Self::Miss => ComponentSlot::Hit0,
        }
    }

    pub fn fallback_text(&self) -> &'static str {
        match self {
            Self::Hit300 => "300",
            Self::Hit100 => "100",
            Self::Hit50 => "50",
            Self::Miss => "MISS",
        }
    }

    pub fn fallback_color(&self) -> egui::Color32 {
        match self {
            Self::Hit300 => egui::Color32::from_rgb(100, 225, 255),
            Self::Hit100 => egui::Color32::from_rgb(90, 235, 140),
            Self::Hit50 => egui::Color32::from_rgb(255, 200, 90),
            Self::Miss => egui::Color32::from_rgb(255, 85, 115),
        }
    }
}

/// Safely truncates a UTF-8 string to at most `max_chars` characters, appending `…` if truncated.
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = text.chars().take(keep).collect();
    out.push('…');
    out
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub lazer_root: Option<PathBuf>,
    pub files_dir: Option<PathBuf>,
    pub client_realm_path: Option<PathBuf>,
    pub stable_root: Option<PathBuf>,
    pub custom_lazer_root: Option<PathBuf>,
    pub custom_stable_root: Option<PathBuf>,
    pub installed_skins: Vec<Skin>,
    pub selected_base_skin_id: Option<String>,
    pub new_skin_name: String,
    pub new_skin_creator: String,
    pub slot_sources: HashMap<ComponentSlot, SlotSource>,
    pub combo_colours: Vec<(u8, u8, u8)>,
    pub active_tab: ActiveTab,
    pub status_message: Option<(String, bool)>,
    pub is_working: bool,
    pub launch_target: LaunchTarget,
    pub show_advanced_cursor: bool,
    pub show_advanced_gameplay: bool,
    pub show_advanced_colors: bool,
    pub extended_ini: SkinIni,
    pub cursor_trail_history: Vec<(egui::Pos2, f64)>,
    pub last_hit_time: Option<f64>,
    pub last_hit_judgement: Option<(HitJudgementKind, f64)>,
    pub inspected_slot: ComponentSlot,
    pub show_lazer_skins: bool,
    pub show_stable_skins: bool,
    pub skin_search_query: String,
    pub expanded_subgroups: HashSet<ComponentSubgroup>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut slot_sources = HashMap::new();
        for slot in ComponentSlot::all() {
            slot_sources.insert(*slot, SlotSource::Base);
        }

        let mut expanded_subgroups = HashSet::new();
        for cat in [
            ComponentCategory::Cursor,
            ComponentCategory::Gameplay,
            ComponentCategory::Hud,
            ComponentCategory::Interface,
            ComponentCategory::Audio,
        ] {
            for &sub in ComponentSubgroup::subgroups_for_category(cat) {
                if sub.default_expanded() {
                    expanded_subgroups.insert(sub);
                }
            }
        }

        Self {
            lazer_root: None,
            files_dir: None,
            client_realm_path: None,
            stable_root: None,
            custom_lazer_root: None,
            custom_stable_root: None,
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
            active_tab: ActiveTab::Category(ComponentCategory::Cursor),
            status_message: None,
            is_working: false,
            launch_target: LaunchTarget::Lazer,
            show_advanced_cursor: false,
            show_advanced_gameplay: false,
            show_advanced_colors: false,
            extended_ini: SkinIni::default(),
            cursor_trail_history: Vec::new(),
            last_hit_time: None,
            last_hit_judgement: None,
            inspected_slot: ComponentSlot::Hitcircle,
            show_lazer_skins: true,
            show_stable_skins: true,
            skin_search_query: String::new(),
            expanded_subgroups,
        }
    }
}

impl AppState {
    pub fn visible_skins(&self) -> Vec<&Skin> {
        let query = self.skin_search_query.trim().to_lowercase();
        self.installed_skins
            .iter()
            .filter(|s| match s.source {
                SkinSource::Lazer => self.show_lazer_skins,
                SkinSource::Stable => self.show_stable_skins,
            })
            .filter(|s| {
                if query.is_empty() {
                    true
                } else {
                    s.name.to_lowercase().contains(&query)
                        || s.creator.to_lowercase().contains(&query)
                }
            })
            .collect()
    }

    pub fn count_by_source(&self, source: SkinSource) -> usize {
        self.installed_skins
            .iter()
            .filter(|s| s.source == source)
            .count()
    }

    pub fn toggle_subgroup(&mut self, subgroup: ComponentSubgroup) {
        if !self.expanded_subgroups.remove(&subgroup) {
            self.expanded_subgroups.insert(subgroup);
        }
    }

    pub fn is_subgroup_expanded(&self, subgroup: ComponentSubgroup) -> bool {
        self.expanded_subgroups.contains(&subgroup)
    }

    pub fn batch_set_subgroup(&mut self, subgroup: ComponentSubgroup, donor_id: &str) {
        for slot in subgroup.slots() {
            self.slot_sources
                .insert(slot, SlotSource::Donor(donor_id.to_string()));
        }
    }

    pub fn reset_subgroup_to_base(&mut self, subgroup: ComponentSubgroup) {
        for slot in subgroup.slots() {
            self.slot_sources.insert(slot, SlotSource::Base);
        }
    }

    pub fn record_hit_judgement(&mut self, cycle: f32, time: f64) -> HitJudgementKind {
        let judgement = if cycle >= 0.72 {
            HitJudgementKind::Hit300
        } else if cycle >= 0.45 {
            HitJudgementKind::Hit100
        } else if cycle >= 0.22 {
            HitJudgementKind::Hit50
        } else {
            HitJudgementKind::Miss
        };
        self.last_hit_time = Some(time);
        self.last_hit_judgement = Some((judgement, time));
        judgement
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
        self.last_hit_judgement = Some((HitJudgementKind::Hit300, time));
    }

    /// Loads skin list from database and/or osu!stable folder, filtering out built-in protected skins
    /// (Protected == true) and empty skins with zero files.
    pub fn set_installed_skins(&mut self, skins: Vec<Skin>) {
        self.installed_skins = skins
            .into_iter()
            .filter(|s| !s.is_protected && !s.files.is_empty())
            .collect();

        if !self.installed_skins.is_empty() {
            let base_valid = self
                .selected_base_skin_id
                .as_ref()
                .is_some_and(|id| self.installed_skins.iter().any(|s| &s.id == id));
            if !base_valid {
                let first_id = self.installed_skins[0].id.clone();
                self.select_base_skin(&first_id);
            }
        }
    }

    /// Selects a base skin and sets the default new skin name.
    pub fn select_base_skin(&mut self, skin_id: &str) {
        if let Some(skin) = self.installed_skins.iter().find(|s| s.id == skin_id) {
            self.selected_base_skin_id = Some(skin.id.clone());
            self.new_skin_name = format!("{} + Mixed", truncate_text(&skin.name, 24));

            // Attempt to load base skin's skin.ini into extended_ini
            if let Some(ini_file) = skin
                .files
                .iter()
                .find(|f| f.filename.eq_ignore_ascii_case("skin.ini"))
            {
                if let Some(path) = skin.resolve_file_path(self.files_dir.as_deref(), ini_file) {
                    if let Ok(parsed) = SkinIni::read_from_file(&path) {
                        self.extended_ini = parsed;
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
            source: Default::default(),
            root_dir: None,
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
    fn default_state_enables_lazer_launch_and_both_version_filters() {
        let state = AppState::default();
        assert_eq!(state.launch_target, LaunchTarget::Lazer);
        assert!(state.show_lazer_skins);
        assert!(state.show_stable_skins);
    }

    #[test]
    fn truncate_text_handles_multibyte_utf8_and_short_strings() {
        assert_eq!(truncate_text("Short", 10), "Short");
        assert_eq!(
            truncate_text("『 WhiteCat 2.1 (CK) Very Long Skin Name 』", 14),
            "『 WhiteCat 2.…"
        );
        assert_eq!(truncate_text("КириллицаДлинноеИмя", 9), "Кириллиц…");
    }

    #[test]
    fn visible_skins_filters_by_source_and_search_query() {
        let mut state = AppState::default();
        let mut s_lazer = make_skin("1", "Rafis HDDT", false, &["cursor.png"]);
        s_lazer.source = skinforge_core::bridge::SkinSource::Lazer;
        let mut s_stable = make_skin("2", "WhiteCat 1.0", false, &["cursor.png"]);
        s_stable.source = skinforge_core::bridge::SkinSource::Stable;

        state.set_installed_skins(vec![s_lazer, s_stable]);
        assert_eq!(state.visible_skins().len(), 2);

        state.show_lazer_skins = false;
        assert_eq!(state.visible_skins().len(), 1);
        assert_eq!(state.visible_skins()[0].name, "WhiteCat 1.0");

        state.show_lazer_skins = true;
        state.skin_search_query = "rafis".to_string();
        assert_eq!(state.visible_skins().len(), 1);
        assert_eq!(state.visible_skins()[0].name, "Rafis HDDT");
    }

    #[test]
    fn record_hit_judgement_assigns_300_100_50_and_miss_by_cycle_timing() {
        let mut state = AppState::default();
        assert_eq!(
            state.record_hit_judgement(0.85, 1.0),
            HitJudgementKind::Hit300
        );
        assert_eq!(
            state.record_hit_judgement(0.60, 2.0),
            HitJudgementKind::Hit100
        );
        assert_eq!(
            state.record_hit_judgement(0.30, 3.0),
            HitJudgementKind::Hit50
        );
        assert_eq!(
            state.record_hit_judgement(0.10, 4.0),
            HitJudgementKind::Miss
        );
        assert_eq!(
            state.last_hit_judgement,
            Some((HitJudgementKind::Miss, 4.0))
        );
    }
}
