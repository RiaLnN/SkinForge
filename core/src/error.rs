use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("не удалось найти директорию данных osu!lazer. Проверены пути: {tried:?}")]
    LazerPathNotFound { tried: Vec<PathBuf> },

    #[error("в директории lazer не найден client.realm: {0}")]
    RealmDatabaseMissing(PathBuf),

    #[error("обнаружен запущенный процесс osu! (pid={pid}, имя='{name}') — операции с базой запрещены, пока игра не закрыта")]
    GameIsRunning { pid: u32, name: String },

    #[error("ошибка ввода-вывода при работе с '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("не удалось создать резервную копию базы: {0}")]
    BackupFailed(String),

    #[error("бинарник realm-bridge не найден по пути: {0}")]
    BridgeBinaryMissing(PathBuf),

    #[error("realm-bridge завершился с ошибкой: {0}")]
    BridgeReportedError(String),

    #[error("не удалось разобрать вывод realm-bridge как JSON: {0}")]
    BridgeOutputInvalid(#[from] serde_json::Error),

    #[error("не удалось запустить процесс realm-bridge: {0}")]
    BridgeSpawnFailed(std::io::Error),

    #[error("realm-bridge завершился без вывода в stdout")]
    BridgeNoOutput,
}

pub type CoreResult<T> = Result<T, CoreError>;

/// Оборачивает std::io::Error, добавляя путь файла для диагностики.
pub(crate) fn io_err(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> CoreError {
    let path = path.into();
    move |source| CoreError::Io { path, source }
}
