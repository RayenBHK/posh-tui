use std::path::PathBuf;
use crate::themes::Theme;
use crate::preview::PreviewWorker;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Confirm,
    Help,
    Immersive,
}

pub struct App {
    pub themes:           Vec<Theme>,
    pub filtered:         Vec<usize>,
    pub selected:         usize,
    pub mode:             Mode,
    pub search_query:     String,
    pub preview_output:   String,
    pub preview_loading:  bool,
    pub show_favs:        bool,
    pub favourites:       std::collections::HashSet<String>,
    pub last_applied:     Option<String>,
    pub should_quit:      bool,
    pub cache_dir:        PathBuf,
    pub worker:           PreviewWorker,
    pub preview_width:    u16,
    pub terminal_width:   u16,
    // scroll & zoom
    pub scroll_offset:    u16,
    pub zoom_factor:      f32,   // 1.0 = natural, 1.5 = compressed to 66%
    // immersive mode
    pub imm_input:        String,
    pub imm_history:      Vec<ImmLine>,
    pub imm_cursor_tick:  u8,
}

#[derive(Clone)]
pub struct ImmLine {
    pub kind:    ImmKind,
    pub content: String,
}

#[derive(Clone, PartialEq)]
pub enum ImmKind {
    Prompt,   // the oh-my-posh rendered prompt line
    Input,    // what the user typed
    Output,   // fake command output
    Blank,
}

impl App {
    pub fn new(themes: Vec<Theme>, cache_dir: PathBuf) -> Self {
        let filtered = (0..themes.len()).collect();
        Self {
            themes,
            filtered,
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            preview_output: String::new(),
            preview_loading: false,
            show_favs: false,
            favourites: std::collections::HashSet::new(),
            last_applied: None,
            should_quit: false,
            cache_dir,
            worker: PreviewWorker::spawn(),
            preview_width: 80,
            terminal_width: 80,
            scroll_offset: 0,
            zoom_factor: 1.0,
            imm_input: String::new(),
            imm_history: Vec::new(),
            imm_cursor_tick: 0,
        }
    }

    pub fn selected_theme(&self) -> Option<&Theme> {
        self.filtered.get(self.selected)
            .and_then(|&i| self.themes.get(i))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 { self.selected -= 1; }
        self.scroll_offset = 0;
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.visible_themes().len() {
            self.selected += 1;
        }
        self.scroll_offset = 0;
    }

    pub fn move_top(&mut self)    { self.selected = 0; self.scroll_offset = 0; }
    pub fn move_bottom(&mut self) {
        let len = self.visible_themes().len();
        if len > 0 { self.selected = len - 1; }
        self.scroll_offset = 0;
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(10);
        self.scroll_offset = 0;
    }

    pub fn page_down(&mut self) {
        let max = self.visible_themes().len().saturating_sub(1);
        self.selected = (self.selected + 10).min(max);
        self.scroll_offset = 0;
    }

    pub fn scroll_left(&mut self)  { self.scroll_offset = self.scroll_offset.saturating_sub(4); }
    pub fn scroll_right(&mut self) { self.scroll_offset = self.scroll_offset.saturating_add(4); }

    pub fn zoom_in(&mut self) {
        self.zoom_factor = (self.zoom_factor - 0.25).max(0.5);
        self.scroll_offset = 0;
    }

    pub fn zoom_out(&mut self) {
        self.zoom_factor = (self.zoom_factor + 0.25).min(3.0);
        self.scroll_offset = 0;
    }

    pub fn zoom_reset(&mut self) {
        self.zoom_factor = 1.0;
        self.scroll_offset = 0;
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
    }

    pub fn apply_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        let q = query.to_lowercase();
        self.filtered = self.themes
            .iter()
            .enumerate()
            .filter(|(_, t)| t.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.filtered = (0..self.themes.len()).collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn visible_themes(&self) -> Vec<&Theme> {
        self.filtered.iter()
            .filter_map(|&i| self.themes.get(i))
            .filter(|t| {
                if self.show_favs { self.favourites.contains(&t.name) }
                else { true }
            })
            .collect()
    }

    // effective columns we tell oh-my-posh to render at
    pub fn effective_columns(&self, base_width: u16) -> u16 {
        ((base_width as f32) * self.zoom_factor) as u16
    }

    pub async fn trigger_preview(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else { return };
        self.preview_loading = true;
        self.preview_output  = String::new();

        let dest = self.cache_dir.join(&theme.filename);
        if !dest.exists() {
            match crate::themes::download_theme(&theme, &self.cache_dir).await {
                Ok(_)  => {}
                Err(e) => {
                    self.preview_output  = format!("  download error: {e}");
                    self.preview_loading = false;
                    return;
                }
            }
        }

        let cols = self.effective_columns(self.preview_width);
        self.worker.request(dest, cols).await;
    }

    pub async fn trigger_immersive_preview(&mut self) {
        let Some(theme) = self.selected_theme().cloned() else { return };
        let dest = self.cache_dir.join(&theme.filename);
        if !dest.exists() {
            let _ = crate::themes::download_theme(&theme, &self.cache_dir).await;
        }
        // render at full terminal width — fully accurate
        self.worker.request(dest, self.terminal_width).await;
        self.imm_history.clear();
        self.imm_input.clear();
    }

    pub async fn poll_preview(&mut self) {
        if let Some(out) = self.worker.take_output().await {
            self.preview_output = out.clone();
            self.preview_loading = false;

            // if we're in immersive mode, seed the first prompt line
            if self.mode == Mode::Immersive && self.imm_history.is_empty() {
                self.imm_history.push(ImmLine {
                    kind:    ImmKind::Prompt,
                    content: out,
                });
            }
        }
    }

    // called in immersive mode when user presses Enter
    pub fn imm_submit(&mut self) {
        let input = self.imm_input.trim().to_lowercase();
        let input_display = self.imm_input.clone();
        self.imm_input.clear();

        // record what was typed
        self.imm_history.push(ImmLine { kind: ImmKind::Input,  content: input_display.clone() });

        // fake output
        let output = fake_command_output(&input);
        for line in output {
            self.imm_history.push(ImmLine { kind: ImmKind::Output, content: line });
        }

        // add next prompt
        self.imm_history.push(ImmLine {
            kind:    ImmKind::Prompt,
            content: self.preview_output.clone(),
        });
    }

    pub fn imm_backspace(&mut self) { self.imm_input.pop(); }
    pub fn imm_push(&mut self, c: char) { self.imm_input.push(c); }

    pub fn tick(&mut self) {
        self.imm_cursor_tick = self.imm_cursor_tick.wrapping_add(1);
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