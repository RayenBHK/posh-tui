#[derive(Debug, thiserror::Error)]
pub enum PoshError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Config read error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Config write error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Preview error: {0}")]
    Preview(String),

    #[error("Shell error: {0}")]
    Shell(String),

    #[error("Config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, PoshError>;
