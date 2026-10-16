use skinforge_core::bridge::{RealmBridge, Skin};
use skinforge_core::mixer::{self, ManifestMixPlan, SlotSource};
use skinforge_core::paths::{self, find_realm_bridge_binary};
use skinforge_core::process_guard;
use skinforge_core::skin_ini::SkinIni;
use skinforge_core::storage::store_resource_bytes;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use crate::app::AppEvent;

pub struct ApplyMixTask {
    pub plan: ManifestMixPlan,
    pub combo_colours: Vec<(u8, u8, u8)>,
    pub installed_skins: Vec<Skin>,
    pub files_dir: PathBuf,
    pub client_realm_path: PathBuf,
}

pub enum WorkerTask {
    CheckProcess,
    LoadSkins,
    ApplyMix(Box<ApplyMixTask>),
}

pub fn run_worker(rx: Receiver<WorkerTask>, tx: Sender<AppEvent>) {
    while let Ok(task) = rx.recv() {
        match task {
            WorkerTask::CheckProcess => {
                let running = process_guard::is_game_running();
                let _ = tx.send(AppEvent::ProcessCheckResult(running));
            }
            WorkerTask::LoadSkins => match paths::resolve_lazer_root() {
                Ok(installation) => {
                    let _ = tx.send(AppEvent::InstallationFound(installation.clone()));
                    let bridge_bin = find_realm_bridge_binary();
                    match bridge_bin {
                        Some(bin) => match RealmBridge::new(bin) {
                            Ok(bridge) => match bridge.dump_skins(&installation.client_realm) {
                                Ok(skins) => {
                                    let _ = tx.send(AppEvent::SkinsLoaded(Ok(skins)));
                                }
                                Err(err) => {
                                    let _ = tx.send(AppEvent::SkinsLoaded(Err(format!(
                                        "Failed to read Realm database: {err}"
                                    ))));
                                }
                            },
                            Err(err) => {
                                let _ = tx.send(AppEvent::SkinsLoaded(Err(format!(
                                    "Failed to initialize RealmBridge: {err}"
                                ))));
                            }
                        },
                        None => {
                            let _ = tx.send(AppEvent::SkinsLoaded(Err(
                                    "realm-bridge binary not found. Please compile or bundle the realm-bridge project.".to_string(),
                                )));
                        }
                    }
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::SkinsLoaded(Err(format!(
                        "osu!lazer data directory not found: {err}"
                    ))));
                }
            },
            WorkerTask::ApplyMix(task) => {
                let result = execute_apply_mix(
                    &task.plan,
                    &task.combo_colours,
                    &task.installed_skins,
                    &task.files_dir,
                    &task.client_realm_path,
                );
                let _ = tx.send(AppEvent::MixCompleted(result));
            }
        }
    }
}

fn execute_apply_mix(
    plan: &ManifestMixPlan,
    combo_colours: &[(u8, u8, u8)],
    installed_skins: &[Skin],
    files_dir: &Path,
    client_realm_path: &Path,
) -> Result<String, String> {
    // 1. Process check
    process_guard::ensure_game_not_running().map_err(|e| format!("Cannot apply skin: {e}"))?;

    // 2. Initialize bridge
    let bridge_bin =
        find_realm_bridge_binary().ok_or_else(|| "realm-bridge binary not found".to_string())?;
    let bridge = RealmBridge::new(bridge_bin)
        .map_err(|e| format!("Failed to initialize RealmBridge: {e}"))?;

    // 3. Prepare skin.ini
    let base_skin = installed_skins
        .iter()
        .find(|s| s.id == plan.base_skin_id)
        .ok_or_else(|| "Base skin not found in installed skins list".to_string())?;

    let base_ini_file = base_skin
        .files
        .iter()
        .find(|f| f.filename.eq_ignore_ascii_case("skin.ini"));

    let base_ini_text = if let Some(f) = base_ini_file {
        let path = paths::hashed_storage_path(files_dir, &f.file_hash);
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        "[General]\n".to_string()
    };

    let mut skin_ini = SkinIni::parse(&base_ini_text);
    skin_ini.set_general("Name", &plan.new_skin_name);
    skin_ini.set_general("Author", &plan.new_skin_creator);
    skin_ini.set_combo_colours(combo_colours);

    // Merge section keys from donors
    for assignment in &plan.slots {
        if let SlotSource::Donor(donor_id) = &assignment.source {
            if let Some(donor) = installed_skins.iter().find(|s| &s.id == donor_id) {
                if let Some(donor_ini_f) = donor
                    .files
                    .iter()
                    .find(|f| f.filename.eq_ignore_ascii_case("skin.ini"))
                {
                    let donor_ini_path =
                        paths::hashed_storage_path(files_dir, &donor_ini_f.file_hash);
                    if let Ok(donor_text) = std::fs::read_to_string(&donor_ini_path) {
                        let donor_ini = SkinIni::parse(&donor_text);
                        skin_ini.merge_section_keys(
                            "General",
                            &donor_ini,
                            &[
                                "CursorExpand",
                                "CursorRotate",
                                "HitCircleOverlayAboveNumber",
                            ],
                        );
                    }
                }
            }
        }
    }

    let serialized_ini = skin_ini.to_string();
    let stored = store_resource_bytes(files_dir, serialized_ini.as_bytes())
        .map_err(|e| format!("Failed to store skin.ini in files storage: {e}"))?;

    // 4. Resolve manifest mix plan into patch operations
    let mut plan = plan.clone();
    plan.custom_skin_ini_hash = Some(stored.hash);

    let patch = mixer::resolve_manifest_mix(installed_skins, &plan)
        .map_err(|e| format!("Failed to build mix plan: {e}"))?;

    // 5. Apply patch via realm-bridge (automatic client.realm backup created inside)
    let outcome = bridge
        .apply_patch(client_realm_path, patch.operations)
        .map_err(|e| format!("Failed to apply patch in Realm: {e}"))?;

    Ok(format!(
        "Skin '{}' successfully installed into osu!lazer! (Backup: {:?})",
        plan.new_skin_name,
        outcome.backup_path.file_name().unwrap_or_default()
    ))
}
