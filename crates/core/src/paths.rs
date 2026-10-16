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
}
