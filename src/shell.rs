use std::path::{Path, PathBuf};
use crate::error::{PoshError, Result};

const MARKER_START: &str = "# posh-tui:start";
const MARKER_END:   &str = "# posh-tui:end";
const BACKUP_EXT:   &str = ".posh-tui.bak";

#[derive(Debug, Clone)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Unknown(String),
}

impl Shell {
    pub fn detect() -> Result<Self> {
        let shell = std::env::var("SHELL")
            .map_err(|_| PoshError::Shell("$SHELL env var not set".into()))?;

        Ok(match shell.as_str() {
            s if s.ends_with("bash") => Shell::Bash,
            s if s.ends_with("zsh")  => Shell::Zsh,
            s if s.ends_with("fish") => Shell::Fish,
            other => Shell::Unknown(other.to_string()),
        })
    }

    pub fn name(&self) -> &str {
        match self {
            Shell::Bash       => "bash",
            Shell::Zsh        => "zsh",
            Shell::Fish       => "fish",
            Shell::Unknown(s) => s.as_str(),
        }
    }

    pub fn rc_path(&self) -> Result<PathBuf> {
        let home = dirs_next::home_dir()
            .ok_or_else(|| PoshError::Shell("cannot find home directory".into()))?;

        let path = match self {
            Shell::Bash       => home.join(".bashrc"),
            Shell::Zsh        => home.join(".zshrc"),
            Shell::Fish       => home.join(".config").join("fish").join("config.fish"),
            Shell::Unknown(s) => return Err(PoshError::Shell(
                format!("unsupported shell: {s} — edit your rc file manually")
            )),
        };

        Ok(path)
    }

    pub fn init_line(&self, theme_path: &Path) -> String {
        // use full path to oh-my-posh binary so PATH order doesn't matter
        let omp_bin = which_omp();
        let path_str = theme_path.display();
        match self {
            Shell::Fish => format!(
                "{omp_bin} init fish --config {path_str} | source"
            ),
            _ => format!(
                "eval \"$({omp_bin} init {} --config {path_str})\"",
                self.name()
            ),
        }
    }
}

// ── info about current state ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShellInfo {
    pub shell:       Shell,
    pub rc_path:     PathBuf,
    pub has_managed: bool,   // posh-tui:start block exists
    #[allow(dead_code)]
    pub has_any_omp: bool,   // any oh-my-posh line exists (for hard revert)
    pub backup_path: PathBuf,
}

impl ShellInfo {
    pub fn load() -> Result<Self> {
        let shell      = Shell::detect()?;
        let rc_path    = shell.rc_path()?;
        let backup_path = backup_path_for(&rc_path);

        let (has_managed, has_any_omp) = if rc_path.exists() {
            let contents = std::fs::read_to_string(&rc_path)?;
            let has_managed  = contents.contains(MARKER_START);
            let has_any_omp  = contents.lines().any(|l| l.contains("oh-my-posh"));
            (has_managed, has_any_omp)
        } else {
            (false, false)
        };

        Ok(Self { shell, rc_path, has_managed, has_any_omp, backup_path })
    }
}

fn backup_path_for(rc_path: &Path) -> PathBuf {
    let mut p = rc_path.to_path_buf();
    let name  = p.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    p.set_file_name(format!("{name}{BACKUP_EXT}"));
    p
}

// ── operations ────────────────────────────────────────────────────────────────

/// Write theme to rc file. Creates backup first.
pub fn apply_theme(info: &ShellInfo, theme_path: &Path) -> Result<()> {
    let contents = read_or_empty(&info.rc_path)?;
    backup(&info.rc_path, &info.backup_path)?;

    let init_line = info.shell.init_line(theme_path);
    let block     = format!("{MARKER_START}\n{init_line}\n{MARKER_END}");
    let new       = if info.has_managed {
        replace_managed_block(&contents, &block)
    } else {
        format!("{}\n\n{}\n", contents.trim_end(), block)
    };

    std::fs::write(&info.rc_path, new)?;
    Ok(())
}

/// Soft revert — remove only the posh-tui managed block.
pub fn soft_revert(info: &ShellInfo) -> Result<()> {
    if !info.has_managed {
        return Err(PoshError::Shell(
            "no posh-tui managed block found in your rc file".into()
        ));
    }

    let contents = std::fs::read_to_string(&info.rc_path)?;
    backup(&info.rc_path, &info.backup_path)?;

    let cleaned = remove_managed_block(&contents);
    std::fs::write(&info.rc_path, cleaned)?;
    Ok(())
}

/// Hard revert — remove every line containing oh-my-posh.
pub fn hard_revert(info: &ShellInfo) -> Result<usize> {
    let contents = std::fs::read_to_string(&info.rc_path)?;
    backup(&info.rc_path, &info.backup_path)?;

    let (cleaned, removed) = remove_all_omp_lines(&contents);
    std::fs::write(&info.rc_path, cleaned)?;
    Ok(removed)
}

/// Restore backup file.
pub fn undo(info: &ShellInfo) -> Result<()> {
    if !info.backup_path.exists() {
        return Err(PoshError::Shell(
            "no backup found — nothing to undo".into()
        ));
    }
    std::fs::copy(&info.backup_path, &info.rc_path)?;
    Ok(())
}

/// Preview what lines would be removed by hard revert.
pub fn hard_revert_preview(info: &ShellInfo) -> Result<Vec<(usize, String)>> {
    let contents = std::fs::read_to_string(&info.rc_path)?;
    let lines = contents
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("oh-my-posh"))
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect();
    Ok(lines)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn read_or_empty(path: &PathBuf) -> Result<String> {
    if path.exists() {
        Ok(std::fs::read_to_string(path)?)
    } else {
        Ok(String::new())
    }
}

fn backup(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    if src.exists() {
        std::fs::copy(src, dest)?;
    }
    Ok(())
}

fn replace_managed_block(contents: &str, new_block: &str) -> String {
    let mut out    = String::new();
    let mut inside = false;

    for line in contents.lines() {
        if line.trim() == MARKER_START {
            inside = true;
            out.push_str(new_block);
            out.push('\n');
            continue;
        }
        if line.trim() == MARKER_END {
            inside = false;
            continue;
        }
        if !inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn remove_managed_block(contents: &str) -> String {
    let mut out    = String::new();
    let mut inside = false;

    for line in contents.lines() {
        if line.trim() == MARKER_START { inside = true;  continue; }
        if line.trim() == MARKER_END   { inside = false; continue; }
        if !inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    // trim trailing blank lines but keep a final newline
    format!("{}\n", out.trim_end())
}

fn remove_all_omp_lines(contents: &str) -> (String, usize) {
    let mut out     = String::new();
    let mut removed = 0usize;
    let mut inside  = false;

    for line in contents.lines() {
        // skip managed block markers too
        if line.trim() == MARKER_START { inside = true;  removed += 1; continue; }
        if line.trim() == MARKER_END   { inside = false; removed += 1; continue; }
        if inside || line.contains("oh-my-posh") {
            removed += 1;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    (format!("{}\n", out.trim_end()), removed)
}


fn which_omp() -> String {
    // try to resolve the full path of oh-my-posh binary
    let candidates = [
        "/home/pavilion/.local/bin/oh-my-posh",  // fallback hardcode
        "/usr/local/bin/oh-my-posh",
        "/usr/bin/oh-my-posh",
    ];

    // prefer `which` output — works for any user
    if let Ok(out) = std::process::Command::new("which")
        .arg("oh-my-posh")
        .output()
    {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() {
            return path;
        }
    }

    // fallback to known candidates
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }

    // last resort — hope it's in PATH by the time shell loads
    "oh-my-posh".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── replace_managed_block ──────────────────────────────────────────────

    #[test]
    fn replace_managed_block_replaces_existing() {
        let contents = "# header\n\n# posh-tui:start\neval \"$(oh-my-posh init bash --config old)\"\n# posh-tui:end\n# footer\n";
        let new_block = "# posh-tui:start\neval \"$(oh-my-posh init bash --config new)\"\n# posh-tui:end";
        let result = replace_managed_block(contents, new_block);
        assert!(result.contains("config new"));
        assert!(!result.contains("config old"));
        assert!(result.contains("# header"));
        assert!(result.contains("# footer"));
        assert!(result.contains(new_block));
    }

    #[test]
    fn replace_managed_block_appends_when_no_block() {
        let contents = "# header\n# some config\n";
        let new_block = "# posh-tui:start\neval \"$(oh-my-posh init bash --config test)\"\n# posh-tui:end";
        let result = replace_managed_block(contents, new_block);
        // replace_managed_block only replaces — it doesn't add a block that doesn't exist
        assert!(result.contains("# header"));
        assert!(result.contains("# some config"));
        assert!(!result.contains("posh-tui:start"));
    }

    #[test]
    fn replace_managed_block_preserves_surrounding_content() {
        let contents = "export PATH=$PATH:/usr/local/bin\n\n# posh-tui:start\neval \"old\"\n# posh-tui:end\n\nsource ~/.bash_aliases\n";
        let new_block = "# posh-tui:start\neval \"new\"\n# posh-tui:end";
        let result = replace_managed_block(contents, new_block);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "export PATH=$PATH:/usr/local/bin");
        assert!(result.contains("source ~/.bash_aliases"));
        assert!(result.contains("eval \"new\""));
    }

    // ── remove_managed_block ───────────────────────────────────────────────

    #[test]
    fn remove_managed_block_removes_block() {
        let contents = "# header\n\n# posh-tui:start\neval \"$(oh-my-posh init bash)\"\n# posh-tui:end\n# footer\n";
        let result = remove_managed_block(contents);
        assert!(!result.contains("posh-tui:start"));
        assert!(!result.contains("posh-tui:end"));
        assert!(!result.contains("oh-my-posh"));
        assert!(result.contains("# header"));
        assert!(result.contains("# footer"));
    }

    #[test]
    fn remove_managed_block_handles_no_block() {
        let contents = "# header\n# no posh-tui here\n# footer\n";
        let result = remove_managed_block(contents);
        assert_eq!(result, "# header\n# no posh-tui here\n# footer\n");
    }

    #[test]
    fn remove_managed_block_handles_empty_file() {
        let result = remove_managed_block("");
        assert_eq!(result, "\n");
    }

    #[test]
    fn remove_managed_block_only_block() {
        let contents = "# posh-tui:start\neval \"$(oh-my-posh init bash)\"\n# posh-tui:end\n";
        let result = remove_managed_block(contents);
        assert_eq!(result.trim(), "");
    }

    // ── remove_all_omp_lines ───────────────────────────────────────────────

    #[test]
    fn remove_all_omp_lines_removes_all() {
        let contents = "# header\neval \"$(oh-my-posh init bash)\"\nsome config\n# posh-tui:start\neval \"test\"\n# posh-tui:end\n";
        let (result, removed) = remove_all_omp_lines(contents);
        assert!(!result.contains("oh-my-posh"));
        assert!(!result.contains("posh-tui:start"));
        assert!(!result.contains("posh-tui:end"));
        assert!(result.contains("# header"));
        assert!(result.contains("some config"));
        assert_eq!(removed, 4);
    }

    #[test]
    fn remove_all_omp_lines_removes_manual_lines() {
        let contents = "# my manual omp config\neval \"$(oh-my-posh init bash --config theme1)\"\nother stuff\n";
        let (result, removed) = remove_all_omp_lines(contents);
        assert!(!result.contains("oh-my-posh"));
        assert!(result.contains("other stuff"));
        assert!(result.contains("# my manual omp config")); // "omp" ≠ "oh-my-posh"
        assert_eq!(removed, 1); // only the line containing "oh-my-posh" is removed
    }

    #[test]
    fn remove_all_omp_lines_no_omp() {
        let contents = "# just normal config\nexport PATH=$PATH\nsource ~/.bashrc\n";
        let (result, removed) = remove_all_omp_lines(contents);
        assert_eq!(result, contents);
        assert_eq!(removed, 0);
    }

    #[test]
    fn remove_all_omp_lines_empty() {
        let (result, removed) = remove_all_omp_lines("");
        assert_eq!(result.trim(), "");
        assert_eq!(removed, 0);
    }

    // ── backup_path_for ────────────────────────────────────────────────────

    #[test]
    fn backup_path_for_bashrc() {
        let rc = PathBuf::from("/home/user/.bashrc");
        let bak = backup_path_for(&rc);
        assert_eq!(bak, PathBuf::from("/home/user/.bashrc.posh-tui.bak"));
    }

    #[test]
    fn backup_path_for_zshrc() {
        let rc = PathBuf::from("/home/user/.zshrc");
        let bak = backup_path_for(&rc);
        assert_eq!(bak, PathBuf::from("/home/user/.zshrc.posh-tui.bak"));
    }

    #[test]
    fn backup_path_for_fish_config() {
        let rc = PathBuf::from("/home/user/.config/fish/config.fish");
        let bak = backup_path_for(&rc);
        assert_eq!(bak, PathBuf::from("/home/user/.config/fish/config.fish.posh-tui.bak"));
    }
}