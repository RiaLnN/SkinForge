use skinforge_core::bridge::{RealmBridge, Skin};
use skinforge_core::mixer::{self, LaunchTarget, ManifestMixPlan};
use skinforge_core::paths::{self, find_realm_bridge_binary};
use skinforge_core::skin_ini::SkinIni;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use crate::app::AppEvent;

pub struct ExportOskTask {
    pub plan: ManifestMixPlan,
    pub combo_colours: Vec<(u8, u8, u8)>,
    pub installed_skins: Vec<Skin>,
    pub files_dir: Option<PathBuf>,
    pub stable_root: Option<PathBuf>,
    pub output_path: PathBuf,
    pub launch_target: LaunchTarget,
    pub extended_ini: SkinIni,
}

pub enum WorkerTask {
    LoadSkins {
        custom_lazer_root: Option<PathBuf>,
        custom_stable_root: Option<PathBuf>,
    },
    ExportOsk(Box<ExportOskTask>),
}

pub fn run_worker(rx: Receiver<WorkerTask>, tx: Sender<AppEvent>) {
    while let Ok(task) = rx.recv() {
        match task {
            WorkerTask::LoadSkins {
                custom_lazer_root,
                custom_stable_root,
            } => {
                let mut combined_skins: Vec<Skin> = Vec::new();
                let mut found_any_source = false;
                let mut lazer_error: Option<String> = None;

                // 1. Discover osu!lazer skins (if available)
                if let Ok(installation) =
                    paths::resolve_lazer_root_with_override(custom_lazer_root)
                {
                    let _ = tx.send(AppEvent::LazerInstallationFound(installation.clone()));
                    match find_realm_bridge_binary() {
                        Some(bin) => match RealmBridge::new(bin) {
                            Ok(bridge) => match bridge.dump_skins(&installation.client_realm) {
                                Ok(skins) => {
                                    found_any_source = true;
                                    combined_skins.extend(skins);
                                }
                                Err(err) => {
                                    lazer_error =
                                        Some(format!("Failed to read osu!lazer Realm DB: {err}"));
                                }
                            },
                            Err(err) => {
                                lazer_error =
                                    Some(format!("Failed to initialize RealmBridge: {err}"));
                            }
                        },
                        None => {
                            lazer_error = Some(
                                "realm-bridge binary not found for osu!lazer skin loading."
                                    .to_string(),
                            );
                        }
                    }
                }

                // 2. Discover osu!stable skins (if available)
                if let Some(stable_inst) =
                    paths::resolve_stable_root_with_override(custom_stable_root)
                {
                    let _ = tx.send(AppEvent::StableInstallationFound(stable_inst.clone()));
                    let stable_skins = paths::scan_stable_skins(&stable_inst.skins_dir);
                    found_any_source = true;
                    combined_skins.extend(stable_skins);
                }

                if found_any_source {
                    let _ = tx.send(AppEvent::SkinsLoaded(Ok(combined_skins)));
                } else if let Some(err) = lazer_error {
                    let _ = tx.send(AppEvent::SkinsLoaded(Err(err)));
                } else {
                    let _ = tx.send(AppEvent::SkinsLoaded(Err(
                        "Neither osu!lazer nor osu!stable skin directory was found. Use the folder buttons in the sidebar to select your osu! folder."
                            .to_string(),
                    )));
                }
            }
            WorkerTask::ExportOsk(task) => {
                let result = execute_export_osk(
                    &task.plan,
                    &task.combo_colours,
                    &task.installed_skins,
                    task.files_dir.as_deref(),
                    task.stable_root.as_deref(),
                    &task.output_path,
                    task.launch_target,
                    &task.extended_ini,
                );
                let _ = tx.send(AppEvent::MixCompleted(result));
            }
        }
    }
}

pub fn sanitize_skin_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        "Mixed_Skin".to_string()
    } else {
        trimmed.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_export_osk(
    plan: &ManifestMixPlan,
    combo_colours: &[(u8, u8, u8)],
    installed_skins: &[Skin],
    files_dir: Option<&Path>,
    stable_root: Option<&Path>,
    output_path: &Path,
    launch_target: LaunchTarget,
    extended_ini: &SkinIni,
) -> Result<String, String> {
    let mut skin_ini = extended_ini.clone();
    skin_ini.set_general("Name", &plan.new_skin_name);
    skin_ini.set_general("Author", &plan.new_skin_creator);
    skin_ini.set_combo_colours(combo_colours);

    // If ComboNumbers is overridden by a donor skin, normalize HitCirclePrefix to "default"
    // so custom subfolder/prefix overrides in the base skin's [Fonts] don't break digit rendering.
    if plan.slots.iter().any(|s| {
        s.slot == skinforge_core::categorize::ComponentSlot::ComboNumbers
            && matches!(s.source, skinforge_core::mixer::SlotSource::Donor(_))
    }) {
        skin_ini
            .get_or_create_section_mut("Fonts")
            .set("HitCirclePrefix", "default");
    }

    let serialized_ini = skin_ini.to_string();

    mixer::export_osk(
        files_dir,
        installed_skins,
        plan,
        Some(&serialized_ini),
        output_path,
    )
    .map_err(|e| format!("Failed to create .osk archive: {e}"))?;

    let filename_display = output_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    match launch_target {
        LaunchTarget::None => Ok(format!("Exported '{filename_display}'")),
        LaunchTarget::Lazer => {
            let _ = mixer::launch_osk_with_target(output_path, LaunchTarget::Lazer, stable_root);
            Ok(format!(
                "Exported '{filename_display}' and opened in osu!lazer"
            ))
        }
        LaunchTarget::Stable => {
            let _ = mixer::launch_osk_with_target(output_path, LaunchTarget::Stable, stable_root);
            Ok(format!(
                "Exported '{filename_display}' and opened in osu!stable"
            ))
        }
    }
}
