use crate::core::error::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
pub enum PreviewMsg {
    Load(PathBuf, u16), // path + terminal width
}

pub struct PreviewWorker {
    pub tx: mpsc::Sender<PreviewMsg>,
    pub output: Arc<Mutex<Option<String>>>,
}

impl PreviewWorker {
    pub fn spawn() -> Self {
        let (tx, mut rx) = mpsc::channel::<PreviewMsg>(32);
        let output = Arc::new(Mutex::new(None::<String>));
        let output_clone = Arc::clone(&output);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let PreviewMsg::Load(path, width) = msg;
                let result = render_preview(&path, width).await;
                let mut lock = output_clone.lock().await;
                // PoshError Display already includes a readable label, so we
                // just format the error directly — avoids "preview error: Preview error: …"
                *lock = Some(match result {
                    Ok(s) => s,
                    Err(e) => format!("  preview error: {e}"),
                });
            }
        });

        Self { tx, output }
    }

    pub async fn request(&self, path: PathBuf, width: u16) {
        let _ = self.tx.send(PreviewMsg::Load(path, width)).await;
    }

    pub async fn take_output(&self) -> Option<String> {
        let mut lock = self.output.lock().await;
        lock.take()
    }
}

async fn render_preview(path: &PathBuf, width: u16) -> Result<String> {
    // subtract 2 for the border chars so content fits perfectly inside the pane
    let cols = width.saturating_sub(2).to_string();

    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        tokio::process::Command::new("oh-my-posh")
            .args(["print", "primary", "--config"])
            .arg(path)
            .arg("--shell")
            .arg("bash")
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .env("COLUMNS", &cols)
            .env("LINES", "10")
            // oh-my-posh caches the active config path in /tmp/bash.<POSH_SESSION_ID>.omp.cache.
            // When posh-tui inherits the parent shell's session ID, every `print primary --config X`
            // call silently uses the cached path (the last applied theme) instead of X.
            // Clearing the session ID forces a fresh session with no config cache.
            .env_remove("POSH_SESSION_ID")
            // Also clear POSH_THEME in case the parent shell has it exported — it would
            // override --config in some oh-my-posh versions.
            .env_remove("POSH_THEME")
            .output(),
    )
    .await
    .map_err(|_| crate::core::error::PoshError::Preview("oh-my-posh timed out after 5s".into()))?
    ?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    if raw.trim().is_empty() {
        return Ok("  (empty output — try another theme)".to_string());
    }

    let cleaned = raw.replace("\\[", "").replace("\\]", "");

    Ok(cleaned)
}
