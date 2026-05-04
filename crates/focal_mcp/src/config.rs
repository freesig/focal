use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

pub const DEFAULT_CONFIG_RELATIVE_PATH: &str = ".config/focal/config.yaml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMode {
    Init,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendConfig {
    Fs {
        root: PathBuf,
        mode: OpenMode,
    },
    #[cfg(feature = "sqlite")]
    Sqlite {
        database_path: PathBuf,
        graph_name: String,
        mode: OpenMode,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransportConfig {
    Stdio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub backend: BackendConfig,
    pub transport: TransportConfig,
}

pub fn default_config_path() -> Result<PathBuf, Error> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        Error::InvalidConfig(
            "HOME is not set; pass --config or set HOME to use the default config path".to_string(),
        )
    })?;
    if home.is_empty() {
        return Err(Error::InvalidConfig(
            "HOME is empty; pass --config or set HOME to use the default config path".to_string(),
        ));
    }
    Ok(config_path_for_home(PathBuf::from(home)))
}

pub fn load_config(path: impl AsRef<Path>) -> Result<ServerConfig, Error> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| Error::ReadConfig {
        path: path.to_path_buf(),
        source,
    })?;
    yaml_serde::from_str(&contents).map_err(|source| Error::ParseConfig {
        path: path.to_path_buf(),
        source,
    })
}

pub fn save_config(path: impl AsRef<Path>, config: &ServerConfig) -> Result<(), Error> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| Error::CreateConfigDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let contents =
        yaml_serde::to_string(config).map_err(|source| Error::SerializeConfig { source })?;
    fs::write(path, contents).map_err(|source| Error::WriteConfig {
        path: path.to_path_buf(),
        source,
    })
}

fn config_path_for_home(home: PathBuf) -> PathBuf {
    home.join(DEFAULT_CONFIG_RELATIVE_PATH)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn default_config_path_uses_home_config_focal_yaml() {
        assert_eq!(
            config_path_for_home(PathBuf::from("/home/example")),
            PathBuf::from("/home/example/.config/focal/config.yaml")
        );
    }

    #[test]
    fn save_config_creates_parent_directories_and_round_trips()
    -> Result<(), Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let config_path = tempdir.path().join("nested/config.yaml");
        let config = ServerConfig {
            backend: BackendConfig::Fs {
                root: PathBuf::from("/tmp/focal-ideas"),
                mode: OpenMode::Init,
            },
            transport: TransportConfig::Stdio,
        };

        save_config(&config_path, &config)?;
        let loaded = load_config(&config_path)?;

        assert_eq!(loaded, config);
        Ok(())
    }
}
