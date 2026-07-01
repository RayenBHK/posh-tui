# posh-tui — Project Documentation

> A terminal UI for browsing, previewing, and applying Oh My Posh themes — built in Rust.

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Features](#2-features)
3. [Architecture](#3-architecture)
4. [Module Reference](#4-module-reference)
5. [Keybindings](#5-keybindings)
6. [Configuration & Cache](#6-configuration--cache)
7. [Installation](#7-installation)
8. [Development](#8-development)
9. [CI/CD](#9-cicd)
10. [Dependencies](#10-dependencies)
11. [Testing](#11-testing)

---

## 1. Project Overview

**posh-tui** is a Rust-based terminal user interface that lets users browse, preview, and apply [Oh My Posh](https://ohmyposh.dev) themes directly from the terminal. It fetches the full theme catalog from GitHub, renders live previews by invoking the real `oh-my-posh` binary, and manages shell configuration files with safe, idempotent patching.

### Why it exists

Oh My Posh ships with 120+ themes but provides no interactive browser. Users must manually edit shell configs and restart terminals to try themes. posh-tui solves this by providing:

- A scrollable, searchable list of all themes
- Live ANSI previews rendered by the actual oh-my-posh binary
- One-key apply with automatic backup and undo
- An immersive full-screen preview mode with simulated shell commands

### Key facts

| Metric | Value |
|--------|-------|
| Language | Rust (edition 2021) |
| Version | 0.4.2 |
| Lines of code | ~3,260 (35 source files) |
| Binary size | ~3 MB (release, stripped, LTO) |
| Dependencies | 14 crates |
| Test count | 32 tests (including UI snapshot) |
| Platforms | Linux (x64 + ARM64), macOS (x64 + ARM64), WSL, Windows |

---

## 2. Features

### Core features (fully implemented)

| Feature | Description |
|---------|-------------|
| **Theme browsing** | Scrollable list of all 120+ Oh My Posh themes fetched from GitHub |
| **Live ANSI preview** | Calls `oh-my-posh print primary` to render the actual prompt with full colors |
| **Auto-preview on navigation** | Preview updates automatically when scrolling (165ms debounce) |
| **Immersive mode** | Full-screen preview at true terminal width with a fake interactive shell |
| **Live Theme Editing** | Open any theme in `$EDITOR` from the TUI and see it reload instantly on save |
| **Theme application** | Writes managed block to shell rc file (bash/zsh/fish) with backup |
| **Undo / revert** | Three tiers: undo (restore backup), soft revert (remove block), hard revert (remove all omp lines) |
| **Fuzzy search** | Powered by nucleo — case-insensitive with smart normalization |
| **Favourites** | Toggle favourites per theme, filter to show only favourites |
| **Recently viewed** | Tracks last 10 previewed themes, accessible via `R` key |
| **Random theme** | Press `x` to jump to a random theme |
| **Horizontal scroll + zoom** | Scroll preview left/right, zoom in/out to adjust column width |
| **Config persistence** | Favourites, last applied, zoom factor, and recent themes persisted to TOML |
| **Offline fallback** | Caches theme list JSON; loads from cache when GitHub API is unavailable. Retries up to 3× with backoff before falling back |
| **Cross-platform CI** | GitHub Actions builds for Linux (x64 + ARM64), macOS (x64 + ARM64), and Windows |

### Immersive mode commands

The immersive mode simulates a shell environment. Available fake commands:

| Command | Output |
|---------|--------|
| `ls`, `ls -la` | Colored directory listing |
| `pwd` | Current path |
| `whoami` | Current user |
| `git status` | Branch status with colored diffs |
| `git log` | Colored commit history |
| `git branch` | Branch list with current branch highlighted |
| `echo` | Echoes input |
| `uname -a` | System information |
| `cargo build` / `cargo run` | Build output simulation |
| `neofetch` / `fastfetch` | System info display with ASCII art |
| `cat Cargo.toml` | File contents |
| `clear` | Clears screen |
| `help` | Lists available commands |

---

## 3. Architecture

### Design pattern

The project follows an **Elm-like architecture** (Model-Update-View):

```mermaid
graph LR
    subgraph Entry ["🚀 Entry — main.rs"]
        EL["Event Loop\n30 fps poll"]
    end

    subgraph State ["📦 Model — app/"]
        APP["App struct\ncomposes sub-states"]
    end

    subgraph Render ["🖥️ View — ui/"]
        UI["draw()\ncomponents + screens + overlays"]
    end

    subgraph Input ["⌨️ Update — input/"]
        IN["Key/mouse dispatch\nper-mode handlers"]
    end

    subgraph Domain ["🔌 Domain — core/"]
        TH["themes.rs\nGitHub API + cache"]
        SH["shell.rs\nRC-file patching"]
        CF["config.rs\nTOML persistence"]
        ERR["error.rs\nPoshError (thiserror)"]
    end

    subgraph Infra ["⚙️ Infrastructure"]
        PV["preview.rs\nAsync oh-my-posh worker"]
        SR["search.rs\nNucleo fuzzy search"]
    end

    EL -->|"dispatch"| IN
    IN -->|"mutate"| APP
    APP -->|"state snapshot"| UI
    APP -->|"fetch / cache"| TH
    APP -->|"preview request"| PV
    APP -->|"apply / revert"| SH
    APP -->|"load / save"| CF
    APP -->|"fuzzy query"| SR
    PV -->|"ANSI output"| APP
    TH -->|"theme list"| APP
```

### Data flow

```mermaid
sequenceDiagram
    autonumber
    participant M  as main.rs
    participant A  as app/
    participant T  as core/themes.rs
    participant PW as preview.rs
    participant SH as core/shell.rs
    participant UI as ui/
    participant GH as GitHub API
    participant OMP as oh-my-posh CLI

    rect rgb(30, 40, 60)
        note over M,GH: 🚀 Startup
        M->>T: fetch_theme_list() [background task, 3× retry + backoff]
        T->>GH: GET /repos/JanDeDobbeleer/oh-my-posh/contents/themes
        GH-->>T: JSON theme list
        T-->>M: AppEvent::ThemesLoaded
        M->>A: init_themes(list)
        M->>A: load_shell_info()
    end

    rect rgb(30, 50, 40)
        note over M,UI: 🎨 Navigation & Preview (165 ms debounce)
        M->>A: input::handle_normal() → move_up / move_down
        A->>A: set preview_timer (5 ticks)
        A->>A: step_preview_timer() fires → trigger_preview()
        A->>T: download_theme() if not in local cache
        A->>PW: request(theme_path, columns)
        PW->>OMP: oh-my-posh print primary --config <theme>
        OMP-->>PW: ANSI-escaped stdout
        PW-->>A: poll_preview() → preview_output string
        A->>A: ansi_to_text() → cached_preview [parsed ONCE]
        A-->>UI: draw() reads cached_preview [every frame, no re-parse]
    end

    rect rgb(60, 30, 30)
        note over M,SH: ✅ Apply Theme  (Enter → Confirm → Enter)
        M->>A: input::handle_overlay() → do_apply()
        A->>SH: apply_theme(omp_path, theme_path, shell)
        SH->>SH: backup rc file → .posh-tui.bak
        SH->>SH: write managed block (# posh-tui:start … end)
        A->>A: save config (last_applied, favourites, zoom, recent)
    end

    rect rgb(50, 40, 20)
        note over M,SH: ↩️ Revert Options
        M->>A: u → do_undo()
        A->>SH: undo() → restore from .posh-tui.bak
        M->>A: U → do_soft_revert()
        A->>SH: soft_revert() → remove managed block only
        M->>A: Ctrl+U → do_hard_revert()
        A->>SH: hard_revert() → remove ALL oh-my-posh lines
    end
```

### Module responsibilities

| Module | Lines | Role |
|--------|------:|------|
| `ui/` | 1,003 | Rendering: layout, list, preview, immersive, ANSI parsing, modal overlays |
| `app/` | 894 | Central state model with sub-states, navigation, search, preview, shell operations |
| `core/shell.rs` | 465 | Shell detection, rc-file patching, backup management |
| `main.rs` | 168 | Entry point, terminal setup/teardown, event loop, CLI args |
| `core/themes.rs` | 191 | GitHub API theme list fetch, theme download, offline cache |
| `core/config.rs` | 135 | TOML-based config persistence (favourites, zoom, last_applied, recent) |
| `input/` | 185 | Key/mouse event dispatch, per-mode handlers (normal, search, immersive, overlay) |
| `preview.rs` | 83 | Async worker that spawns `oh-my-posh` CLI and returns stdout |
| `search.rs` | 97 | Fuzzy search wrapper around nucleo matcher |
| `core/error.rs` | 28 | `PoshError` enum via `thiserror` derive |

---

## 4. Module Reference

### 4.1 `main.rs` — Entry point & event loop

**Responsibilities:**
- Terminal initialization (raw mode, alternate screen, mouse capture)
- Background theme fetching with network/cached fallback
- Event loop running at ~30fps (33ms poll interval)
- CLI argument parsing (`--dry-run`, `--generate-completions`)
- Delegates key/mouse events to `input/` module

**Key items:**
- `main()` — Bootstraps cache directory, terminal, and background fetch task
- `run_app()` — Core loop: poll events → tick → draw → handle input → drain background channels
- `AppEvent` — Enum for background task communication (`ThemesLoaded`, `ThemeLoadError`)
- `Cli` — Clap struct for CLI args

### 4.2 `app/` — State model & behavior

**Responsibilities:**
- Holds all runtime state via composed sub-state structs
- Navigation (up/down/top/bottom/page), search, preview orchestration
- Shell operations (apply, undo, soft revert, hard revert)
- Immersive mode command simulation
- Config persistence

**Sub-states:**

| Sub-state | File | Fields | Methods |
|-----------|------|--------|---------|
| `ThemeState` | `theme_state.rs` | `themes`, `filtered`, `selected`, `favourites`, `show_favs`, `show_recent`, `last_applied` | `move_*`, `page_*`, `toggle_favourite`, `random_theme`, `init_themes`, `selected_theme`, `visible_themes` |
| `PreviewState` | `preview_state.rs` | `preview_output`, `cached_preview`, `preview_loading`, `worker`, `preview_width`, `terminal_width`, `scroll_offset`, `zoom_factor`, `preview_timer` | `scroll_*`, `zoom_*`, `step_preview_timer`, `effective_columns`, `trigger_preview`, `trigger_immersive_preview`, `poll_preview`, `edit_theme` |
| `SearchState` | `search_state.rs` | `search_query`, `fuzzy` | `apply_search`, `clear_search` |
| `ImmersiveState` | `immersive_state.rs` | `imm_input`, `imm_history`, `imm_cursor_tick` | `imm_submit`, `imm_backspace`, `imm_push` |

**Key types (in `mod.rs`):**
- `Mode` — Enum: Normal, Search, Confirm, Help, Immersive, SoftRevert, HardRevert, Message
- `App` — Central state struct composing all sub-states + app-level fields
- `ImmLine` / `ImmKind` — Immersive history line representation (`Prompt`, `Input`, `Output`)

**Orchestration methods (in `mod.rs`):**
- `new()` — Initializes state, restores config, spawns preview worker
- `save_config()` — Syncs sub-state fields back to `Config` and persists
- `tick()` — Advances immersive cursor blink counter
- `start_refresh()` / `poll_refresh()` — GitHub re-fetch via background channel
- `load_shell_info()` — Detects shell and loads `ShellInfo`
- `do_apply()` / `do_undo()` / `do_soft_revert()` / `do_hard_revert()` / `prepare_hard_revert()` — Shell operations

### 4.3 `input/` — Event handling

**Responsibilities:**
- Dispatches keyboard and mouse events to per-mode handlers
- Splits keybinding logic by application mode for extensibility

**Files:**
- `mod.rs` — `handle_key_event(app, key)` dispatcher + `handle_mouse(app, mouse)` (scroll only)
- `normal.rs` — `handle_normal()` — 20+ keybindings for `Mode::Normal`
- `search.rs` — `handle_search()` — Character input, backspace, cancel for `Mode::Search`
- `immersive.rs` — `handle_immersive()` — Typing, Enter, Ctrl+A, Esc for `Mode::Immersive`
- `overlay.rs` — `handle_overlay()` — Help, Confirm, SoftRevert, HardRevert, Message modes

> **Note:** `Event::Resize` stays in `main.rs::run_app` (needs `&mut Terminal` for `autoresize()`).

### 4.4 `ui/` — Rendering

**Responsibilities:**
- Declarative rendering of all UI elements
- ANSI escape sequence parsing via `ansi-to-tui` crate (parsing done in `app/`; `ui/` reads cached `Text<'static>`)
- Horizontal scroll with span-aware slicing (preserves styling)
- Modal overlays (help, confirm, soft revert, hard revert, message)

**Structure:**
- `mod.rs` — `pub fn draw()` dispatcher + `pub use ansi_to_text` re-export + insta snapshot test
- `components/` — `theme_list.rs`, `preview_pane.rs`, `search_bar.rs`, `status_bar.rs`
- `screens/` — `main_screen.rs` (two-pane layout), `immersive_screen.rs` (full-screen terminal)
- `overlays/` — `help.rs`, `confirm.rs`, `revert.rs` (soft + hard), `message.rs`
- `utilities/` — `ansi.rs` (`ansi_to_text`), `layout.rs` (`centered_rect`)

**Key functions:**
- `draw()` — Top-level dispatcher; switches to immersive if active, otherwise renders search bar + main screen + status bar + optional overlay
- `ansi_to_text()` — Wrapper around `ansi-to-tui::IntoText`; called from `app/preview_state.rs`

### 4.5 `core/shell.rs` — Shell integration

**Responsibilities:**
- Shell detection via `$SHELL` environment variable
- Rc file path resolution for bash, zsh, and fish
- Managed block patching (apply, replace, remove)
- Backup management and restoration

**Key types:**
- `Shell` — Enum: Bash, Zsh, Fish, Unknown
- `ShellInfo` — Current shell state: path, has_managed_block, backup_path

**Key functions:**
- `apply_theme()` — Creates backup, writes managed block (appends or replaces)
- `soft_revert()` — Removes only the posh-tui managed block
- `hard_revert()` — Removes all lines containing "oh-my-posh"
- `undo()` — Restores from backup file
- `which_omp()` — Resolves oh-my-posh binary path via `which` command

### 4.6 `core/themes.rs` — Theme sourcing

**Responsibilities:**
- Fetch theme list from GitHub Contents API with automatic retry
- Download theme files to local cache
- Save/load theme list cache for offline use

**Key functions:**
- `fetch_theme_list()` — Retries up to 3× (1s / 2s / 4s backoff), then returns last error
- `download_theme()` — Fetch raw theme file, cache to disk
- `save_theme_list_cache()` / `load_cached_theme_list()` — Offline cache support

### 4.7 `core/config.rs` — Configuration

**Responsibilities:**
- Persist user preferences to TOML
- Load/save with fault-tolerant defaults

**Key fields:**
- `last_applied: Option<String>` — Last applied theme name
- `favourites: HashSet<String>` — Favourited theme names
- `zoom_factor: f32` — Preview zoom level (0.5–3.0)
- `recent: Vec<String>` — Last 10 previewed theme names

### 4.8 `core/error.rs` — Error types

**Responsibilities:**
- Unified error type via `thiserror` derive

**Variants:**
- `Io` — `#[from] std::io::Error`
- `Http` — `#[from] reqwest::Error`
- `Json` — `#[from] serde_json::Error`
- `Toml` — `#[from] toml::de::Error`
- `TomlSer` — `#[from] toml::ser::Error`
- `Preview(String)` — Preview-specific errors
- `Shell(String)` — Shell operation errors
- `Config(String)` — Configuration errors

### 4.9 `preview.rs` — Preview pipeline

**Responsibilities:**
- Run `oh-my-posh print primary` asynchronously with 5-second timeout
- Return stdout as styled preview output

**Key types:**
- `PreviewWorker` — Tokio task + mpsc channel + shared Mutex for output
- `PreviewMsg` — `Load(path, width)` or `Quit`

### 4.10 `search.rs` — Fuzzy search

**Responsibilities:**
- Wrap nucleo matcher for fuzzy search
- Case-insensitive with smart normalization

---

## 5. Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `PgUp` | Page up (10 items) |
| `PgDn` | Page down (10 items) |

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

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Apply selected theme to shell config |
| `u` | Undo last apply (restores backup) |
| `U` | Soft revert — remove posh-tui block only |
| `Ctrl+U` | Hard revert — remove all oh-my-posh lines |
| `e` | Edit selected theme in `$EDITOR` |
| `f` | Toggle favourite |
| `F` | Toggle favourites-only view |
| `R` | Toggle recently viewed view |
| `x` | Jump to random theme |
| `r` | Refresh theme list from GitHub |
| `/` | Fuzzy search |
| `n` / `N` | Dismiss Nerd Font warning banner |
| `?` | Help overlay |
| `q` / `Ctrl+C` | Quit |
| `Scroll` | Scroll up/down the theme list |

---

## 6. Configuration & Cache

### Config file

Location: `~/.config/posh-tui/config.toml`

```toml
last_applied = "catppuccin"

favourites = [
  "catppuccin",
  "tokyo-night",
  "agnoster",
]

zoom_factor = 1.0

recent = [
  "catppuccin",
  "tokyo-night",
  "powerlevel10k",
]
```

### Cache locations

```
~/.cache/posh-tui/
├── themes/                    # Downloaded .omp.json files
│   ├── catppuccin.omp.json
│   ├── tokyo-night.omp.json
│   └── ...
└── themes_cache.json          # Cached theme list for offline use
```

### Shell rc file patching

When you apply a theme, posh-tui writes a managed block to your shell config:

```bash
# posh-tui:start
eval "$(/home/user/.local/bin/oh-my-posh init bash --config /home/user/.cache/posh-tui/themes/catppuccin.omp.json)"
# posh-tui:end
```

**Backup locations:**
- `~/.bashrc.posh-tui.bak`
- `~/.zshrc.posh-tui.bak`
- `~/.config/fish/config.fish.posh-tui.bak`

---

## 7. Installation

### Prerequisites

- [Oh My Posh](https://ohmyposh.dev/docs/installation/linux) installed and in `$PATH`
- A [Nerd Font](https://www.nerdfonts.com) installed and set in your terminal
- Rust 1.70+ (for building from source)

### Curl install (recommended)

```bash
curl -sSf https://raw.githubusercontent.com/RayenBHK/posh-tui/main/install.sh | bash
```

The installer:
1. Detects your OS and architecture
2. Fetches the latest release from GitHub
3. Installs to `~/.local/bin/posh-tui`

### Cargo install

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

### Usage

```bash
posh-tui
```

The app opens instantly. Themes load from GitHub in the background with a spinner.

---

## 8. Development

### Project structure

```
posh-tui/
├── src/
│   ├── main.rs              # Entry point, event loop, CLI
│   ├── preview.rs           # Async oh-my-posh worker
│   ├── search.rs            # Nucleo fuzzy search
│   ├── app/                 # State model (sub-state composition)
│   │   ├── mod.rs           #   App struct, Mode, ImmLine, ImmKind, orchestration
│   │   ├── theme_state.rs   #   Theme list + navigation
│   │   ├── preview_state.rs #   Preview + zoom + scroll
│   │   ├── search_state.rs  #   Search query + fuzzy
│   │   └── immersive_state.rs #  Immersive mode + fake shell
│   ├── core/                # Domain logic
│   │   ├── mod.rs           #   Re-exports
│   │   ├── config.rs        #   TOML config persistence
│   │   ├── error.rs         #   PoshError (thiserror)
│   │   ├── shell.rs         #   Shell detection, rc-file patching
│   │   └── themes.rs        #   GitHub API, cache, download
│   ├── input/               # Event handling
│   │   ├── mod.rs           #   Key/mouse dispatcher
│   │   ├── normal.rs        #   Normal mode keybindings
│   │   ├── search.rs        #   Search mode keybindings
│   │   ├── immersive.rs     #   Immersive mode keybindings
│   │   └── overlay.rs       #   Overlay mode keybindings
│   └── ui/                  # Rendering
│       ├── mod.rs           #   draw() dispatcher + insta test
│       ├── components/      #   theme_list, preview_pane, search_bar, status_bar
│       ├── screens/         #   main_screen, immersive_screen
│       ├── overlays/        #   help, confirm, revert, message
│       └── utilities/       #   ansi (ansi_to_text), layout (centered_rect)
├── .github/workflows/
│   └── release.yml          # CI/CD for cross-platform releases
├── Cargo.toml               # Dependencies and build config
├── install.sh               # Curl installer script
└── DOCUMENTATION.md         # This file
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized + stripped)
cargo build --release
```

### Running

```bash
cargo run
cargo run --release
```

---

## 9. CI/CD

### GitHub Actions workflow

File: `.github/workflows/release.yml`

**Trigger:** Push a tag matching `v*` (e.g., `git tag v0.4.2 && git push --tags`)

**Build matrix:**

| Target | OS | Artifact | Toolchain |
|--------|-----|----------|-----------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | `posh-tui-x86_64-linux` | cargo |
| `aarch64-unknown-linux-gnu` | ubuntu-latest | `posh-tui-aarch64-linux` | cross |
| `x86_64-apple-darwin` | macos-latest | `posh-tui-x86_64-macos` | cargo |
| `aarch64-apple-darwin` | macos-latest | `posh-tui-aarch64-macos` | cargo |
| `x86_64-pc-windows-msvc` | windows-latest | `posh-tui-x86_64-windows.exe` | cargo |

> The `aarch64-unknown-linux-gnu` target uses [`cross`](https://github.com/cross-rs/cross) for Docker-based cross-compilation.

**Pipeline:**
1. Checkout → Rust toolchain → Cache dependencies
2. Build release binary for each target
3. Upload artifacts
4. Download artifacts → Generate SHA256 checksums → Extract changelog notes
5. Create GitHub release with binaries + checksums + release notes

### Releasing

```bash
git tag v0.4.2
git push --tags
```

The CI will automatically build and attach binaries to a new GitHub release.

---

## 10. Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.29 | TUI framework — layout, widgets, rendering |
| `crossterm` | 0.28 | Terminal raw mode, alternate screen, keyboard events |
| `tokio` | 1 (full) | Async runtime, process spawning, channels |
| `reqwest` | 0.12 (json, rustls-tls) | HTTP client for GitHub API |
| `serde` | 1 (derive) | Serialization/deserialization |
| `serde_json` | 1 | JSON parsing for GitHub API and theme cache |
| `toml` | 0.8 | TOML config file parsing |
| `dirs-next` | 2 | Cross-platform config/cache/home directory resolution |
| `nucleo` | 0.5 | Fuzzy matcher for theme search |
| `ansi-to-tui` | 7 | ANSI escape sequence parsing to ratatui Text |
| `rand` | 0.9 | Random theme selection |
| `clap` | 4.5 | CLI argument parsing and shell completion generation |
| `clap_complete` | 4.5 | Shell completion generation |
| `thiserror` | 1 | Ergonomic error type derive macros |

**Dev-dependencies:**

| Crate | Version | Purpose |
|-------|---------|---------|
| `tempfile` | 3.10 | Temporary directories for shell integration tests |
| `wiremock` | 0.6.5 | HTTP mocking for GitHub API tests |
| `insta` | 1.39 | UI snapshot testing |

---

## 11. Testing

### Running tests

```bash
cargo test
```

### Linting

```bash
cargo clippy -- -D warnings
```

### Snapshot review

```bash
cargo insta review
```

### Test coverage — 32 tests total

#### `core/shell.rs` — 15 tests (rc-file patching logic)

**`replace_managed_block` (3 tests):**
- Replaces existing managed block with new content
- Preserves surrounding config when no block exists
- Handles complex multi-line rc files

**`remove_managed_block` (4 tests):**
- Removes managed block while preserving other content
- Handles files with no managed block
- Handles empty files
- Handles files containing only the managed block

**`remove_all_omp_lines` (4 tests):**
- Removes all lines containing "oh-my-posh"
- Removes manual oh-my-posh lines
- Handles files with no oh-my-posh lines
- Handles empty files

**`backup_path_for` (3 tests):**
- Generates correct backup path for `.bashrc`
- Generates correct backup path for `.zshrc`
- Generates correct backup path for fish config

**`e2e_apply_and_undo` (1 test):**
- End-to-end apply → undo cycle with real temp directory

#### `core/config.rs` — 4 tests

- `test_defaults_on_missing_file` — `Config::load()` returns sensible defaults without panicking
- `test_push_recent_dedup` — pushing the same theme name twice results in one entry
- `test_push_recent_truncate` — pushing 12 names keeps the list at ≤ 10 entries
- `test_push_recent_order` — most recently pushed name appears first

#### `search.rs` — 4 tests

- `test_exact_match_included` — querying "catppuccin" includes "catppuccin" in results
- `test_no_match_returns_empty` — nonsense query returns empty results
- `test_case_insensitive` — "CATPPUCCIN" matches "catppuccin"
- `test_empty_query_returns_all` — empty query returns all theme names

#### `app/theme_state.rs` — 3 tests (async, `#[tokio::test]`)

- `test_toggle_favourite_adds_and_removes` — double-toggle leaves favourites empty
- `test_move_up_at_top_clamps` — `move_up()` at index 0 stays at 0
- `test_visible_themes_favs_filter` — `show_favs=true` returns only favourited themes

#### `app/preview_state.rs` — 3 tests (async, `#[tokio::test]`)

- `test_zoom_in_bounded` — `zoom_in()` never exceeds maximum zoom factor
- `test_zoom_out_bounded` — `zoom_out()` never goes below minimum zoom factor
- `test_zoom_reset` — `zoom_reset()` restores `zoom_factor` to `1.0`

#### `core/themes.rs` — 2 tests (async, `#[tokio::test]`, wiremock)

- `test_fetch_theme_list_success_after_2_errors` — mock returns 500 twice then 200
- `test_fetch_theme_list_exhaustion` — all 500s, asserts error returned

#### `ui/mod.rs` — 1 test (async, `#[tokio::test]`, insta snapshot)

- `test_ui_snapshot` — renders 80×24 TUI and compares against committed snapshot

### What's still not tested (future additions)

- `input/` handlers (keybinding dispatch, mouse handling)
- `preview.rs` (PreviewWorker subprocess)
- Integration tests for `do_apply()` / `do_undo()` end-to-end
- `main.rs` event loop (requires terminal mock)
