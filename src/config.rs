use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::core::error::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ccusage: CcUsageConfig,
    pub server: ServerConfig,
    #[serde(default = "default_block_duration_hours")]
    pub block_duration_hours: i64,
}

fn default_block_duration_hours() -> i64 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcUsageConfig {
    pub runner: String,
    #[serde(rename = "packageSpec")]
    pub package_spec: String,
    #[serde(rename = "extraArgs", default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ccusage: CcUsageConfig {
                runner: String::new(),
                package_spec: "ccusage".to_string(),
                extra_args: vec!["--offline".to_string()],
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3766,
            },
            block_duration_hours: 5,
        }
    }
}

impl AppConfig {
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self, ConfigError> {
        let path = match config_path {
            Some(p) => p.clone(),
            None => Self::default_config_path()?,
        };

        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| ConfigError::ReadError {
                path: path.display().to_string(),
                source: e,
            })?;
            let config: AppConfig =
                toml::from_str(&content).map_err(|e| ConfigError::ParseError {
                    path: path.display().to_string(),
                    source: e,
                })?;
            Ok(config)
        } else {
            let config = AppConfig::default();
            config.save(&path)?;
            Ok(config)
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteError {
                path: path.display().to_string(),
                source: e,
            })?;
        }
        let content = toml::to_string_pretty(self).map_err(|_e| ConfigError::SerializeError)?;
        std::fs::write(path, content).map_err(|e| ConfigError::WriteError {
            path: path.display().to_string(),
            source: e,
        })?;
        Ok(())
    }

    fn default_config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(config_dir.join("llm-usage-dashboard").join("config.toml"))
    }

    pub fn config_path_display() -> String {
        Self::default_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }
}
