use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use osu_mixer_core::bridge::{RealmBridge, Skin};
use osu_mixer_core::mixer::{self, ManifestMixPlan, SlotSource};
use osu_mixer_core::paths::{self, find_realm_bridge_binary};
use osu_mixer_core::process_guard;
use osu_mixer_core::skin_ini::SkinIni;
use osu_mixer_core::storage::store_resource_bytes;

use crate::app::AppEvent;

pub enum WorkerTask {
    CheckProcess,
    LoadSkins,
    ApplyMix {
        plan: ManifestMixPlan,
        combo_colours: Vec<(u8, u8, u8)>,
        installed_skins: Vec<Skin>,
        files_dir: PathBuf,
        client_realm_path: PathBuf,
    },
}

pub fn run_worker(rx: Receiver<WorkerTask>, tx: Sender<AppEvent>) {
    while let Ok(task) = rx.recv() {
        match task {
            WorkerTask::CheckProcess => {
                let running = process_guard::is_game_running();
                let _ = tx.send(AppEvent::ProcessCheckResult(running));
            }
            WorkerTask::LoadSkins => {
                match paths::resolve_lazer_root() {
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
                                            "Ошибка чтения Realm базы данных: {err}"
                                        ))));
                                    }
                                },
                                Err(err) => {
                                    let _ = tx.send(AppEvent::SkinsLoaded(Err(format!(
                                        "Ошибка инициализации RealmBridge: {err}"
                                    ))));
                                }
                            },
                            None => {
                                let _ = tx.send(AppEvent::SkinsLoaded(Err(
                                    "Бинарник realm-bridge не найден. Скомпилируйте проект realm-bridge перед запуском.".to_string(),
                                )));
                            }
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(AppEvent::SkinsLoaded(Err(format!(
                            "Директория osu! lazer не обнаружена: {err}"
                        ))));
                    }
                }
            }
            WorkerTask::ApplyMix {
                plan,
                combo_colours,
                installed_skins,
                files_dir,
                client_realm_path,
            } => {
                let result = execute_apply_mix(
                    &plan,
                    &combo_colours,
                    &installed_skins,
                    &files_dir,
                    &client_realm_path,
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
    // 1. Проверка процесса
    process_guard::ensure_game_not_running()
        .map_err(|e| format!("Невозможно применить скин: {e}"))?;

    // 2. Инициализация моста
    let bridge_bin = find_realm_bridge_binary()
        .ok_or_else(|| "Бинарник realm-bridge не найден".to_string())?;
    let bridge = RealmBridge::new(bridge_bin)
        .map_err(|e| format!("Ошибка создания RealmBridge: {e}"))?;

    // 3. Подготовка skin.ini
    let base_skin = installed_skins
        .iter()
        .find(|s| s.id == plan.base_skin_id)
        .ok_or_else(|| "Базовый скин не найден в списке установленных".to_string())?;

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

    // Слияние секций из доноров
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
                            &["CursorExpand", "CursorRotate", "HitCircleOverlayAboveNumber"],
                        );
                    }
                }
            }
        }
    }

    let serialized_ini = skin_ini.to_string();
    let stored = store_resource_bytes(files_dir, serialized_ini.as_bytes())
        .map_err(|e| format!("Ошибка сохранения skin.ini в хранилище: {e}"))?;

    // 4. Построение манифеста микса и операций патча
    let mut plan = plan.clone();
    plan.custom_skin_ini_hash = Some(stored.hash);

    let patch = mixer::resolve_manifest_mix(installed_skins, &plan)
        .map_err(|e| format!("Ошибка формирования плана микса: {e}"))?;

    // 5. Применение патча через realm-bridge (бэкап базы client.realm создается внутри)
    let outcome = bridge
        .apply_patch(client_realm_path, patch.operations)
        .map_err(|e| format!("Ошибка применения патча в Realm: {e}"))?;

    Ok(format!(
        "Скин '{}' успешно создан в osu! lazer! (Бэкап: {:?})",
        plan.new_skin_name,
        outcome.backup_path.file_name().unwrap_or_default()
    ))
}
