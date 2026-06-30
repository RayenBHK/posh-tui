use std::fmt;

#[derive(Debug)]
pub enum PoshError {
    Io(std::io::Error),
    Http(reqwest::Error),
    Json(serde_json::Error),
    Toml(toml::de::Error),
    TomlSer(toml::ser::Error),
    #[allow(dead_code)]
    Preview(String),
    Shell(String),
    Config(String),
}

impl fmt::Display for PoshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoshError::Io(e) => write!(f, "IO error: {e}"),
            PoshError::Http(e) => write!(f, "HTTP error: {e}"),
            PoshError::Json(e) => write!(f, "JSON error: {e}"),
            PoshError::Toml(e) => write!(f, "Config read error: {e}"),
            PoshError::TomlSer(e) => write!(f, "Config write error: {e}"),
            PoshError::Preview(s) => write!(f, "Preview error: {s}"),
            PoshError::Shell(s) => write!(f, "Shell error: {s}"),
            PoshError::Config(s) => write!(f, "Config error: {s}"),
        }
    }
}

impl From<std::io::Error> for PoshError {
    fn from(e: std::io::Error) -> Self {
        PoshError::Io(e)
    }
}
impl From<reqwest::Error> for PoshError {
    fn from(e: reqwest::Error) -> Self {
        PoshError::Http(e)
    }
}
impl From<serde_json::Error> for PoshError {
    fn from(e: serde_json::Error) -> Self {
        PoshError::Json(e)
    }
}
impl From<toml::de::Error> for PoshError {
    fn from(e: toml::de::Error) -> Self {
        PoshError::Toml(e)
    }
}
impl From<toml::ser::Error> for PoshError {
    fn from(e: toml::ser::Error) -> Self {
        PoshError::TomlSer(e)
    }
}

pub type Result<T> = std::result::Result<T, PoshError>;
