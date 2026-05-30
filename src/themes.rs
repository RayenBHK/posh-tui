use serde::Deserialize;
use std::path::PathBuf;
use crate::error::Result;

const GITHUB_API: &str =
    "https://api.github.com/repos/JanDeDobbeleer/oh-my-posh/contents/themes";

const RAW_BASE: &str =
    "https://raw.githubusercontent.com/JanDeDobbeleer/oh-my-posh/main/themes";

#[derive(Deserialize, Debug)]
struct GithubEntry {
    name: String,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub name:     String,
    pub filename: String,
    pub raw_url:  String,
    pub local:    Option<PathBuf>,
}

impl Theme {
    pub fn cache_path(&self, cache_dir: &PathBuf) -> PathBuf {
        cache_dir.join(&self.filename)
    }
}

pub async fn fetch_theme_list() -> Result<Vec<Theme>> {
    let client = reqwest::Client::builder()
        .user_agent("posh-tui/0.1")
        .build()?;

    let entries: Vec<GithubEntry> = client
        .get(GITHUB_API)
        .send()
        .await?
        .json()
        .await?;

    let themes = entries
        .into_iter()
        .filter(|e| e.name.ends_with(".omp.json"))
        .map(|e| {
            let name = e.name.replace(".omp.json", "");
            let raw_url = format!("{RAW_BASE}/{}", e.name);
            Theme { name, filename: e.name, raw_url, local: None }
        })
        .collect();

    Ok(themes)
}

pub async fn download_theme(theme: &Theme, cache_dir: &PathBuf) -> Result<PathBuf> {
    let dest = theme.cache_path(cache_dir);
    if dest.exists() {
        return Ok(dest);
    }

    let client = reqwest::Client::builder()
        .user_agent("posh-tui/0.1")
        .build()?;

    let bytes = client
        .get(&theme.raw_url)
        .send()
        .await?
        .bytes()
        .await?;

    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(&dest, &bytes)?;

    Ok(dest)
}