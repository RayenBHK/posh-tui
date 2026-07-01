use super::{ImmKind, ImmLine};

pub struct ImmersiveState {
    pub imm_input: String,
    pub imm_history: Vec<ImmLine>,
    pub imm_cursor_tick: u8,
}

impl super::App {
    pub fn imm_submit(&mut self) {
        let input = self.immersive_state.imm_input.trim().to_string();
        self.immersive_state.imm_input.clear();

        if input.is_empty() {
            self.immersive_state.imm_history.push(ImmLine {
                kind: ImmKind::Prompt,
                content: self.preview_state.preview_output.clone(),
            });
            return;
        }

        self.immersive_state.imm_history.push(ImmLine {
            kind: ImmKind::Input,
            content: input.clone(),
        });

        let cmd = input.to_lowercase();
        let output_lines = fake_command_output(&cmd);

        if cmd == "clear" {
            self.immersive_state.imm_history.clear();
        } else {
            for line in output_lines {
                self.immersive_state.imm_history.push(ImmLine {
                    kind: ImmKind::Output,
                    content: line,
                });
            }
        }

        self.immersive_state.imm_history.push(ImmLine {
            kind: ImmKind::Prompt,
            content: self.preview_state.preview_output.clone(),
        });
    }

    pub fn imm_backspace(&mut self) {
        self.immersive_state.imm_input.pop();
    }
    pub fn imm_push(&mut self, c: char) {
        self.immersive_state.imm_input.push(c);
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
