mod immersive_state;
mod preview_state;
mod search_state;
mod theme_state;

pub use immersive_state::ImmersiveState;
pub use preview_state::PreviewState;
pub use search_state::SearchState;
pub use theme_state::ThemeState;

use crate::core::config::Config;
use crate::core::shell::ShellInfo;
use crate::core::themes::Theme;
use crate::preview::PreviewWorker;
use crate::search::FuzzySearch;
use std::path::PathBuf;

const PREVIEW_DEBOUNCE: u8 = 5;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Confirm,
    Help,
    Immersive,
    SoftRevert,
    HardRevert,
    Message,
}

#[derive(Clone)]
pub struct ImmLine {
    pub kind: ImmKind,
    pub content: String,
}

#[derive(Clone, PartialEq)]
pub enum ImmKind {
    Prompt,
    Input,
    Output,
}

pub struct App {
    // sub-states
    pub theme_state: ThemeState,
    pub preview_state: PreviewState,
    pub search_state: SearchState,
    pub immersive_state: ImmersiveState,

    // app-level meta
    pub mode: Mode,
    pub should_quit: bool,
    pub hide_font_warning: bool,
    pub cache_dir: PathBuf,
    pub shell_info: Option<ShellInfo>,
    pub message: String,
    pub message_is_err: bool,
    pub hard_revert_preview: Vec<(usize, String)>,
    pub config: Config,
    pub loading: bool,
    pub refreshing: bool,
    pub refresh_rx: Option<tokio::sync::mpsc::Receiver<Vec<Theme>>>,
    pub dry_run: bool,
}

impl App {
    pub fn new(themes: Vec<Theme>, cache_dir: PathBuf) -> Self {
        let filtered = (0..themes.len()).collect();
        let theme_names: Vec<String> = themes.iter().map(|t| t.name.clone()).collect();
        let config = Config::load();

        let favourites = config.favourites.clone();
        let zoom_factor = config.zoom_factor;
        let last_applied = config.last_applied.clone();
        let hide_font_warning = config.hide_font_warning;

        let selected = last_applied
            .as_ref()
            .and_then(|name| themes.iter().position(|t| &t.name == name))
            .unwrap_or(0);

        Self {
            theme_state: ThemeState {
                themes,
                filtered,
                selected,
                favourites,
                show_favs: false,
                show_recent: false,
                last_applied,
            },
            preview_state: PreviewState {
                preview_output: String::new(),
                cached_preview: None,
                preview_loading: false,
                worker: PreviewWorker::spawn(),
                preview_width: 80,
                terminal_width: 80,
                scroll_offset: 0,
                zoom_factor,
                preview_timer: 0,
            },
            search_state: SearchState {
                search_query: String::new(),
                fuzzy: FuzzySearch::new(theme_names),
            },
            immersive_state: ImmersiveState {
                imm_input: String::new(),
                imm_history: Vec::new(),
                imm_cursor_tick: 0,
            },
            mode: Mode::Normal,
            should_quit: false,
            hide_font_warning,
            cache_dir,
            shell_info: None,
            message: String::new(),
            message_is_err: false,
            hard_revert_preview: Vec::new(),
            config,
            loading: false,
            refreshing: false,
            refresh_rx: None,
            dry_run: false,
        }
    }

    pub fn save_config(&mut self) {
        self.config.favourites = self.theme_state.favourites.clone();
        self.config.zoom_factor = self.preview_state.zoom_factor;
        self.config.last_applied = self.theme_state.last_applied.clone();
        self.config.hide_font_warning = self.hide_font_warning;

        if let Err(e) = self.config.save() {
            eprintln!("config save failed: {e}");
        }
    }

    pub fn tick(&mut self) {
        self.immersive_state.imm_cursor_tick =
            self.immersive_state.imm_cursor_tick.wrapping_add(1);
    }

    pub fn start_refresh(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        self.preview_state.preview_output.clear();
        self.preview_state.cached_preview = None;

        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<Theme>>(1);
        self.refresh_rx = Some(rx);

        tokio::spawn(async move {
            if let Ok(list) = crate::core::themes::fetch_theme_list().await {
                let _ = tx.send(list).await;
            }
        });
    }

    pub async fn poll_refresh(&mut self) {
        if !self.refreshing {
            return;
        }
        let done = if let Some(rx) = &mut self.refresh_rx {
            match rx.try_recv() {
                Ok(list) => {
                    self.init_themes(list);
                    Some(true)
                }
                Err(_) => None,
            }
        } else {
            None
        };

        if done.is_some() {
            self.refreshing = false;
            self.refresh_rx = None;
            self.message = format!("✓ refreshed — {} themes loaded", self.theme_state.themes.len());
            self.message_is_err = false;
            self.mode = Mode::Message;
        }
    }

    pub fn load_shell_info(&mut self) {
        match crate::core::shell::ShellInfo::load() {
            Ok(info) => self.shell_info = Some(info),
            Err(e) => {
                self.message = format!("shell detection failed: {e}");
                self.message_is_err = true;
                self.mode = Mode::Message;
            }
        }
    }

    pub fn do_apply(&mut self) {
        let Some(info) = &self.shell_info else { return };
        let Some(theme) = self.selected_theme() else {
            return;
        };
        let theme_path = self.cache_dir.join(&theme.filename);
        let theme_name = theme.name.clone();
        let rc_path = info.rc_path.display().to_string();

        if self.dry_run {
            self.message = format!("DRY RUN: Would have modified {}", rc_path);
            self.message_is_err = false;
            self.mode = Mode::Message;
            return;
        }

        match crate::core::shell::apply_theme(self.shell_info.as_ref().unwrap(), &theme_path) {
            Ok(_) => {
                self.theme_state.last_applied = Some(theme_name.clone());
                self.config.last_applied = Some(theme_name.clone());
                self.save_config();
                self.message = format!(
                    "✓ applied {}  →  {}\n\nopen a new terminal to see it.\npress u to undo.",
                    theme_name, rc_path,
                );
                self.message_is_err = false;
                self.load_shell_info();
            }
            Err(e) => {
                self.message = format!("✗ apply failed: {e}");
                self.message_is_err = true;
            }
        }
        self.mode = Mode::Message;
    }

    pub fn do_undo(&mut self) {
        let Some(info) = &self.shell_info else { return };

        if self.dry_run {
            self.message = format!(
                "DRY RUN: Would have restored backup to {}",
                info.rc_path.display()
            );
            self.message_is_err = false;
            self.mode = Mode::Message;
            return;
        }

        match crate::core::shell::undo(info) {
            Ok(_) => {
                self.message = format!("✓ restored backup\n→ {}", info.backup_path.display());
                self.message_is_err = false;
                self.load_shell_info();
            }
            Err(e) => {
                self.message = format!("✗ undo failed: {e}");
                self.message_is_err = true;
            }
        }
        self.mode = Mode::Message;
    }

    pub fn do_soft_revert(&mut self) {
        let Some(info) = &self.shell_info else { return };

        if self.dry_run {
            self.message = format!(
                "DRY RUN: Would have removed posh-tui block from {}",
                info.rc_path.display()
            );
            self.message_is_err = false;
            self.mode = Mode::Message;
            return;
        }

        match crate::core::shell::soft_revert(info) {
            Ok(_) => {
                self.message = "✓ removed posh-tui block from rc file\n\nopen a new terminal to see default prompt.".into();
                self.message_is_err = false;
                self.load_shell_info();
            }
            Err(e) => {
                self.message = format!("✗ revert failed: {e}");
                self.message_is_err = true;
            }
        }
        self.mode = Mode::Message;
    }

    pub fn do_hard_revert(&mut self) {
        let Some(info) = &self.shell_info else { return };

        if self.dry_run {
            self.message = format!(
                "DRY RUN: Would have removed oh-my-posh lines from {}",
                info.rc_path.display()
            );
            self.message_is_err = false;
            self.mode = Mode::Message;
            return;
        }

        match crate::core::shell::hard_revert(info) {
            Ok(n) => {
                self.message = format!(
                    "✓ removed {n} oh-my-posh line(s) from {}\n\nopen a new terminal to see default prompt.",
                    info.rc_path.display()
                );
                self.message_is_err = false;
                self.load_shell_info();
            }
            Err(e) => {
                self.message = format!("✗ hard revert failed: {e}");
                self.message_is_err = true;
            }
        }
        self.mode = Mode::Message;
    }

    pub fn prepare_hard_revert(&mut self) {
        let Some(info) = &self.shell_info else { return };
        match crate::core::shell::hard_revert_preview(info) {
            Ok(lines) => {
                self.hard_revert_preview = lines;
                self.mode = Mode::HardRevert;
            }
            Err(e) => {
                self.message = format!("✗ {e}");
                self.message_is_err = true;
                self.mode = Mode::Message;
            }
        }
    }
}
