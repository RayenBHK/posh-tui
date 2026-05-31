use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashSet;
use crate::error::{PoshError, Result};

const CONFIG_FILE: &str = "config.toml";
const APP_DIR:     &str = "posh-tui";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub last_applied: Option<String>,

    #[serde(default)]
    pub favourites: HashSet<String>,

    #[serde(default = "default_zoom")]
    pub zoom_factor: f32,
}

fn default_zoom() -> f32 { 1.0 }

impl Default for Config {
    fn default() -> Self {
        Self {
            last_applied: None,
            favourites:   HashSet::new(),
            zoom_factor:  1.0,
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let dir = dirs_next::config_dir()
            .ok_or_else(|| PoshError::Config("cannot find config directory".into()))?
            .join(APP_DIR);
        Ok(dir.join(CONFIG_FILE))
    }

    pub fn load() -> Self {
        match Self::try_load() {
            Ok(c)  => c,
            Err(_) => Self::default(),
        }
    }

    fn try_load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(PoshError::TomlSer)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}
