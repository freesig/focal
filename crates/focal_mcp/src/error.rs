use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read config {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse YAML config {path}: {source}")]
    ParseConfig {
        path: PathBuf,
        source: yaml_serde::Error,
    },
    #[error("failed to serialize config: {source}")]
    SerializeConfig { source: yaml_serde::Error },
    #[error("failed to create config directory {path}: {source}")]
    CreateConfigDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write config {path}: {source}")]
    WriteConfig {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
    #[error("invalid server config: {0}")]
    InvalidConfig(String),
    #[error("core graph error: {0}")]
    Core(#[from] focal_core::Error),
    #[error("MCP server initialization failed: {0}")]
    McpInitialize(#[source] Box<rmcp::service::ServerInitializeError>),
    #[error("MCP server task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "sqlite")]
    #[error("sqlite open failed: {0}")]
    SqliteOpen(#[from] rusqlite::Error),
}

impl From<rmcp::service::ServerInitializeError> for Error {
    fn from(source: rmcp::service::ServerInitializeError) -> Self {
        Self::McpInitialize(Box::new(source))
    }
}
