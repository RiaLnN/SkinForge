use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};

/// Найденная установка osu!lazer: корень директории данных, поддиректория
/// файлового хранилища (content-addressed storage) и путь к самой базе.
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

/// Автоматически находит директорию данных lazer, перебирая известные для
/// текущей ОС пути. Возвращает первый кандидат, где реально лежит `client.realm`.
pub fn resolve_lazer_root() -> CoreResult<LazerInstallation> {
    resolve_lazer_root_with_override(None)
}

/// То же самое, но с возможностью явно указать путь (например, из настроек
/// пользователя или флага `--lazer-path`), минуя автодетект.
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

/// Список путей-кандидатов для текущей ОС, в порядке приоритета.
/// Ничего не проверяет на диске — только строит список.
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
            // lazer официально не имеет отдельного macOS-нативного клиента наравне
            // с Linux/Windows на момент написания, но путь добавлен на случай
            // кастомных/будущих сборок — стандартная для macOS директория данных приложений.
            if let Some(home) = dirs::home_dir() {
                out.push(home.join("Library/Application Support/osu"));
            }
        }
        _ => {
            // Linux и большинство unix-like систем.
            // 1) Штатный путь через XDG_DATA_HOME (или ~/.local/share, если переменная не задана).
            if let Some(data_dir) = dirs::data_dir() {
                out.push(data_dir.join("osu"));
            }
            // 2) Явный ~/.local/share/osu — на случай, если XDG_DATA_HOME указывает
            //    на что-то нестандартное, а реальная установка лежит в дефолтном месте.
            if let Some(home) = dirs::home_dir() {
                out.push(home.join(".local/share/osu"));
                // 3) Flatpak-версия (sh.ppy.osu) — своя песочница данных.
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

/// Удобный доступ к директории хранения ресурсов по хэшу: `files/<h0>/<h0h1>/<hash>`.
/// Именно так lazer раскладывает content-addressed файлы (первый символ хэша —
/// один уровень вложенности, первые два символа — второй).
pub fn hashed_storage_path(files_dir: &Path, hash_hex: &str) -> PathBuf {
    let first = &hash_hex[0..1];
    let first_two = &hash_hex[0..2];
    files_dir.join(first).join(first_two).join(hash_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_with_existing_realm_succeeds() {
        let dir = std::env::temp_dir().join(format!("osu-mixer-test-{}", std::process::id()));
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
        let dir = std::env::temp_dir().join(format!("osu-mixer-test-empty-{}", std::process::id()));
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
        // На любой поддерживаемой ОС список кандидатов не должен быть пустым —
        // иначе автодетект в принципе не сможет ничего найти.
        assert!(!candidate_roots().is_empty());
    }
}
