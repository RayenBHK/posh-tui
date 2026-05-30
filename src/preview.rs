use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::error::Result;

#[derive(Debug)]
pub enum PreviewMsg {
    Load(PathBuf, u16),  // path + terminal width
    Quit,
}

pub struct PreviewWorker {
    pub tx:     mpsc::Sender<PreviewMsg>,
    pub output: Arc<Mutex<Option<String>>>,
}

impl PreviewWorker {
    pub fn spawn() -> Self {
        let (tx, mut rx) = mpsc::channel::<PreviewMsg>(32);
        let output = Arc::new(Mutex::new(None::<String>));
        let output_clone = Arc::clone(&output);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    PreviewMsg::Quit => break,
                    PreviewMsg::Load(path, width) => {
                        let result = render_preview(&path, width).await;
                        let mut lock = output_clone.lock().await;
                        *lock = Some(match result {
                            Ok(s)  => s,
                            Err(e) => format!("  preview error: {e}"),
                        });
                    }
                }
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

    let output = tokio::process::Command::new("oh-my-posh")
        .args(["print", "primary", "--config"])
        .arg(path)
        .arg("--shell").arg("bash")
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("COLUMNS", &cols)      // tells omp exactly how wide to render
        .env("LINES", "10")
        .output()
        .await?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    if raw.trim().is_empty() {
        return Ok("  (empty output — try another theme)".to_string());
    }

    let cleaned = raw
        .replace("\\[", "")
        .replace("\\]", "");

    Ok(cleaned)
}