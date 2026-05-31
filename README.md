# posh-tui

A terminal UI for browsing, previewing, and applying [Oh My Posh](https://ohmyposh.dev) themes — built in Rust.

![Rust](https://img.shields.io/badge/rust-1.96%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20WSL-lightgrey)

---

## What it does

- Browse all 120+ Oh My Posh themes in a scrollable list
- **Live ANSI preview** — calls the real `oh-my-posh` binary, renders the actual prompt with full colors
- **Immersive mode** — full-screen preview at true terminal width with a fake interactive shell
- Apply a theme directly to your shell config (`~/.bashrc`, `~/.zshrc`, `~/.config/fish/config.fish`)
- Undo, soft revert, and hard revert — always backs up before touching anything
- Fuzzy search powered by [nucleo](https://github.com/helix-editor/nucleo)
- Persistent favourites and last-applied theme across sessions

---

## Requirements

- [Oh My Posh](https://ohmyposh.dev/docs/installation/linux) installed and in `$PATH`
- A [Nerd Font](https://www.nerdfonts.com) installed and set in your terminal
- Rust 1.70+ (for building from source)

---

## Install

### curl (recommended)

```bash
curl -sSf https://raw.githubusercontent.com/RayenBHK/posh-tui/main/install.sh | bash
```

### cargo

```bash
cargo install --git https://github.com/RayenBHK/posh-tui
```

### Manual build

```bash
git clone https://github.com/RayenBHK/posh-tui
cd posh-tui
cargo build --release
./target/release/posh-tui
```

---

## Usage

```bash
posh-tui
```

The app opens instantly. Themes are fetched from GitHub in the background.

---

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `g` / `G` | Jump to top / bottom |
| `PgUp` / `PgDn` | Page scroll |

### Preview

| Key | Action |
|-----|--------|
| `Space` | Preview selected theme in side pane |
| `p` | Immersive full-screen preview |
| `<` / `>` | Scroll preview left / right |
| `-` / `=` | Zoom out / in |
| `0` | Reset zoom |

### Immersive mode

| Key | Action |
|-----|--------|
| `Enter` | Run fake command |
| `Ctrl+A` | Apply theme from immersive mode |
| `Esc` | Return to browser |

Try typing: `ls`, `git status`, `git log`, `neofetch`, `pwd`, `help`

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Apply selected theme to shell config |
| `u` | Undo last apply (restores backup) |
| `U` | Soft revert — remove posh-tui block only |
| `Ctrl+U` | Hard revert — remove all oh-my-posh lines |
| `f` | Toggle favourite |
| `F` | Toggle favourites-only view |
| `r` | Refresh theme list from GitHub |
| `/` | Fuzzy search |
| `?` | Help overlay |
| `q` | Quit |

---

## How apply works

When you apply a theme, posh-tui:

1. Detects your shell via `$SHELL`
2. Backs up your rc file to `~/.bashrc.posh-tui.bak`
3. Writes a managed block at the bottom:

```bash
# posh-tui:start
eval "$(/home/user/.local/bin/oh-my-posh init bash --config ~/.cache/posh-tui/themes/catppuccin.omp.json)"
# posh-tui:end
```

Open a new terminal to see the theme. Press `u` to undo at any time.

---

## Config & cache

```
~/.config/posh-tui/config.toml   # favourites, last applied, zoom factor
~/.cache/posh-tui/themes/        # downloaded .omp.json files
```

---

## Built with

- [ratatui](https://github.com/ratatui-org/ratatui) — TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal events
- [tokio](https://tokio.rs) — async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client
- [nucleo](https://github.com/helix-editor/nucleo) — fuzzy search
- [serde](https://serde.rs) — serialisation

---

## License

MIT © [RayenBHK](https://github.com/RayenBHK)
