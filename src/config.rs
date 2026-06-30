use crate::error::{PoshError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const CONFIG_FILE: &str = "config.toml";
const APP_DIR: &str = "posh-tui";
const MAX_RECENT: usize = 10;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub hide_font_warning: bool,

    #[serde(default)]
    pub last_applied: Option<String>,

    #[serde(default)]
    pub favourites: HashSet<String>,

    #[serde(default = "default_zoom")]
    pub zoom_factor: f32,

    #[serde(default)]
    pub recent: Vec<String>,
}

fn default_zoom() -> f32 {
    1.0
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hide_font_warning: false,
            last_applied: None,
            favourites: HashSet::new(),
            zoom_factor: 1.0,
            recent: Vec::new(),
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
        Self::try_load().unwrap_or_default()
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

        let contents = toml::to_string_pretty(self).map_err(PoshError::TomlSer)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn push_recent(&mut self, name: &str) {
        self.recent.retain(|n| n != name);
        self.recent.insert(0, name.to_string());
        self.recent.truncate(MAX_RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_on_missing_file() {
        // Config::load() must not panic even if no config file exists on disk
        let cfg = Config::load();
        // zoom_factor defaults to 1.0 which is a sensible positive value
        assert!(cfg.zoom_factor > 0.0);
    }

    #[test]
    fn test_push_recent_dedup() {
        let mut cfg = Config::load();
        cfg.recent.clear();
        cfg.push_recent("catppuccin");
        cfg.push_recent("catppuccin");
        let count = cfg
            .recent
            .iter()
            .filter(|n| n.as_str() == "catppuccin")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_push_recent_truncate() {
        let mut cfg = Config::load();
        cfg.recent.clear();
        for i in 0..12 {
            cfg.push_recent(&format!("theme-{}", i));
        }
        assert!(cfg.recent.len() <= MAX_RECENT);
    }

    #[test]
    fn test_push_recent_order() {
        let mut cfg = Config::load();
        cfg.recent.clear();
        cfg.push_recent("a");
        cfg.push_recent("b");
        // most recently pushed item should be at the front
        assert_eq!(cfg.recent.first().map(|s| s.as_str()), Some("b"));
    }
}
