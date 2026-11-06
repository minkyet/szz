use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SzzError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Manifest error: {0}")]
    Manifest(String),

    #[error("Network error requesting {url}: {source}")]
    Network {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Fabric error: {0}")]
    Fabric(String),

    #[error("Launcher error: {0}")]
    Launcher(String),

    #[error("Loader error: {0}")]
    Loader(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Config sync error: {0}")]
    Config(String),

    #[error("Assets error: {0}")]
    Assets(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Instance not installed: {0}")]
    InstanceNotInstalled(String),

    #[error("Instance is currently running: {0}")]
    InstanceRunning(String),

    #[error("Storage garbage collection is currently in progress")]
    StorageGcInProgress,

    #[error("Server is offline: {0}")]
    ServerOffline(String),
}

pub type Result<T> = std::result::Result<T, SzzError>;
