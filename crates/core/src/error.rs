use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("failed to find osu!lazer data directory. Checked paths: {tried:?}")]
    LazerPathNotFound { tried: Vec<PathBuf> },

    #[error("client.realm not found in osu!lazer directory: {0}")]
    RealmDatabaseMissing(PathBuf),

    #[error("running osu! process detected (pid={pid}, name='{name}') — database operations are prohibited while game is running")]
    GameIsRunning { pid: u32, name: String },

    #[error("I/O error accessing '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create database backup: {0}")]
    BackupFailed(String),

    #[error("realm-bridge binary not found at: {0}")]
    BridgeBinaryMissing(PathBuf),

    #[error("realm-bridge exited with error: {0}")]
    BridgeReportedError(String),

    #[error("failed to parse realm-bridge output as JSON: {0}")]
    BridgeOutputInvalid(#[from] serde_json::Error),

    #[error("failed to spawn realm-bridge process: {0}")]
    BridgeSpawnFailed(std::io::Error),

    #[error("realm-bridge exited without output in stdout")]
    BridgeNoOutput,
}

pub type CoreResult<T> = Result<T, CoreError>;

/// Wraps std::io::Error with the file path for diagnostics.
pub(crate) fn io_err(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> CoreError {
    let path = path.into();
    move |source| CoreError::Io { path, source }
}
