use crate::preview::PreviewWorker;
// NOTE: This creates an intentional intra-crate coupling: `app` → `ui::ansi_to_text`.
// The function is used here to keep ANSI parsing in the state layer (Model in MUV),
// so that `ui/` only renders pre-parsed `Text<'static>` and never does expensive I/O.
// Rust allows this cycle within a single crate. Future refactor: extract `ansi_to_text`
// to `core/` or expose it via a standalone crate to eliminate the architectural coupling.
use crate::ui::ansi_to_text;
use crossterm::ExecutableCommand;

use super::{ImmKind, ImmLine, Mode};

pub struct PreviewState {
    pub preview_output: String,
    pub cached_preview: Option<ratatui::text::Text<'static>>,
    pub preview_loading: bool,
    pub worker: PreviewWorker,
    pub preview_width: u16,
    pub terminal_width: u16,
    pub scroll_offset: u16,
    pub zoom_factor: f32,
    pub preview_timer: u8,
}

impl super::App {
    pub fn scroll_left(&mut self) {
        self.preview_state.scroll_offset = self.preview_state.scroll_offset.saturating_sub(4);
    }
    pub fn scroll_right(&mut self) {
        self.preview_state.scroll_offset = self.preview_state.scroll_offset.saturating_add(4);
    }

    pub fn zoom_in(&mut self) {
        self.preview_state.zoom_factor = (self.preview_state.zoom_factor - 0.25).max(0.5);
        self.preview_state.scroll_offset = 0;
        self.save_config();
    }

    pub fn zoom_out(&mut self) {
        self.preview_state.zoom_factor = (self.preview_state.zoom_factor + 0.25).min(3.0);
        self.preview_state.scroll_offset = 0;
        self.save_config();
    }

    pub fn zoom_reset(&mut self) {
        self.preview_state.zoom_factor = 1.0;
        self.preview_state.scroll_offset = 0;
        self.save_config();
    }

    pub fn step_preview_timer(&mut self) -> bool {
        if self.preview_state.preview_timer > 0 {
            self.preview_state.preview_timer -= 1;
            self.preview_state.preview_timer == 0
        } else {
            false
        }
    }

    pub fn effective_columns(&self, base_width: u16) -> u16 {
        ((base_width as f32) * self.preview_state.zoom_factor) as u16
    }

    pub async fn edit_theme(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else {
            return;
        };

        let path = self.cache_dir.join(&theme.filename);
        if !path.exists() {
            return;
        }

        let mut stdout = std::io::stdout();
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = stdout.execute(crossterm::terminal::LeaveAlternateScreen);

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());

        if let Err(e) = std::process::Command::new(&editor).arg(&path).status() {
            eprintln!("Failed to run editor ({}): {}", editor, e);
        }

        let _ = crossterm::terminal::enable_raw_mode();
        let _ = stdout.execute(crossterm::terminal::EnterAlternateScreen);

        self.trigger_preview().await;
    }

    pub async fn trigger_preview(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else {
            return;
        };
        self.config.push_recent(&theme.name);
        self.save_config();
        self.preview_state.preview_loading = true;
        self.preview_state.preview_output = String::new();
        self.preview_state.cached_preview = None;

        let dest = self.cache_dir.join(&theme.filename);
        if !dest.exists() {
            match crate::core::themes::download_theme(&theme, &self.cache_dir).await {
                Ok(_) => {}
                Err(e) => {
                    self.preview_state.preview_output = format!("  download error: {e}");
                    self.preview_state.preview_loading = false;
                    return;
                }
            }
        }

        let cols = self.effective_columns(self.preview_state.preview_width);
        self.preview_state.worker.request(dest, cols).await;
    }

    pub async fn trigger_immersive_preview(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else {
            return;
        };
        let dest = self.cache_dir.join(&theme.filename);
        if !dest.exists() {
            let _ = crate::core::themes::download_theme(&theme, &self.cache_dir).await;
        }
        self.immersive_state.imm_history.clear();
        self.immersive_state.imm_input.clear();
        self.preview_state
            .worker
            .request(dest, self.preview_state.terminal_width.saturating_sub(2))
            .await;
    }

    pub async fn poll_preview(&mut self) {
        if let Some(out) = self.preview_state.worker.take_output().await {
            self.preview_state.preview_output = out.clone();
            self.preview_state.cached_preview = Some(ansi_to_text(&out));
            self.preview_state.preview_loading = false;

            if self.mode == Mode::Immersive && self.immersive_state.imm_history.is_empty() {
                self.immersive_state.imm_history.push(ImmLine {
                    kind: ImmKind::Prompt,
                    content: out,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::App;
    use std::path::PathBuf;

    /// Delegate to the shared helper in app/mod.rs
    fn dummy_themes() -> Vec<crate::core::themes::Theme> {
        super::super::dummy_themes()
    }

    #[tokio::test]
    async fn test_zoom_in_bounded() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        for _ in 0..20 {
            app.zoom_in();
        }
        assert!(
            app.preview_state.zoom_factor >= 0.5,
            "zoom_factor should not go below 0.5, got {}",
            app.preview_state.zoom_factor
        );
    }

    #[tokio::test]
    async fn test_zoom_out_bounded() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        for _ in 0..20 {
            app.zoom_out();
        }
        assert!(
            app.preview_state.zoom_factor <= 3.0,
            "zoom_factor should not exceed 3.0, got {}",
            app.preview_state.zoom_factor
        );
    }

    #[tokio::test]
    async fn test_zoom_reset() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        app.zoom_in();
        app.zoom_in();
        app.zoom_reset();
        assert_eq!(app.preview_state.zoom_factor, 1.0);
    }
}
