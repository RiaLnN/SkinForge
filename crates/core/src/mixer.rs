use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::bridge::{PatchOperation, Skin};
use crate::categorize::{self, ComponentCategory, ComponentInfo, ComponentSlot};
use crate::error::{CoreError, CoreResult};
use crate::skin_ini::SkinIni;
use crate::storage;

/// Donor skin loaded from a directory: parsed `skin.ini` and categorized components.
#[derive(Debug, Clone)]
pub struct DonorSkin {
    /// Directory containing the `skin.ini`.
    pub root: PathBuf,
    pub ini: SkinIni,
    pub components: Vec<ComponentInfo>,
}

impl DonorSkin {
    pub fn components_by_category(&self, category: ComponentCategory) -> Vec<&ComponentInfo> {
        self.components
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    pub fn grouped_by_category(&self) -> HashMap<ComponentCategory, Vec<&ComponentInfo>> {
        let mut map: HashMap<ComponentCategory, Vec<&ComponentInfo>> = HashMap::new();
        for component in &self.components {
            map.entry(component.category).or_default().push(component);
        }
        map
    }

    /// Absolute path to a donor resource file given its relative filename.
    pub fn resource_path(&self, relative_filename: &str) -> PathBuf {
        self.root.join(relative_filename)
    }
}

/// Loads a donor skin from a directory: locates `skin.ini`, parses it, and scans all components.
pub fn load_donor(extracted_root: &Path) -> CoreResult<DonorSkin> {
    let ini_path = find_skin_ini(extracted_root).ok_or_else(|| {
        CoreError::BackupFailed(format!(
            "skin.ini not found in '{}'",
            extracted_root.display()
        ))
    })?;

    let effective_root = ini_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| extracted_root.to_path_buf());

    let ini = SkinIni::read_from_file(&ini_path)?;
    let components = scan_components(&effective_root, &ini_path)?;

    Ok(DonorSkin {
        root: effective_root,
        ini,
        components,
    })
}

fn find_skin_ini(root: &Path) -> Option<PathBuf> {
    let direct = root.join("skin.ini");
    if direct.is_file() {
        return Some(direct);
    }

    for entry in walkdir::WalkDir::new(root).min_depth(1).max_depth(3) {
        let Ok(entry) = entry else { continue };
        if entry.file_type().is_file() && entry.file_name().eq_ignore_ascii_case("skin.ini") {
            return Some(entry.path().to_path_buf());
        }
    }

    None
}

fn scan_components(effective_root: &Path, ini_path: &Path) -> CoreResult<Vec<ComponentInfo>> {
    let mut components = Vec::new();

    for entry in walkdir::WalkDir::new(effective_root).min_depth(1) {
        let entry = entry
            .map_err(|e| CoreError::BackupFailed(format!("failed to walk donor directory: {e}")))?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path() == ini_path {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(effective_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        let basename = entry.file_name().to_string_lossy().to_string();
        let mut info = categorize::classify(&basename);
        info.filename = relative;
        components.push(info);
    }

    Ok(components)
}

/// A selected replacement: source file on disk mapping to a target skin filename.
#[derive(Debug, Clone)]
pub struct ReplacementSelection {
    pub target_filename: String,
    pub source_path: PathBuf,
}

/// Mix plan: base skin ID, new metadata, and replacements.
#[derive(Debug, Clone)]
pub struct MixPlan {
    pub base_skin_id: String,
    pub new_skin_name: String,
    pub new_skin_creator: String,
    pub replacements: Vec<ReplacementSelection>,
}

/// Generated patch for Realm bridge: new skin ID and patch operations.
pub struct GeneratedPatch {
    pub new_skin_id: String,
    pub operations: Vec<PatchOperation>,
}

/// Hashes and stores each replacement file into `files_dir`, then generates patch operations:
/// `duplicate_skin` + `add_file` for each replacement.
pub fn build_patch(files_dir: &Path, plan: &MixPlan) -> CoreResult<GeneratedPatch> {
    let new_skin_id = uuid::Uuid::new_v4().to_string();

    let mut operations = vec![PatchOperation::duplicate_skin(
        plan.base_skin_id.clone(),
        new_skin_id.clone(),
        plan.new_skin_name.clone(),
        plan.new_skin_creator.clone(),
    )];

    for replacement in &plan.replacements {
        let stored = storage::store_resource(files_dir, &replacement.source_path)?;
        operations.push(PatchOperation::add_file(
            new_skin_id.clone(),
            replacement.target_filename.clone(),
            stored.hash,
        ));
    }

    Ok(GeneratedPatch {
        new_skin_id,
        operations,
    })
}

/// Source of a component for a slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotSource {
    /// Keep the file from the base skin (default).
    Base,
    /// Take the component from another installed skin by ID.
    Donor(String),
    /// Directly imported custom file from disk.
    CustomFile(PathBuf),
    /// Completely remove the component (for optional components like cursormiddle, combobreak).
    None,
}

/// Slot assignment for mixing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAssignment {
    pub slot: ComponentSlot,
    pub source: SlotSource,
}

/// Mix plan based on skin manifests from Realm (zero-copy asset mapping).
#[derive(Debug, Clone)]
pub struct ManifestMixPlan {
    pub base_skin_id: String,
    pub new_skin_name: String,
    pub new_skin_creator: String,
    pub slots: Vec<SlotAssignment>,
    pub custom_skin_ini_hash: Option<String>,
}

fn target_filename_for_custom_file(slot: ComponentSlot, source_path: &Path) -> String {
    let primary = slot.primary_filename();
    let lower_primary = primary.to_ascii_lowercase();
    let is_audio = lower_primary.ends_with(".wav")
        || lower_primary.ends_with(".ogg")
        || lower_primary.ends_with(".mp3");
    if is_audio {
        if let Some(ext) = source_path.extension().and_then(|e| e.to_str()) {
            let lower_ext = ext.to_ascii_lowercase();
            if lower_ext == "wav" || lower_ext == "ogg" || lower_ext == "mp3" {
                if let Some((stem, _)) = primary.rsplit_once('.') {
                    return format!("{stem}.{lower_ext}");
                }
            }
        }
    }
    primary.to_string()
}

/// Resolves a zero-copy manifest mix plan into Realm patch operations:
/// - `duplicate_skin`: clones base skin under a new ID
/// - `add_file`: references existing hashes from donor skins (zero duplicate disk bytes)
/// - `remove_file`: prunes orphaned @2x files or disabled components (None)
/// - fallback: if donor lacks a component, preserves the base component
pub fn resolve_manifest_mix(
    installed_skins: &[Skin],
    plan: &ManifestMixPlan,
) -> CoreResult<GeneratedPatch> {
    resolve_manifest_mix_with_storage(Path::new(""), installed_skins, plan)
}

/// Resolves a manifest mix plan into Realm patch operations, with access to `files_dir`
/// for staging newly imported custom files.
pub fn resolve_manifest_mix_with_storage(
    files_dir: &Path,
    installed_skins: &[Skin],
    plan: &ManifestMixPlan,
) -> CoreResult<GeneratedPatch> {
    let base_skin = installed_skins
        .iter()
        .find(|s| s.id == plan.base_skin_id)
        .ok_or_else(|| {
            CoreError::BackupFailed(format!(
                "Base skin with ID '{}' not found",
                plan.base_skin_id
            ))
        })?;

    let new_skin_id = uuid::Uuid::new_v4().to_string();

    let mut operations = vec![PatchOperation::duplicate_skin(
        base_skin.id.clone(),
        new_skin_id.clone(),
        plan.new_skin_name.clone(),
        plan.new_skin_creator.clone(),
    )];

    for assignment in &plan.slots {
        let slot = assignment.slot;

        if slot == ComponentSlot::HudLayout {
            match &assignment.source {
                SlotSource::Base => {}
                SlotSource::None => {
                    if let Some(base_hud) = base_skin
                        .files
                        .iter()
                        .find(|f| f.filename.eq_ignore_ascii_case("skin.json"))
                    {
                        operations.push(PatchOperation::remove_file(
                            new_skin_id.clone(),
                            base_hud.filename.clone(),
                        ));
                    }
                }
                SlotSource::Donor(donor_id) => {
                    if let Some(donor_skin) = installed_skins.iter().find(|s| &s.id == donor_id) {
                        if let Some(donor_hud) = donor_skin
                            .files
                            .iter()
                            .find(|f| f.filename.eq_ignore_ascii_case("skin.json"))
                        {
                            operations.push(PatchOperation::add_file(
                                new_skin_id.clone(),
                                "skin.json".to_string(),
                                donor_hud.file_hash.clone(),
                            ));
                        } else if let Some(base_hud) = base_skin
                            .files
                            .iter()
                            .find(|f| f.filename.eq_ignore_ascii_case("skin.json"))
                        {
                            operations.push(PatchOperation::remove_file(
                                new_skin_id.clone(),
                                base_hud.filename.clone(),
                            ));
                        }
                    }
                }
                SlotSource::CustomFile(path) => {
                    let stored = storage::store_resource(files_dir, path)?;
                    operations.push(PatchOperation::add_file(
                        new_skin_id.clone(),
                        "skin.json".to_string(),
                        stored.hash,
                    ));
                }
            }
            continue;
        }

        let base_files: Vec<_> = base_skin
            .files
            .iter()
            .filter(|f| {
                let info = categorize::classify(&f.filename);
                slot.matches(&info)
            })
            .collect();

        match &assignment.source {
            SlotSource::Base => {}
            SlotSource::None => {
                for base_file in base_files {
                    operations.push(PatchOperation::remove_file(
                        new_skin_id.clone(),
                        base_file.filename.clone(),
                    ));
                }
            }
            SlotSource::Donor(donor_id) => {
                if let Some(donor_skin) = installed_skins.iter().find(|s| &s.id == donor_id) {
                    let donor_files: Vec<_> = donor_skin
                        .files
                        .iter()
                        .filter(|f| {
                            let info = categorize::classify(&f.filename);
                            slot.matches(&info)
                        })
                        .collect();

                    if donor_files.is_empty() {
                        // Fallback: donor doesn't have this component, preserve base
                        continue;
                    }

                    for donor_file in &donor_files {
                        operations.push(PatchOperation::add_file(
                            new_skin_id.clone(),
                            donor_file.filename.clone(),
                            donor_file.file_hash.clone(),
                        ));
                    }

                    // Prune orphaned base sprites (e.g. base had cursor@2x, donor only had 1x cursor):
                    for base_file in base_files {
                        if !donor_files
                            .iter()
                            .any(|df| df.filename.eq_ignore_ascii_case(&base_file.filename))
                        {
                            operations.push(PatchOperation::remove_file(
                                new_skin_id.clone(),
                                base_file.filename.clone(),
                            ));
                        }
                    }
                }
            }
            SlotSource::CustomFile(path) => {
                let stored = storage::store_resource(files_dir, path)?;
                let target_filename = target_filename_for_custom_file(slot, path);

                operations.push(PatchOperation::add_file(
                    new_skin_id.clone(),
                    target_filename.clone(),
                    stored.hash,
                ));

                // Prune any base files that belong to this slot but differ from target_filename
                // (e.g. cursor@2x.png when importing cursor.png, or normal-hitnormal.wav when importing .ogg)
                for base_file in base_files {
                    if !base_file.filename.eq_ignore_ascii_case(&target_filename) {
                        operations.push(PatchOperation::remove_file(
                            new_skin_id.clone(),
                            base_file.filename.clone(),
                        ));
                    }
                }
            }
        }
    }

    if let Some(ini_hash) = &plan.custom_skin_ini_hash {
        operations.push(PatchOperation::add_file(
            new_skin_id.clone(),
            "skin.ini",
            ini_hash.clone(),
        ));
    }

    Ok(GeneratedPatch {
        new_skin_id,
        operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "skinforge-mixer-test-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_donor_from_flat_directory() {
        let root = temp_dir("flat");
        fs::write(
            root.join("skin.ini"),
            "[General]\nName: Flat Donor\nAuthor: me\n",
        )
        .unwrap();
        fs::write(root.join("cursor.png"), b"fake").unwrap();
        fs::create_dir_all(root.join("sounds")).unwrap();
        fs::write(root.join("sounds/hitnormal.wav"), b"fake").unwrap();

        let donor = load_donor(&root).unwrap();
        assert_eq!(donor.ini.skin_name(), Some("Flat Donor"));
        assert_eq!(donor.components.len(), 2);

        let cursor = donor
            .components
            .iter()
            .find(|c| c.filename == "cursor.png")
            .unwrap();
        assert_eq!(cursor.category, ComponentCategory::Cursor);

        let hitnormal = donor
            .components
            .iter()
            .find(|c| c.filename.ends_with("hitnormal.wav"))
            .unwrap();
        assert_eq!(hitnormal.category, ComponentCategory::Audio);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn loads_donor_from_nested_single_subfolder() {
        let root = temp_dir("nested");
        let nested = root.join("My Donor Skin");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("skin.ini"), "[General]\nName: Nested Donor\n").unwrap();
        fs::write(nested.join("hitcircle.png"), b"fake").unwrap();

        let donor = load_donor(&root).unwrap();
        assert_eq!(donor.root, nested);
        assert_eq!(donor.ini.skin_name(), Some("Nested Donor"));
        assert_eq!(donor.components.len(), 1);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_skin_ini_is_an_error() {
        let root = temp_dir("missing-ini");
        fs::write(root.join("cursor.png"), b"fake").unwrap();

        let result = load_donor(&root);
        assert!(result.is_err());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn build_patch_generates_duplicate_plus_add_file_ops() {
        let work_dir = temp_dir("patch");
        let files_dir = work_dir.join("files");
        let source_file = work_dir.join("cursor.png");
        fs::write(&source_file, b"fake cursor bytes").unwrap();

        let plan = MixPlan {
            base_skin_id: "11111111-1111-1111-1111-111111111111".to_string(),
            new_skin_name: "My Mix".to_string(),
            new_skin_creator: "me".to_string(),
            replacements: vec![ReplacementSelection {
                target_filename: "cursor.png".to_string(),
                source_path: source_file,
            }],
        };

        let generated = build_patch(&files_dir, &plan).unwrap();
        assert_eq!(generated.operations.len(), 2);
        assert!(!generated.new_skin_id.is_empty());

        let stored_files: Vec<_> = walkdir::WalkDir::new(&files_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();
        assert_eq!(stored_files.len(), 1);

        fs::remove_dir_all(&work_dir).ok();
    }

    fn make_test_skin(id: &str, name: &str, files: &[(&str, &str)]) -> Skin {
        Skin {
            id: id.to_string(),
            name: name.to_string(),
            creator: "test".to_string(),
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: files
                .iter()
                .map(|(f, h)| crate::bridge::SkinFile {
                    filename: f.to_string(),
                    file_hash: h.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn mix_plan_generates_zero_copy_references() {
        let base = make_test_skin("base-id", "Base", &[("cursor.png", "base_hash")]);
        let donor = make_test_skin("donor-id", "Donor", &[("cursor.png", "donor_hash")]);

        let plan = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::Cursor,
                source: SlotSource::Donor("donor-id".to_string()),
            }],
            custom_skin_ini_hash: None,
        };

        let patch = resolve_manifest_mix(&[base, donor], &plan).unwrap();
        assert_eq!(patch.operations.len(), 2);
        assert_eq!(patch.operations[0].op, "duplicate_skin");
        assert_eq!(patch.operations[1].op, "add_file");
        assert_eq!(patch.operations[1].filename.as_deref(), Some("cursor.png"));
        assert_eq!(patch.operations[1].file_hash.as_deref(), Some("donor_hash"));
    }

    #[test]
    fn mix_plan_removes_orphaned_2x_sprites() {
        let base = make_test_skin(
            "base-id",
            "Base",
            &[("cursor.png", "base_1x"), ("cursor@2x.png", "base_2x")],
        );
        let donor = make_test_skin("donor-id", "Donor", &[("cursor.png", "donor_1x")]);

        let plan = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::Cursor,
                source: SlotSource::Donor("donor-id".to_string()),
            }],
            custom_skin_ini_hash: None,
        };

        let patch = resolve_manifest_mix(&[base, donor], &plan).unwrap();
        assert_eq!(patch.operations.len(), 3);
        assert!(patch
            .operations
            .iter()
            .any(|op| op.op == "add_file" && op.filename.as_deref() == Some("cursor.png")));
        assert!(patch
            .operations
            .iter()
            .any(|op| op.op == "remove_file" && op.filename.as_deref() == Some("cursor@2x.png")));
    }

    #[test]
    fn mix_plan_handles_explicit_none() {
        let base = make_test_skin("base-id", "Base", &[("cursormiddle.png", "mid_hash")]);

        let plan = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::CursorMiddle,
                source: SlotSource::None,
            }],
            custom_skin_ini_hash: None,
        };

        let patch = resolve_manifest_mix(&[base], &plan).unwrap();
        assert_eq!(patch.operations.len(), 2);
        assert_eq!(patch.operations[1].op, "remove_file");
        assert_eq!(
            patch.operations[1].filename.as_deref(),
            Some("cursormiddle.png")
        );
    }

    #[test]
    fn mix_plan_falls_back_when_donor_lacks_component() {
        let base = make_test_skin(
            "base-id",
            "Base",
            &[("approachcircle.png", "base_app_hash")],
        );
        let donor = make_test_skin("donor-id", "Donor", &[("cursor.png", "donor_cursor_hash")]);

        let plan = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::ApproachCircle,
                source: SlotSource::Donor("donor-id".to_string()),
            }],
            custom_skin_ini_hash: None,
        };

        let patch = resolve_manifest_mix(&[base, donor], &plan).unwrap();
        assert_eq!(patch.operations.len(), 1);
        assert_eq!(patch.operations[0].op, "duplicate_skin");
    }

    #[test]
    fn custom_file_import_stages_bytes_and_prunes_base_retina() {
        let files_dir = temp_dir("custom-files");
        let base_skin = Skin {
            id: "base-id".to_string(),
            name: "Base".to_string(),
            creator: "ck".to_string(),
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: vec![
                crate::bridge::SkinFile {
                    filename: "cursor.png".to_string(),
                    file_hash: "base1x".to_string(),
                },
                crate::bridge::SkinFile {
                    filename: "cursor@2x.png".to_string(),
                    file_hash: "base2x".to_string(),
                },
            ],
        };

        let custom_file = files_dir.join("my_cursor.png");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(&custom_file, b"brand-new-custom-cursor-content").unwrap();

        let plan = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::Cursor,
                source: SlotSource::CustomFile(custom_file),
            }],
            custom_skin_ini_hash: None,
        };

        let patch = resolve_manifest_mix_with_storage(&files_dir, &[base_skin], &plan).unwrap();

        // Must add the custom file as cursor.png
        assert!(patch
            .operations
            .iter()
            .any(|op| { op.op == "add_file" && op.filename.as_deref() == Some("cursor.png") }));
        // Must remove the base cursor@2x.png so it doesn't shadow custom 1x
        assert!(patch.operations.iter().any(|op| {
            op.op == "remove_file" && op.filename.as_deref() == Some("cursor@2x.png")
        }));
    }

    #[test]
    fn custom_audio_file_retains_extension_and_prunes_base_wav() {
        let files_dir = temp_dir("custom-audio");
        let base_skin = Skin {
            id: "base-id".to_string(),
            name: "Base".to_string(),
            creator: "ck".to_string(),
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: vec![crate::bridge::SkinFile {
                filename: "normal-hitnormal.wav".to_string(),
                file_hash: "base_wav".to_string(),
            }],
        };

        let custom_file = files_dir.join("hit.ogg");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(&custom_file, b"custom-ogg-hitsound-bytes").unwrap();

        let plan = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::Hitnormal,
                source: SlotSource::CustomFile(custom_file),
            }],
            custom_skin_ini_hash: None,
        };

        let patch = resolve_manifest_mix_with_storage(&files_dir, &[base_skin], &plan).unwrap();

        // Must retain the .ogg extension: normal-hitnormal.ogg
        assert!(patch.operations.iter().any(|op| {
            op.op == "add_file" && op.filename.as_deref() == Some("normal-hitnormal.ogg")
        }));
        // Must prune base normal-hitnormal.wav
        assert!(patch.operations.iter().any(|op| {
            op.op == "remove_file" && op.filename.as_deref() == Some("normal-hitnormal.wav")
        }));
    }

    #[test]
    fn hud_layout_transfer_from_donor_and_default_removal() {
        let base_with_hud = Skin {
            id: "base-id".to_string(),
            name: "Base".to_string(),
            creator: "ck".to_string(),
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: vec![crate::bridge::SkinFile {
                filename: "skin.json".to_string(),
                file_hash: "base_hud_hash".to_string(),
            }],
        };

        let donor_with_hud = Skin {
            id: "donor-id".to_string(),
            name: "Donor".to_string(),
            creator: "ck".to_string(),
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: vec![crate::bridge::SkinFile {
                filename: "skin.json".to_string(),
                file_hash: "donor_hud_hash".to_string(),
            }],
        };

        let donor_no_hud = Skin {
            id: "donor-no-hud".to_string(),
            name: "Donor No HUD".to_string(),
            creator: "ck".to_string(),
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files: vec![],
        };

        // 1. Transfer from donor with skin.json
        let plan_donor = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::HudLayout,
                source: SlotSource::Donor("donor-id".to_string()),
            }],
            custom_skin_ini_hash: None,
        };
        let patch =
            resolve_manifest_mix(&[base_with_hud.clone(), donor_with_hud], &plan_donor).unwrap();
        assert!(patch.operations.iter().any(|op| {
            op.op == "add_file"
                && op.filename.as_deref() == Some("skin.json")
                && op.file_hash.as_deref() == Some("donor_hud_hash")
        }));

        // 2. Transfer from donor without skin.json should remove base skin.json
        let plan_fallback = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::HudLayout,
                source: SlotSource::Donor("donor-no-hud".to_string()),
            }],
            custom_skin_ini_hash: None,
        };
        let patch2 =
            resolve_manifest_mix(&[base_with_hud.clone(), donor_no_hud], &plan_fallback).unwrap();
        assert!(patch2
            .operations
            .iter()
            .any(|op| { op.op == "remove_file" && op.filename.as_deref() == Some("skin.json") }));

        // 3. SlotSource::None should remove base skin.json
        let plan_none = ManifestMixPlan {
            base_skin_id: "base-id".to_string(),
            new_skin_name: "Mixed".to_string(),
            new_skin_creator: "me".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::HudLayout,
                source: SlotSource::None,
            }],
            custom_skin_ini_hash: None,
        };
        let patch3 = resolve_manifest_mix(&[base_with_hud], &plan_none).unwrap();
        assert!(patch3
            .operations
            .iter()
            .any(|op| { op.op == "remove_file" && op.filename.as_deref() == Some("skin.json") }));
    }
}
