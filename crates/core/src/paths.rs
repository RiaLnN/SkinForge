use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};

/// Detected osu!lazer installation: data root directory, content-addressed files/
/// storage subfolder, and client.realm database path.
#[derive(Debug, Clone)]
pub struct LazerInstallation {
    pub root: PathBuf,
    pub files_dir: PathBuf,
    pub client_realm: PathBuf,
}

impl LazerInstallation {
    fn from_root(root: PathBuf) -> Self {
        Self {
            files_dir: root.join("files"),
            client_realm: root.join("client.realm"),
            root,
        }
    }
}

/// Automatically locates the osu!lazer data directory by searching standard OS paths.
/// Returns the first candidate where `client.realm` is present on disk.
pub fn resolve_lazer_root() -> CoreResult<LazerInstallation> {
    resolve_lazer_root_with_override(None)
}

/// Resolves the lazer installation root with an optional explicit path override.
pub fn resolve_lazer_root_with_override(
    explicit: Option<PathBuf>,
) -> CoreResult<LazerInstallation> {
    if let Some(path) = explicit {
        return validate_root(path);
    }

    let candidates = candidate_roots();
    for candidate in &candidates {
        if candidate.join("client.realm").is_file() {
            return Ok(LazerInstallation::from_root(candidate.clone()));
        }
    }

    Err(CoreError::LazerPathNotFound { tried: candidates })
}

fn validate_root(root: PathBuf) -> CoreResult<LazerInstallation> {
    let installation = LazerInstallation::from_root(root);
    if !installation.client_realm.is_file() {
        return Err(CoreError::RealmDatabaseMissing(installation.client_realm));
    }
    Ok(installation)
}

/// List of candidate root directories for the current OS in priority order.
fn candidate_roots() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    match std::env::consts::OS {
        "windows" => {
            // %APPDATA%/osu/
            if let Some(appdata) = std::env::var_os("APPDATA") {
                out.push(PathBuf::from(appdata).join("osu"));
            }
        }
        "macos" => {
            if let Some(home) = dirs::home_dir() {
                out.push(home.join("Library/Application Support/osu"));
            }
        }
        _ => {
            // Linux and unix-like systems:
            // 1) Standard XDG_DATA_HOME path (~/.local/share/osu)
            if let Some(data_dir) = dirs::data_dir() {
                out.push(data_dir.join("osu"));
            }
            // 2) Explicit ~/.local/share/osu fallback
            if let Some(home) = dirs::home_dir() {
                out.push(home.join(".local/share/osu"));
                // 3) Flatpak sandbox (sh.ppy.osu)
                out.push(home.join(".var/app/sh.ppy.osu/data/osu"));
            }
        }
    }

    dedup_preserve_order(out)
}

fn dedup_preserve_order(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// Returns the path to a hashed storage item: `files/<h0>/<h0h1>/<hash>`.
/// This matches osu!lazer's content-addressed storage layout.
pub fn hashed_storage_path(files_dir: &Path, hash_hex: &str) -> PathBuf {
    let first = &hash_hex[0..1];
    let first_two = &hash_hex[0..2];
    files_dir.join(first).join(first_two).join(hash_hex)
}

/// Locates the compiled `realm-bridge` binary next to the current executable,
/// inside a `bridge/` subdirectory, in standard .NET output directories, or in working directory.
pub fn find_realm_bridge_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Bundled in package root
            candidates.push(dir.join("realm-bridge"));
            candidates.push(dir.join("realm-bridge.exe"));
            // Bundled in bridge/ subdirectory (standard distribution layout)
            candidates.push(dir.join("bridge/realm-bridge"));
            candidates.push(dir.join("bridge/realm-bridge.exe"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("bridge/realm-bridge"));
        candidates.push(cwd.join("bridge/realm-bridge.exe"));
        candidates.push(cwd.join("realm-bridge/bin/Release/net8.0/realm-bridge"));
        candidates.push(cwd.join("realm-bridge/bin/Debug/net8.0/realm-bridge"));
        candidates.push(cwd.join("realm-bridge/bin/Release/net8.0/realm-bridge.exe"));
        candidates.push(cwd.join("realm-bridge/bin/Debug/net8.0/realm-bridge.exe"));
        candidates.push(cwd.join("../realm-bridge/bin/Release/net8.0/realm-bridge"));
        candidates.push(cwd.join("../realm-bridge/bin/Debug/net8.0/realm-bridge"));
    }

    candidates.into_iter().find(|c| c.is_file())
}

/// Detected osu!stable installation: root directory and `Skins/` subdirectory.
#[derive(Debug, Clone)]
pub struct StableInstallation {
    pub root: PathBuf,
    pub skins_dir: PathBuf,
}

/// Automatically locates an installed `osu!stable` directory by searching standard OS paths.
pub fn resolve_stable_root() -> Option<StableInstallation> {
    resolve_stable_root_with_override(None)
}

/// Resolves the `osu!stable` installation root (or direct `Skins` directory) with an optional override.
pub fn resolve_stable_root_with_override(explicit: Option<PathBuf>) -> Option<StableInstallation> {
    if let Some(path) = explicit {
        return validate_stable_root(path);
    }

    for candidate in stable_candidate_roots() {
        if let Some(inst) = validate_stable_root(candidate) {
            return Some(inst);
        }
    }

    None
}

fn validate_stable_root(path: PathBuf) -> Option<StableInstallation> {
    if !path.is_dir() {
        return None;
    }
    // Check if the user selected the `Skins` directory directly
    if path
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("Skins"))
    {
        let root = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.clone());
        return Some(StableInstallation {
            root,
            skins_dir: path,
        });
    }
    // Check for `Skins` or `skins` subdirectory
    for sub in ["Skins", "skins"] {
        let candidate = path.join(sub);
        if candidate.is_dir() {
            return Some(StableInstallation {
                root: path,
                skins_dir: candidate,
            });
        }
    }
    None
}

fn stable_candidate_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Some(env_path) = std::env::var_os("OSU_STABLE_PATH") {
        out.push(PathBuf::from(env_path));
    }

    match std::env::consts::OS {
        "windows" => {
            if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
                out.push(PathBuf::from(local_appdata).join("osu!"));
            }
            if let Some(program_files) = std::env::var_os("ProgramFiles") {
                out.push(PathBuf::from(program_files).join("osu!"));
            }
            if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                out.push(PathBuf::from(program_files_x86).join("osu!"));
            }
            out.push(PathBuf::from(r"C:\osu!"));
            out.push(PathBuf::from(r"D:\osu!"));
            out.push(PathBuf::from(r"E:\osu!"));
        }
        "macos" => {
            out.push(PathBuf::from(
                "/Applications/osu!.app/Contents/Resources/drive_c/osu!",
            ));
            if let Some(home) = dirs::home_dir() {
                out.push(home.join("Library/Application Support/osu-stable"));
                out.push(home.join("Games/osu!"));
            }
        }
        _ => {
            if let Some(home) = dirs::home_dir() {
                out.push(home.join(".local/share/osu-wine/osu!"));
                out.push(home.join(".local/share/osu-stable"));
                out.push(home.join("Games/osu!"));
                out.push(home.join(".wine/drive_c/osu!"));
                if let Ok(user) = std::env::var("USER") {
                    out.push(
                        home.join(".wine/drive_c/users")
                            .join(&user)
                            .join("AppData/Local/osu!"),
                    );
                    out.push(
                        home.join("Games/osu/drive_c/users")
                            .join(&user)
                            .join("AppData/Local/osu!"),
                    );
                }
            }
            if let Some(data_dir) = dirs::data_dir() {
                out.push(data_dir.join("osu-wine/osu!"));
                out.push(data_dir.join("osu-stable"));
            }
        }
    }

    dedup_preserve_order(out)
}

/// Scans an `osu!stable` `Skins/` directory and returns all valid skins as `Skin` structs
/// with `source: SkinSource::Stable` and `root_dir: Some(skin_folder)`.
pub fn scan_stable_skins(skins_dir: &Path) -> Vec<crate::bridge::Skin> {
    let Ok(read_dir) = std::fs::read_dir(skins_dir) else {
        return Vec::new();
    };

    let mut folder_paths: Vec<PathBuf> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
        .map(|e| e.path())
        .collect();

    folder_paths.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .cmp(
                &b.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase(),
            )
    });

    let mut out = Vec::new();

    for folder in folder_paths {
        let folder_name = folder
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if folder_name.is_empty() || folder_name.starts_with('.') {
            continue;
        }

        let mut files = Vec::new();
        let mut ini_path: Option<PathBuf> = None;

        for entry in walkdir::WalkDir::new(&folder).min_depth(1).max_depth(3) {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&folder)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");

            if ini_path.is_none() && rel.eq_ignore_ascii_case("skin.ini") {
                ini_path = Some(entry.path().to_path_buf());
            }

            files.push(crate::bridge::SkinFile {
                file_hash: format!("stable:{folder_name}/{rel}"),
                filename: rel,
            });
        }

        if files.is_empty() {
            continue;
        }

        let mut creator = String::new();
        if let Some(ini_file) = ini_path {
            if let Ok(parsed) = crate::skin_ini::SkinIni::read_from_file(&ini_file) {
                if let Some(author) = parsed.skin_author() {
                    creator = author.to_string();
                }
            }
        }

        out.push(crate::bridge::Skin {
            id: format!("stable:{folder_name}"),
            name: folder_name,
            creator,
            is_protected: false,
            instantiation_info: String::new(),
            hash: String::new(),
            delete_pending: false,
            files,
            source: crate::bridge::SkinSource::Stable,
            root_dir: Some(folder),
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_with_existing_realm_succeeds() {
        let dir = std::env::temp_dir().join(format!("skinforge-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("client.realm"), b"fake").unwrap();

        let result = resolve_lazer_root_with_override(Some(dir.clone()));
        assert!(result.is_ok());
        let installation = result.unwrap();
        assert_eq!(installation.client_realm, dir.join("client.realm"));
        assert_eq!(installation.files_dir, dir.join("files"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn override_without_realm_fails() {
        let dir = std::env::temp_dir().join(format!("skinforge-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let result = resolve_lazer_root_with_override(Some(dir.clone()));
        assert!(matches!(result, Err(CoreError::RealmDatabaseMissing(_))));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hashed_storage_path_matches_lazer_layout() {
        let files_dir = PathBuf::from("/tmp/files");
        let path = hashed_storage_path(&files_dir, "3a7fbe21...");
        assert_eq!(path, PathBuf::from("/tmp/files/3/3a/3a7fbe21..."));
    }

    #[test]
    fn candidate_roots_nonempty() {
        assert!(!candidate_roots().is_empty());
    }

    #[test]
    fn scan_stable_skins_discovers_folders_and_reads_skin_ini_author() {
        let dir =
            std::env::temp_dir().join(format!("skinforge-stable-scan-test-{}", std::process::id()));
        let skins_dir = dir.join("Skins");
        let skin_a = skins_dir.join("-+ WhiteCat (1.0)");
        let skin_b = skins_dir.join("EmptyFolder");
        std::fs::create_dir_all(&skin_a).unwrap();
        std::fs::create_dir_all(&skin_b).unwrap();

        std::fs::write(
            skin_a.join("skin.ini"),
            "[General]\nName: WhiteCat 1.0\nAuthor: CK\n",
        )
        .unwrap();
        std::fs::write(skin_a.join("cursor.png"), b"png").unwrap();

        let inst = resolve_stable_root_with_override(Some(dir.clone())).unwrap();
        assert_eq!(inst.skins_dir, skins_dir);

        let skins = scan_stable_skins(&inst.skins_dir);
        assert_eq!(skins.len(), 1);
        assert_eq!(skins[0].name, "-+ WhiteCat (1.0)");
        assert_eq!(skins[0].creator, "CK");
        assert_eq!(skins[0].source, crate::bridge::SkinSource::Stable);
        assert_eq!(skins[0].badge_label(), "stable");
        assert_eq!(skins[0].root_dir.as_deref(), Some(skin_a.as_path()));

        let cursor_file = skins[0]
            .files
            .iter()
            .find(|f| f.filename == "cursor.png")
            .unwrap();
        assert_eq!(
            skins[0].resolve_file_path(None, cursor_file).as_deref(),
            Some(skin_a.join("cursor.png").as_path())
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
