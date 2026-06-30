use crate::config::Config;
use crate::preview::PreviewWorker;
use crate::search::FuzzySearch;
use crate::shell::ShellInfo;
use crate::themes::Theme;
use crate::ui::ansi_to_text;
use crossterm::ExecutableCommand;
use rand::Rng;
use std::path::PathBuf;

const PREVIEW_DEBOUNCE: u8 = 5; // ~165ms at 30fps

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Confirm,
    Help,
    Immersive,
    SoftRevert, // U — remove posh-tui block
    HardRevert, // Ctrl+U — remove all omp lines
    Message,    // show result of an operation
}

pub struct App {
    pub themes: Vec<Theme>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub preview_output: String,
    pub cached_preview: Option<ratatui::text::Text<'static>>,
    pub preview_loading: bool,
    pub show_favs: bool,
    pub show_recent: bool,
    pub favourites: std::collections::HashSet<String>,
    pub last_applied: Option<String>,
    pub hide_font_warning: bool,
    pub should_quit: bool,
    pub cache_dir: PathBuf,
    pub worker: PreviewWorker,
    pub preview_width: u16,
    pub terminal_width: u16,
    // scroll & zoom
    pub scroll_offset: u16,
    pub zoom_factor: f32, // 1.0 = natural, 1.5 = compressed to 66%
    // auto-preview debounce (ticks, ~33 ms each)
    pub preview_timer: u8,
    // immersive mode
    pub imm_input: String,
    pub imm_history: Vec<ImmLine>,
    pub imm_cursor_tick: u8,
    pub fuzzy: FuzzySearch,
    pub shell_info: Option<ShellInfo>,
    pub message: String, // shown in Message mode
    pub message_is_err: bool,
    pub hard_revert_preview: Vec<(usize, String)>,
    pub config: Config,
    pub loading: bool,
    pub refreshing: bool,
    pub refresh_rx: Option<tokio::sync::mpsc::Receiver<Vec<Theme>>>,
    pub dry_run: bool,
}

#[derive(Clone)]
pub struct ImmLine {
    pub kind: ImmKind,
    pub content: String,
}

#[derive(Clone, PartialEq)]
pub enum ImmKind {
    Prompt, // the oh-my-posh rendered prompt line
    Input,  // what the user typed
    Output, // fake command output
}

impl App {
    pub fn new(themes: Vec<Theme>, cache_dir: PathBuf) -> Self {
        let filtered = (0..themes.len()).collect();
        let theme_names: Vec<String> = themes.iter().map(|t| t.name.clone()).collect();
        let config = Config::load();

        // restore saved state
        let favourites = config.favourites.clone();
        let zoom_factor = config.zoom_factor;
        let last_applied = config.last_applied.clone();
        let hide_font_warning = config.hide_font_warning;

        // find index of last applied theme so we can scroll to it
        let selected = last_applied
            .as_ref()
            .and_then(|name| themes.iter().position(|t| &t.name == name))
            .unwrap_or(0);

        Self {
            themes,
            filtered,
            selected,
            mode: Mode::Normal,
            search_query: String::new(),
            preview_output: String::new(),
            cached_preview: None,
            preview_loading: false,
            show_favs: false,
            show_recent: false,
            favourites,
            last_applied,
            hide_font_warning,
            should_quit: false,
            cache_dir,
            worker: PreviewWorker::spawn(),
            preview_width: 80,
            terminal_width: 80,
            scroll_offset: 0,
            zoom_factor,
            preview_timer: 0,
            imm_input: String::new(),
            imm_history: Vec::new(),
            imm_cursor_tick: 0,
            fuzzy: FuzzySearch::new(theme_names),
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
        self.config.favourites = self.favourites.clone();
        self.config.zoom_factor = self.zoom_factor;
        self.config.last_applied = self.last_applied.clone();
        self.config.hide_font_warning = self.hide_font_warning;

        if let Err(e) = self.config.save() {
            // non-fatal — just show a brief message
            eprintln!("config save failed: {e}");
        }
    }

    pub fn selected_theme(&self) -> Option<&Theme> {
        self.visible_themes().get(self.selected).copied()
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.visible_themes().len() {
            self.selected += 1;
        }
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }
    pub fn move_bottom(&mut self) {
        let len = self.visible_themes().len();
        if len > 0 {
            self.selected = len - 1;
        }
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(10);
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn page_down(&mut self) {
        let max = self.visible_themes().len().saturating_sub(1);
        self.selected = (self.selected + 10).min(max);
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn scroll_left(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(4);
    }
    pub fn scroll_right(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(4);
    }

    pub fn zoom_in(&mut self) {
        self.zoom_factor = (self.zoom_factor - 0.25).max(0.5);
        self.scroll_offset = 0;
        self.save_config();
    }

    pub fn zoom_out(&mut self) {
        self.zoom_factor = (self.zoom_factor + 0.25).min(3.0);
        self.scroll_offset = 0;
        self.save_config();
    }

    pub fn zoom_reset(&mut self) {
        self.zoom_factor = 1.0;
        self.scroll_offset = 0;
        self.save_config();
    }

    pub fn step_preview_timer(&mut self) -> bool {
        if self.preview_timer > 0 {
            self.preview_timer -= 1;
            self.preview_timer == 0
        } else {
            false
        }
    }

    pub fn toggle_favourite(&mut self) {
        if let Some(t) = self.selected_theme() {
            let name = t.name.clone();
            if self.favourites.contains(&name) {
                self.favourites.remove(&name);
            } else {
                self.favourites.insert(name);
            }
        }
        self.save_config();
    }

    pub fn random_theme(&mut self) {
        let len = self.visible_themes().len();
        if len == 0 {
            return;
        }
        self.selected = rand::rng().random_range(0..len);
        self.scroll_offset = 0;
        self.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn apply_search(&mut self, query: &str) {
        self.search_query = query.to_string();

        let matches = self.fuzzy.query(query);

        // map fuzzy results back to indices into self.themes
        self.filtered = matches
            .iter()
            .filter_map(|name| self.themes.iter().position(|t| &t.name == name))
            .collect();

        self.selected = 0;
        self.scroll_offset = 0;
        self.preview_timer = 0;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        // empty query returns all themes sorted
        let matches = self.fuzzy.query("");
        self.filtered = matches
            .iter()
            .filter_map(|name| self.themes.iter().position(|t| &t.name == name))
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
        self.preview_timer = 0;
    }

    pub fn visible_themes(&self) -> Vec<&Theme> {
        self.filtered
            .iter()
            .filter_map(|&i| self.themes.get(i))
            .filter(|t| {
                if self.show_favs {
                    self.favourites.contains(&t.name)
                } else if self.show_recent {
                    self.config.recent.contains(&t.name)
                } else {
                    true
                }
            })
            .collect()
    }

    // effective columns we tell oh-my-posh to render at
    pub fn effective_columns(&self, base_width: u16) -> u16 {
        ((base_width as f32) * self.zoom_factor) as u16
    }

    pub async fn edit_theme(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else {
            return;
        };

        let path = self.cache_dir.join(&theme.filename);
        if !path.exists() {
            // Can't edit what isn't downloaded
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
        self.preview_loading = true;
        self.preview_output = String::new();
        self.cached_preview = None;

        let dest = self.cache_dir.join(&theme.filename);
        if !dest.exists() {
            match crate::themes::download_theme(&theme, &self.cache_dir).await {
                Ok(_) => {}
                Err(e) => {
                    self.preview_output = format!("  download error: {e}");
                    self.preview_loading = false;
                    return;
                }
            }
        }

        let cols = self.effective_columns(self.preview_width);
        self.worker.request(dest, cols).await;
    }

    pub async fn trigger_immersive_preview(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else {
            return;
        };
        let dest = self.cache_dir.join(&theme.filename);
        if !dest.exists() {
            let _ = crate::themes::download_theme(&theme, &self.cache_dir).await;
        }
        self.imm_history.clear();
        self.imm_input.clear();
        // render at true full terminal width
        self.worker
            .request(dest, self.terminal_width.saturating_sub(2))
            .await;
    }

    pub async fn poll_preview(&mut self) {
        if let Some(out) = self.worker.take_output().await {
            self.preview_output = out.clone();
            self.cached_preview = Some(ansi_to_text(&out));
            self.preview_loading = false;

            if self.mode == Mode::Immersive && self.imm_history.is_empty() {
                self.imm_history.push(ImmLine {
                    kind: ImmKind::Prompt,
                    content: out,
                });
            }
        }
    }

    // called once background fetch completes
    pub fn init_themes(&mut self, list: Vec<Theme>) {
        let theme_names: Vec<String> = list.iter().map(|t| t.name.clone()).collect();
        self.fuzzy = crate::search::FuzzySearch::new(theme_names);
        self.filtered = (0..list.len()).collect();
        self.themes = list;
        self.loading = false;

        // restore cursor to last applied
        if let Some(name) = &self.last_applied.clone() {
            if let Some(pos) = self.themes.iter().position(|t| &t.name == name) {
                self.selected = pos;
            }
        }
    }

    // r key — re-fetch from GitHub
    pub fn start_refresh(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        self.preview_output.clear();
        self.cached_preview = None;

        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<Theme>>(1);
        self.refresh_rx = Some(rx);

        tokio::spawn(async move {
            if let Ok(list) = crate::themes::fetch_theme_list().await {
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
            self.message = format!("✓ refreshed — {} themes loaded", self.themes.len());
            self.message_is_err = false;
            self.mode = Mode::Message;
        }
    }

    pub fn tick(&mut self) {
        self.imm_cursor_tick = self.imm_cursor_tick.wrapping_add(1);
    }

    // called in immersive mode when user presses Enter
    pub fn imm_submit(&mut self) {
        let input = self.imm_input.trim().to_string();
        self.imm_input.clear();

        if input.is_empty() {
            // blank enter — just add a new prompt
            self.imm_history.push(ImmLine {
                kind: ImmKind::Prompt,
                content: self.preview_output.clone(),
            });
            return;
        }

        // find the last prompt in history and replace it with:
        // prompt → typed input → output → new prompt
        // this way it reads top-to-bottom like a real shell

        // 1. record the typed command on the last prompt line
        self.imm_history.push(ImmLine {
            kind: ImmKind::Input,
            content: input.clone(),
        });

        // 2. fake output
        let cmd = input.to_lowercase();
        let output_lines = fake_command_output(&cmd);

        if cmd == "clear" {
            self.imm_history.clear();
        } else {
            for line in output_lines {
                self.imm_history.push(ImmLine {
                    kind: ImmKind::Output,
                    content: line,
                });
            }
        }

        // 3. new prompt ready for next command
        self.imm_history.push(ImmLine {
            kind: ImmKind::Prompt,
            content: self.preview_output.clone(),
        });
    }

    pub fn imm_backspace(&mut self) {
        self.imm_input.pop();
    }
    pub fn imm_push(&mut self, c: char) {
        self.imm_input.push(c);
    }

    pub fn load_shell_info(&mut self) {
        match crate::shell::ShellInfo::load() {
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
        let theme_name = theme.name.clone(); // clone before borrow ends
        let rc_path = info.rc_path.display().to_string(); // clone before borrow ends

        if self.dry_run {
            self.message = format!("DRY RUN: Would have modified {}", rc_path);
            self.message_is_err = false;
            self.mode = Mode::Message;
            return;
        }

        match crate::shell::apply_theme(self.shell_info.as_ref().unwrap(), &theme_path) {
            Ok(_) => {
                self.last_applied = Some(theme_name.clone());
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

        match crate::shell::undo(info) {
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

        match crate::shell::soft_revert(info) {
            Ok(_) => {
                self.message        = "✓ removed posh-tui block from rc file\n\nopen a new terminal to see default prompt.".into();
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

        match crate::shell::hard_revert(info) {
            Ok(n) => {
                self.message        = format!(
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
        match crate::shell::hard_revert_preview(info) {
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

fn fake_command_output(cmd: &str) -> Vec<String> {
    let cmd = cmd.trim();
    match cmd {
        "ls" | "ls -la" | "ls -l" => vec![
            "total 48".into(),
            "drwxr-xr-x  8 user user 4096 May 30 13:01 \x1b[34m.\x1b[0m".into(),
            "drwxr-xr-x 42 user user 4096 May 30 12:00 \x1b[34m..\x1b[0m".into(),
            "drwxr-xr-x  8 user user 4096 May 30 13:01 \x1b[34m.git\x1b[0m".into(),
            "-rw-r--r--  1 user user  734 May 30 11:22 Cargo.toml".into(),
            "drwxr-xr-x  2 user user 4096 May 30 13:01 \x1b[34msrc\x1b[0m".into(),
            "drwxr-xr-x  3 user user 4096 May 30 12:45 \x1b[34mtarget\x1b[0m".into(),
        ],
        "pwd" => vec!["/home/user/projects/posh-tui".into()],
        "whoami" => vec!["user".into()],
        "git status" => vec![
            "On branch main".into(),
            "Your branch is up to date with 'origin/main'.".into(),
            "".into(),
            "Changes not staged for commit:".into(),
            "  (use \"git add <file>...\" to update what will be committed)".into(),
            "".into(),
            "\t\x1b[31mmodified:   src/ui.rs\x1b[0m".into(),
            "\t\x1b[31mmodified:   src/app.rs\x1b[0m".into(),
            "".into(),
            "no changes added to commit (use \"git add\" and \"git commit\")".into(),
        ],
        "git log --oneline" | "git log" => vec![
            "\x1b[33ma3f1c2d\x1b[0m feat: add immersive preview mode".into(),
            "\x1b[33mb8e4a1f\x1b[0m feat: scroll and zoom for preview pane".into(),
            "\x1b[33mc9d2b3e\x1b[0m feat: live ANSI preview via oh-my-posh CLI".into(),
            "\x1b[33md1e5f4a\x1b[0m feat: TUI layout with ratatui".into(),
            "\x1b[33me7c8d2b\x1b[0m init: project scaffold".into(),
        ],
        "git branch" => vec![
            "* \x1b[32mmain\x1b[0m".into(),
            "  dev".into(),
            "  feature/immersive-mode".into(),
        ],
        "echo hello" | "echo" => vec!["hello".into()],
        "uname -a" => vec![
            "Linux Pavilion 5.15.167.4-microsoft-standard-WSL2 #1 SMP x86_64 GNU/Linux".into(),
        ],
        "cat cargo.toml" | "cat ./cargo.toml" => vec![
            "[package]".into(),
            "name = \"posh-tui\"".into(),
            "version = \"0.1.0\"".into(),
            "edition = \"2021\"".into(),
        ],
        "cargo build" => vec![
            "   \x1b[32mCompiling\x1b[0m posh-tui v0.1.0".into(),
            "    \x1b[32mFinished\x1b[0m dev [unoptimized + debuginfo] target(s) in 0.84s".into(),
        ],
        "cargo run" => vec![
            "    \x1b[32mFinished\x1b[0m dev [unoptimized + debuginfo] target(s) in 0.12s".into(),
            "     \x1b[32mRunning\x1b[0m `target/debug/posh-tui`".into(),
        ],
        "neofetch" | "fastfetch" => vec![
            "".into(),
            "        \x1b[34m████████\x1b[0m  \x1b[1muser\x1b[0m@\x1b[1mPavilion\x1b[0m".into(),
            "      \x1b[34m████████████\x1b[0m  \x1b[90m─────────────────\x1b[0m".into(),
            "    \x1b[34m████\x1b[0m  \x1b[34m████████\x1b[0m  \x1b[1mOS:\x1b[0m     Ubuntu 26.04 LTS".into(),
            "    \x1b[34m████████████\x1b[0m    \x1b[1mKernel:\x1b[0m WSL2 5.15.167".into(),
            "      \x1b[34m████████\x1b[0m    \x1b[1mShell:\x1b[0m  bash 5.2.21".into(),
            "        \x1b[34m████\x1b[0m      \x1b[1mTerm:\x1b[0m   Windows Terminal".into(),
            "".into(),
        ],
        "clear" => vec!["[screen cleared]".into()],
        "help" | "?" => vec![
            "available fake commands:".into(),
            "  ls, pwd, whoami, git status, git log,".into(),
            "  git branch, echo, uname -a, cargo build,".into(),
            "  cargo run, neofetch, clear, cat Cargo.toml".into(),
            "".into(),
            "press Esc to return · Enter to apply theme".into(),
        ],
        "" => vec![],
        _ => vec![format!("bash: {}: command not found", cmd.split_whitespace().next().unwrap_or(cmd))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::Theme;
    use std::path::PathBuf;

    fn dummy_themes() -> Vec<Theme> {
        vec![
            Theme {
                name: "catppuccin".into(),
                filename: "catppuccin.omp.json".into(),
                raw_url: "http://example.com/1".into(),
                local: None,
            },
            Theme {
                name: "tokyo-night".into(),
                filename: "tokyo-night.omp.json".into(),
                raw_url: "http://example.com/2".into(),
                local: None,
            },
            Theme {
                name: "agnoster".into(),
                filename: "agnoster.omp.json".into(),
                raw_url: "http://example.com/3".into(),
                local: None,
            },
        ]
    }

    // zoom_in DECREASES zoom_factor (more lines visible), clamped at 0.5
    // zoom_out INCREASES zoom_factor (fewer lines visible), clamped at 3.0
    #[tokio::test]
    async fn test_zoom_in_bounded() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        for _ in 0..20 {
            app.zoom_in();
        }
        assert!(
            app.zoom_factor >= 0.5,
            "zoom_factor should not go below 0.5, got {}",
            app.zoom_factor
        );
    }

    #[tokio::test]
    async fn test_zoom_out_bounded() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        for _ in 0..20 {
            app.zoom_out();
        }
        assert!(
            app.zoom_factor <= 3.0,
            "zoom_factor should not exceed 3.0, got {}",
            app.zoom_factor
        );
    }

    #[tokio::test]
    async fn test_zoom_reset() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        app.zoom_in();
        app.zoom_in();
        app.zoom_reset();
        assert_eq!(app.zoom_factor, 1.0);
    }

    #[tokio::test]
    async fn test_toggle_favourite_adds_and_removes() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        // filtered already spans all themes from App::new; selected=0 picks themes[0] = catppuccin
        app.selected = 0;

        // toggle on — catppuccin should appear in favourites
        app.toggle_favourite();
        assert!(
            app.favourites.contains("catppuccin"),
            "expected catppuccin in favourites after first toggle"
        );

        // toggle off — catppuccin should be removed
        app.toggle_favourite();
        assert!(
            !app.favourites.contains("catppuccin"),
            "expected catppuccin removed from favourites after second toggle"
        );
    }

    #[tokio::test]
    async fn test_move_up_at_top_clamps() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        app.selected = 0;
        app.move_up();
        assert_eq!(app.selected, 0, "move_up at index 0 should stay at 0");
    }

    #[tokio::test]
    async fn test_visible_themes_favs_filter() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        // filtered covers all three themes from App::new
        app.favourites.insert("catppuccin".into());
        app.show_favs = true;
        let visible = app.visible_themes();
        assert_eq!(visible.len(), 1, "only the favourite should be visible");
        assert_eq!(visible[0].name, "catppuccin");
    }
}
