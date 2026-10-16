use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{io_err, CoreError, CoreResult};
use crate::process_guard::ensure_game_not_running;

/// Конверт ответа моста — зеркалит `BridgeResult` из C# (Этап 1).
#[derive(Debug, Deserialize)]
struct BridgeEnvelope<T> {
    success: bool,
    error: Option<String>,
    // Поле типа Option<T> serde и так считает необязательным при отсутствии
    // ключа в JSON — отдельный #[serde(default)] тут не нужен и, что важнее,
    // заставлял бы derive-макрос требовать T: Default без реальной необходимости.
    data: Option<T>,
}

/// Одна запись файла скина — зеркалит `SkinFileDto`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinFile {
    pub filename: String,
    pub file_hash: String,
}

/// Скин из базы — зеркалит `SkinDto`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub creator: String,
    #[serde(rename = "protected")]
    pub is_protected: bool,
    pub instantiation_info: String,
    pub hash: String,
    pub delete_pending: bool,
    pub files: Vec<SkinFile>,
}

/// Операция патча — зеркалит `PatchOperation`. Строится через конструкторы
/// ниже, чтобы вызывающий код не мог собрать невалидную комбинацию полей.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchOperation {
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
}

impl PatchOperation {
    pub fn duplicate_skin(
        source_id: impl Into<String>,
        new_id: impl Into<String>,
        name: impl Into<String>,
        creator: impl Into<String>,
    ) -> Self {
        Self {
            op: "duplicate_skin".to_string(),
            source_id: Some(source_id.into()),
            new_id: Some(new_id.into()),
            name: Some(name.into()),
            creator: Some(creator.into()),
            skin_id: None,
            id: None,
            filename: None,
            file_hash: None,
        }
    }

    pub fn add_file(
        skin_id: impl Into<String>,
        filename: impl Into<String>,
        file_hash: impl Into<String>,
    ) -> Self {
        Self {
            op: "add_file".to_string(),
            source_id: None,
            new_id: None,
            name: None,
            creator: None,
            skin_id: Some(skin_id.into()),
            id: None,
            filename: Some(filename.into()),
            file_hash: Some(file_hash.into()),
        }
    }

    pub fn remove_file(skin_id: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            op: "remove_file".to_string(),
            source_id: None,
            new_id: None,
            name: None,
            creator: None,
            skin_id: Some(skin_id.into()),
            id: None,
            filename: Some(filename.into()),
            file_hash: None,
        }
    }

    pub fn soft_delete_skin(id: impl Into<String>) -> Self {
        Self {
            op: "soft_delete_skin".to_string(),
            source_id: None,
            new_id: None,
            name: None,
            creator: None,
            skin_id: None,
            id: Some(id.into()),
            filename: None,
            file_hash: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct PatchFile {
    operations: Vec<PatchOperation>,
}

/// Обёртка над бинарником `realm-bridge`. Гарантирует: перед любой операцией,
/// изменяющей базу, — проверку "игра не запущена" и бэкап `client.realm`.
pub struct RealmBridge {
    bridge_binary: PathBuf,
}

impl RealmBridge {
    pub fn new(bridge_binary: impl Into<PathBuf>) -> CoreResult<Self> {
        let bridge_binary = bridge_binary.into();
        if !bridge_binary.is_file() {
            return Err(CoreError::BridgeBinaryMissing(bridge_binary));
        }
        Ok(Self { bridge_binary })
    }

    /// Только чтение — бэкап и проверка процесса не требуются (мост открывает
    /// базу read-only с точки зрения намерений вызывающей стороны, хотя формально
    /// Realm всё равно открывает файл на запись под капотом; риска порчи данных
    /// при простом чтении принципиально ниже, чем при патче).
    pub fn dump_skins(&self, db_path: &Path) -> CoreResult<Vec<Skin>> {
        let output = self.run(&["dump-skins", "--db", &path_to_arg(db_path)])?;
        parse_envelope(&output)
    }

    /// Изменяющая операция: обязательна проверка "игра закрыта" + бэкап базы
    /// перед вызовом моста. Возвращает путь к созданному бэкапу вместе с результатом.
    pub fn apply_patch(
        &self,
        db_path: &Path,
        operations: Vec<PatchOperation>,
    ) -> CoreResult<ApplyPatchOutcome> {
        ensure_game_not_running()?;
        let backup_path = backup_database(db_path)?;

        let patch = PatchFile { operations };
        let patch_json = serde_json::to_string_pretty(&patch)?;

        let patch_file_path =
            db_path.with_file_name(format!("osu-mixer-patch-{}.json", std::process::id()));
        std::fs::write(&patch_file_path, &patch_json).map_err(io_err(&patch_file_path))?;

        let result = self.run(&[
            "apply-patch",
            "--db",
            &path_to_arg(db_path),
            "--patch",
            &path_to_arg(&patch_file_path),
        ]);

        // Временный файл патча — не нужен после вызова моста вне зависимости от исхода.
        std::fs::remove_file(&patch_file_path).ok();

        let output = result?;
        let applied: serde_json::Value = parse_envelope(&output)?;

        Ok(ApplyPatchOutcome {
            backup_path,
            applied_operations: applied,
        })
    }

    fn run(&self, args: &[&str]) -> CoreResult<String> {
        let output = Command::new(&self.bridge_binary)
            .args(args)
            .output()
            .map_err(CoreError::BridgeSpawnFailed)?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            return Err(CoreError::BridgeNoOutput);
        }
        Ok(stdout)
    }
}

#[derive(Debug)]
pub struct ApplyPatchOutcome {
    pub backup_path: PathBuf,
    pub applied_operations: serde_json::Value,
}

fn parse_envelope<T: for<'de> Deserialize<'de>>(raw_json: &str) -> CoreResult<T> {
    // BridgeEnvelope<T> не выводит Deserialize при отсутствующем `data`, если T не
    // реализует Default — а нам не хочется требовать Default от всех T (например,
    // от Vec<Skin> это бессмысленное ограничение). Поэтому разбираем сначала в
    // envelope с data: Option<serde_json::Value>, а в T приводим уже отдельно.
    let raw: BridgeEnvelope<serde_json::Value> = serde_json::from_str(raw_json)?;
    if !raw.success {
        return Err(CoreError::BridgeReportedError(raw.error.unwrap_or_else(
            || "мост вернул success:false без сообщения об ошибке".to_string(),
        )));
    }
    let data = raw.data.ok_or(CoreError::BridgeNoOutput)?;
    Ok(serde_json::from_value(data)?)
}

fn path_to_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// Копирует `client.realm` и сопутствующие файлы блокировки/журнала Realm
/// (`client.realm.lock`, каталог `client.realm.management/`, `client.realm.note`,
/// если они существуют) в `<имя>.realm.bak` рядом с оригиналом, с меткой времени —
/// чтобы не перетирать предыдущий бэкап при повторных запусках.
fn backup_database(db_path: &Path) -> CoreResult<PathBuf> {
    if !db_path.is_file() {
        return Err(CoreError::BackupFailed(format!(
            "исходная база не найдена: {}",
            db_path.display()
        )));
    }

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = db_path.with_extension(format!("realm.bak-{timestamp}"));

    std::fs::copy(db_path, &backup_path).map_err(io_err(db_path))?;

    // Сопутствующие файлы Realm — копируем best-effort: их отсутствие (например,
    // если база была аккуратно закрыта и журнал уже пуст/удалён) не должно
    // проваливать весь бэкап, раз основной .realm файл уже успешно скопирован.
    for suffix in [".lock", ".note"] {
        let side_file = PathBuf::from(format!("{}{suffix}", db_path.display()));
        if side_file.is_file() {
            let side_backup = PathBuf::from(format!("{}{suffix}", backup_path.display()));
            let _ = std::fs::copy(&side_file, &side_backup);
        }
    }

    let management_dir = PathBuf::from(format!("{}.management", db_path.display()));
    if management_dir.is_dir() {
        let management_backup = PathBuf::from(format!("{}.management", backup_path.display()));
        let _ = copy_dir_recursive(&management_dir, &management_backup);
    }

    Ok(backup_path)
}

fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_is_rejected_at_construction() {
        let result = RealmBridge::new("/definitely/does/not/exist/realm-bridge");
        assert!(matches!(result, Err(CoreError::BridgeBinaryMissing(_))));
    }

    #[test]
    fn backup_copies_main_file_and_timestamps_name() {
        let dir =
            std::env::temp_dir().join(format!("osu-mixer-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("client.realm");
        std::fs::write(&db_path, b"fake realm bytes").unwrap();

        let backup = backup_database(&db_path).unwrap();
        assert!(backup.is_file());
        assert_ne!(backup, db_path);
        assert_eq!(std::fs::read(&backup).unwrap(), b"fake realm bytes");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn patch_operation_serializes_only_relevant_fields() {
        let op = PatchOperation::add_file("skin-id", "cursor.png", "abc123");
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("\"op\":\"add_file\""));
        assert!(json.contains("\"skin_id\":\"skin-id\""));
        assert!(!json.contains("source_id"));
        assert!(!json.contains("new_id"));
    }
}
