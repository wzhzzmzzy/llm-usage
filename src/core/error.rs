use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Refresh error: {0}")]
    Refresh(#[from] RefreshError),

    #[error("Normalization error: {0}")]
    Normalize(#[from] NormalizeError),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Cannot determine config directory")]
    NoConfigDir,

    #[error("Failed to read config from {path}: {source}")]
    ReadError {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse config from {path}: {source}")]
    ParseError {
        path: String,
        source: toml::de::Error,
    },

    #[error("Failed to write config to {path}: {source}")]
    WriteError {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to serialize config")]
    SerializeError,
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Runner not found: {0}")]
    RunnerNotFound(String),

    #[error("Command failed with exit code {exit_code}: {stderr}")]
    CommandFailed { exit_code: i32, stderr: String },

    #[error("Command timed out after {0} seconds")]
    Timeout(u64),

    #[error("Invalid JSON output: {0}")]
    InvalidJson(String),

    #[error("Unsupported source: {0}")]
    UnsupportedSource(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum RefreshError {
    #[error("Refresh already in progress")]
    AlreadyInProgress,

    #[error("Refresh timed out after {0} seconds")]
    Timeout(u64),

    #[error("All cells failed")]
    AllCellsFailed,

    #[error("No data available")]
    NoData,
}

#[derive(Error, Debug)]
pub enum NormalizeError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Error, Debug)]
pub enum PricingError {
    #[error("Cache not found")]
    CacheNotFound,

    #[error("Cache expired")]
    CacheExpired,

    #[error("Fetch error: {0}")]
    FetchError(String),

    #[error("IO error: {0}")]
    IoError(String),
}
