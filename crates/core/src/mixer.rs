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
    if slot.is_audio() {
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

/// Exports the mixed skin as a standard `.osk` zip archive.
///
/// Gathers all component files according to the `ManifestMixPlan`:
/// - Base skin files as default foundation
/// - Replacements from donor skins
/// - Custom imported files from disk
/// - Customized or overridden `skin.ini`
/// - Optional HUD layout (`skin.json`)
///
/// Writes a compressed `.osk` archive to `output_osk_path`.
pub fn export_osk(
    files_dir: Option<&Path>,
    installed_skins: &[Skin],
    plan: &ManifestMixPlan,
    custom_skin_ini: Option<&str>,
    output_osk_path: &Path,
) -> CoreResult<()> {
    use std::io::Write;

    let base_skin = installed_skins
        .iter()
        .find(|s| s.id == plan.base_skin_id)
        .ok_or_else(|| {
            CoreError::BackupFailed(format!(
                "Base skin with ID '{}' not found",
                plan.base_skin_id
            ))
        })?;

    // Map: target filename in zip -> source PathBuf on disk
    let mut file_map: HashMap<String, PathBuf> = HashMap::new();

    // 1. Initialize with base skin files
    for file in &base_skin.files {
        // Skip skin.ini if custom content is supplied directly
        if file.filename.eq_ignore_ascii_case("skin.ini") && custom_skin_ini.is_some() {
            continue;
        }
        if let Some(disk_path) = base_skin.resolve_file_path(files_dir, file) {
            file_map.insert(file.filename.clone(), disk_path);
        }
    }

    // 2. Apply slot assignments
    for assignment in &plan.slots {
        let slot = assignment.slot;

        if slot == ComponentSlot::HudLayout {
            match &assignment.source {
                SlotSource::Base => {}
                SlotSource::None => {
                    file_map.retain(|k, _| !k.eq_ignore_ascii_case("skin.json"));
                }
                SlotSource::Donor(donor_id) => {
                    if let Some(donor_skin) = installed_skins.iter().find(|s| &s.id == donor_id) {
                        if let Some(donor_hud) = donor_skin
                            .files
                            .iter()
                            .find(|f| f.filename.eq_ignore_ascii_case("skin.json"))
                        {
                            if let Some(disk_path) =
                                donor_skin.resolve_file_path(files_dir, donor_hud)
                            {
                                file_map.insert("skin.json".to_string(), disk_path);
                            }
                        } else {
                            file_map.retain(|k, _| !k.eq_ignore_ascii_case("skin.json"));
                        }
                    }
                }
                SlotSource::CustomFile(path) => {
                    file_map.insert("skin.json".to_string(), path.clone());
                }
            }
            continue;
        }

        match &assignment.source {
            SlotSource::Base => {}
            SlotSource::None => {
                file_map.retain(|filename, _| {
                    let info = categorize::classify(filename);
                    !slot.matches(&info)
                });
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

                    if !donor_files.is_empty() {
                        // Prune all current files for this slot first
                        file_map.retain(|filename, _| {
                            let info = categorize::classify(filename);
                            !slot.matches(&info)
                        });

                        for df in donor_files {
                            if let Some(disk_path) = donor_skin.resolve_file_path(files_dir, df) {
                                file_map.insert(df.filename.clone(), disk_path);
                            }
                        }
                    }
                }
            }
            SlotSource::CustomFile(path) => {
                // Prune any current files for this slot
                file_map.retain(|filename, _| {
                    let info = categorize::classify(filename);
                    !slot.matches(&info)
                });

                let target_filename = target_filename_for_custom_file(slot, path);
                file_map.insert(target_filename, path.clone());
            }
        }
    }

    // 3. Create parent directory if needed
    if let Some(parent) = output_osk_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(crate::error::io_err(parent))?;
        }
    }

    // 4. Create ZIP archive
    let file =
        std::fs::File::create(output_osk_path).map_err(crate::error::io_err(output_osk_path))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write custom skin.ini if provided
    if let Some(ini_text) = custom_skin_ini {
        zip.start_file("skin.ini", options)?;
        zip.write_all(ini_text.as_bytes())
            .map_err(crate::error::io_err(output_osk_path))?;
    }

    // Write all collected files
    for (filename, disk_path) in file_map {
        if filename.eq_ignore_ascii_case("skin.ini") && custom_skin_ini.is_some() {
            continue;
        }

        if disk_path.is_file() {
            zip.start_file(&filename, options)?;
            let mut file_content =
                std::fs::File::open(&disk_path).map_err(crate::error::io_err(&disk_path))?;
            std::io::copy(&mut file_content, &mut zip).map_err(crate::error::io_err(&disk_path))?;
        }
    }

    zip.finish()?;

    Ok(())
}

/// Opens an `.osk` skin file with the default system handler (launching it in osu!lazer).
pub fn launch_osk_file(osk_path: &Path) -> CoreResult<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &osk_path.to_string_lossy()])
            .spawn()
            .map_err(crate::error::io_err(osk_path))?;
    }

    #[cfg(target_os = "linux")]
    {
        let xdg_res = std::process::Command::new("xdg-open").arg(osk_path).spawn();

        if xdg_res.is_err() {
            std::process::Command::new("flatpak")
                .args(["run", "--file-forwarding", "sh.ppy.osu", "@@"])
                .arg(osk_path)
                .arg("@@")
                .spawn()
                .map_err(crate::error::io_err(osk_path))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(osk_path)
            .spawn()
            .map_err(crate::error::io_err(osk_path))?;
    }

    Ok(())
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
            source: Default::default(),
            root_dir: None,
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
        let base_skin = make_test_skin(
            "base-id",
            "Base",
            &[("cursor.png", "base1x"), ("cursor@2x.png", "base2x")],
        );

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
        let base_skin = make_test_skin("base-id", "Base", &[("normal-hitnormal.wav", "base_wav")]);

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
        let base_with_hud = make_test_skin("base-id", "Base", &[("skin.json", "base_hud_hash")]);
        let donor_with_hud =
            make_test_skin("donor-id", "Donor", &[("skin.json", "donor_hud_hash")]);
        let donor_no_hud = make_test_skin("donor-no-hud", "Donor No HUD", &[]);

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

    #[test]
    fn export_osk_creates_valid_zip_with_all_elements() {
        let root = temp_dir("export-osk");
        let files_dir = root.join("files");
        fs::create_dir_all(&files_dir).unwrap();

        let base_cursor_bytes = b"base-cursor-png";
        let base_cursor_stored =
            storage::store_resource_bytes(&files_dir, base_cursor_bytes).unwrap();

        let base_hitcircle_bytes = b"base-hitcircle-png";
        let base_hitcircle_stored =
            storage::store_resource_bytes(&files_dir, base_hitcircle_bytes).unwrap();

        let base_skin = make_test_skin(
            "base_id",
            "BaseSkin",
            &[
                ("cursor.png", &base_cursor_stored.hash),
                ("hitcircle.png", &base_hitcircle_stored.hash),
            ],
        );

        let donor_cursor_bytes = b"donor-cursor-png";
        let donor_cursor_stored =
            storage::store_resource_bytes(&files_dir, donor_cursor_bytes).unwrap();

        let donor_skin =
            make_test_skin("donor_id", "DonorSkin", &[("cursor.png", &donor_cursor_stored.hash)]);

        let plan = ManifestMixPlan {
            base_skin_id: "base_id".to_string(),
            new_skin_name: "ExportedSkin".to_string(),
            new_skin_creator: "Mixer".to_string(),
            slots: vec![
                SlotAssignment {
                    slot: ComponentSlot::Cursor,
                    source: SlotSource::Donor("donor_id".to_string()),
                },
                SlotAssignment {
                    slot: ComponentSlot::Hitcircle,
                    source: SlotSource::Base,
                },
            ],
            custom_skin_ini_hash: None,
        };

        let osk_path = root.join("ExportedSkin.osk");
        let custom_ini = "[General]\nName: ExportedSkin\nAuthor: Mixer\n";

        export_osk(
            Some(&files_dir),
            &[base_skin, donor_skin],
            &plan,
            Some(custom_ini),
            &osk_path,
        )
        .unwrap();

        assert!(osk_path.exists());

        let file = fs::File::open(&osk_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        {
            let mut ini_entry = archive.by_name("skin.ini").unwrap();
            let mut ini_read = String::new();
            std::io::Read::read_to_string(&mut ini_entry, &mut ini_read).unwrap();
            assert_eq!(ini_read, custom_ini);
        }

        {
            let mut cursor_entry = archive.by_name("cursor.png").unwrap();
            let mut cursor_read = Vec::new();
            std::io::Read::read_to_end(&mut cursor_entry, &mut cursor_read).unwrap();
            assert_eq!(cursor_read, donor_cursor_bytes);
        }

        {
            let mut hitcircle_entry = archive.by_name("hitcircle.png").unwrap();
            let mut hitcircle_read = Vec::new();
            std::io::Read::read_to_end(&mut hitcircle_entry, &mut hitcircle_read).unwrap();
            assert_eq!(hitcircle_read, base_hitcircle_bytes);
        }

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn export_osk_mixes_stable_and_lazer_skins_without_lazer_files_dir() {
        let root = temp_dir("export-osk-stable");
        let stable_base_dir = root.join("StableBase");
        let stable_donor_dir = root.join("StableDonor");
        fs::create_dir_all(&stable_base_dir).unwrap();
        fs::create_dir_all(&stable_donor_dir).unwrap();

        fs::write(stable_base_dir.join("cursor.png"), b"stable-base-cursor").unwrap();
        fs::write(stable_base_dir.join("cursor@2x.png"), b"stable-base-cursor-2x").unwrap();
        fs::write(stable_base_dir.join("hitcircle.png"), b"stable-base-hitcircle").unwrap();

        // Donor only has 1x cursor.png -> must prune base cursor@2x.png in exported .osk
        fs::write(stable_donor_dir.join("cursor.png"), b"stable-donor-cursor-1x").unwrap();

        let skins = crate::paths::scan_stable_skins(&root);
        assert_eq!(skins.len(), 2);
        let base_id = skins.iter().find(|s| s.name == "StableBase").unwrap().id.clone();
        let donor_id = skins.iter().find(|s| s.name == "StableDonor").unwrap().id.clone();

        let plan = ManifestMixPlan {
            base_skin_id: base_id,
            new_skin_name: "MixedStable".to_string(),
            new_skin_creator: "SkinForge".to_string(),
            slots: vec![SlotAssignment {
                slot: ComponentSlot::Cursor,
                source: SlotSource::Donor(donor_id),
            }],
            custom_skin_ini_hash: None,
        };

        let osk_path = root.join("MixedStable.osk");
        export_osk(None, &skins, &plan, Some("[General]\nName: MixedStable\n"), &osk_path).unwrap();

        let file = fs::File::open(&osk_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("cursor@2x.png").is_err());

        let mut cursor_read = Vec::new();
        std::io::Read::read_to_end(&mut archive.by_name("cursor.png").unwrap(), &mut cursor_read)
            .unwrap();
        assert_eq!(cursor_read, b"stable-donor-cursor-1x");

        fs::remove_dir_all(&root).ok();
    }
}
