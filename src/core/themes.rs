use crate::core::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RAW_BASE: &str = "https://raw.githubusercontent.com/JanDeDobbeleer/oh-my-posh/main/themes";

#[derive(Deserialize, Debug)]
struct GithubEntry {
    name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub filename: String,
    pub raw_url: String,
    pub local: Option<PathBuf>,
}

impl Theme {
    pub fn cache_path(&self, cache_dir: &Path) -> PathBuf {
        cache_dir.join(&self.filename)
    }
}

pub async fn fetch_theme_list() -> Result<Vec<Theme>> {
    let delays = [1u64, 2, 4]; // seconds between retries
    let mut last_err = None;

    for (attempt, &delay_secs) in delays.iter().enumerate() {
        match try_fetch_theme_list().await {
            Ok(list) => return Ok(list),
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("[posh-tui] fetch attempt {} failed: {}", attempt + 1, e);
                last_err = Some(e);
                if attempt < delays.len() - 1 {
                    #[cfg(test)]
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_secs)).await;
                    #[cfg(not(test))]
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    Err(last_err.unwrap())
}

async fn try_fetch_theme_list() -> Result<Vec<Theme>> {
    let client = reqwest::Client::builder()
        .user_agent("posh-tui/0.1")
        .build()?;

    let api_url =
        std::env::var("GITHUB_API_URL").unwrap_or_else(|_| "https://api.github.com".to_string());
    let url = format!("{api_url}/repos/JanDeDobbeleer/oh-my-posh/contents/themes");

    let entries: Vec<GithubEntry> = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let themes = entries
        .into_iter()
        .filter(|e| e.name.ends_with(".omp.json"))
        .map(|e| {
            let name = e.name.replace(".omp.json", "");
            let raw_url = format!("{RAW_BASE}/{}", e.name);
            Theme {
                name,
                filename: e.name,
                raw_url,
                local: None,
            }
        })
        .collect();

    Ok(themes)
}

pub async fn download_theme(theme: &Theme, cache_dir: &Path) -> Result<PathBuf> {
    let dest = theme.cache_path(cache_dir);
    if dest.exists() {
        return Ok(dest);
    }

    let client = reqwest::Client::builder()
        .user_agent("posh-tui/0.1")
        .build()?;

    let bytes = client.get(&theme.raw_url).send().await?.bytes().await?;

    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(&dest, &bytes)?;

    Ok(dest)
}

fn theme_list_cache_path(cache_dir: &Path) -> PathBuf {
    cache_dir
        .parent()
        .unwrap_or(cache_dir)
        .join("themes_cache.json")
}

pub fn save_theme_list_cache(themes: &[Theme], cache_dir: &Path) {
    let path = theme_list_cache_path(cache_dir);
    if let Ok(json) = serde_json::to_string_pretty(themes) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn load_cached_theme_list(cache_dir: &Path) -> Option<Vec<Theme>> {
    let path = theme_list_cache_path(cache_dir);
    if !path.exists() {
        return None;
    }
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn test_fetch_theme_list_success_after_2_errors() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mock_server = MockServer::start().await;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let custom_responder = move |_req: &wiremock::Request| {
            let attempt = counter_clone.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                ResponseTemplate::new(500)
            } else {
                ResponseTemplate::new(200).set_body_json(vec![
                    serde_json::json!({"name": "test1.omp.json"}),
                    serde_json::json!({"name": "test2.omp.json"}),
                ])
            }
        };

        Mock::given(method("GET"))
            .and(path("/repos/JanDeDobbeleer/oh-my-posh/contents/themes"))
            .respond_with(custom_responder)
            .expect(3)
            .mount(&mock_server)
            .await;

        std::env::set_var("GITHUB_API_URL", &mock_server.uri());

        let result = fetch_theme_list().await;
        assert!(result.is_ok());
        let themes = result.unwrap();
        assert_eq!(themes.len(), 2);
        assert_eq!(themes[0].name, "test1");
        assert_eq!(themes[1].name, "test2");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_fetch_theme_list_exhaustion() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/JanDeDobbeleer/oh-my-posh/contents/themes"))
            .respond_with(ResponseTemplate::new(500))
            .expect(3)
            .mount(&mock_server)
            .await;

        std::env::set_var("GITHUB_API_URL", &mock_server.uri());

        let result = fetch_theme_list().await;
        assert!(result.is_err());
    }
}
