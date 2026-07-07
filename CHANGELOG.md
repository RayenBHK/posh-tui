# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Snapshot test `test_ui_snapshot` was non-deterministic: forced `hide_font_warning = true` so the Nerd Font banner never appears in the snapshot output
- Snapshot test was calling `app.save_config()` which overwrote the developer's real `~/.config/posh-tui/config.toml`; removed the call
- Download and preview errors were rendered as an empty preview pane instead of showing the error text; the preview pane now detects error-prefixed output and renders it in red
- `preview.rs` worker message had duplicate `"preview error: Preview error: …"` prefix; now errors from `PoshError::Display` are written verbatim
- Removed dead `PreviewMsg::Quit` variant (never sent, was suppressed with `#[allow(dead_code)]`)
- GitHub API rate limits (403 Forbidden, 429 Too Many Requests) were not explicitly detected in `themes.rs`, contradicting the v0.4.1 changelog entry; now returns `PoshError::RateLimit` with a user-readable message
- Dead `_title` local variable in `ui/components/search_bar.rs` (leftover from module split) removed
- `F` (toggle favourites filter) and `R` (toggle recent filter) no longer leave a stale out-of-bounds `selected` index when switching to a shorter visible list
- `r` (refresh) now resets `selected` to 0 so the newly-loaded list starts at the top

### Added
- `PoshError::RateLimit(String)` variant for explicit GitHub API rate-limit errors
- `.github/workflows/ci.yml` — CI gate that runs `cargo clippy -- -D warnings` and `cargo test` on every push to `main` and on pull requests
- Shared `dummy_themes()` test helper in `app/mod.rs`; `theme_state::tests` and `preview_state::tests` now delegate to it instead of duplicating

### Changed
- `core/mod.rs` now only declares modules (`pub mod`); removed the four glob `pub use X::*` re-exports that were shims for the old flat structure
- `main.rs` now uses fully-qualified `crate::core::*` paths instead of the `pub use crate::core::*` glob shim
- Documented the `app` → `ui::ansi_to_text` intra-crate coupling in `preview_state.rs` with a future-refactor note

## [0.4.2] - 2026-07-01

### Changed
- Decomposed `app.rs` (832 lines) into `src/app/` module with `ThemeState`, `PreviewState`, `SearchState`, and `ImmersiveState` sub-states using nested pub fields
- Split `ui.rs` (903 lines) into `src/ui/` with `components/`, `screens/`, `overlays/`, and `utilities/` sub-modules
- Extracted input handlers from `main.rs` (323 lines) into `src/input/` module (`normal`, `search`, `immersive`, `overlay`); `main.rs` reduced to 168 lines
- Moved `config.rs`, `themes.rs`, `shell.rs`, `error.rs` into `src/core/` directory
- Migrated `PoshError` from hand-written `Display`/`From` impls to `thiserror` derive macros
- Cleaned hardcoded machine-specific path in `shell.rs::which_omp()`
- Insta snapshot test now forces `zoom_factor = 1.0` for deterministic CI output

### Added
- `thiserror = "1"` dependency for ergonomic error types

## [0.4.1] - 2026-06-30

### Added
- `--fail-fast: false` added to GitHub Actions workflow matrix to prevent one failing target from canceling others
- `checksums.txt` containing SHA256 hashes is now generated and attached to GitHub releases
- `lto = "fat"` and `codegen-units = 1` added to `[profile.release]` to optimize binary size (~4.2MB → ~3MB)
- `tokio::time::timeout` (5s) added to preview subprocess to prevent TUI hangs if `oh-my-posh` hangs
- `n` and `N` keybindings added to `README.md` and `DOCUMENTATION.md`
- `e`, `n/N`, `U`, and `Ctrl+U` keys added to the help overlay inside the TUI

### Changed
- `config.save()` now explicitly logs write errors in debug builds instead of silently failing

### Fixed
- Handled `403 FORBIDDEN` and `429 TOO_MANY_REQUESTS` GitHub API rate limits explicitly in `themes.rs`
- Removed residual `#[allow(dead_code)]` suppressions

## [0.4.0] - 2026-06-30

### Added
- Live theme editing via `e` key — opens `$EDITOR` for selected theme
- Nerd Font diagnostic warning banner (dismissable)
- CLI argument parsing via `clap` with `--dry-run` and `--generate-completions`
- UI snapshot testing via `insta`
- 4 new config tests, 4 search tests, 6 app tests (32 total)
- Automated GitHub Releases for 5 platform targets via CI/CD pipeline

### Changed
- Switched from `native-tls` to `rustls-tls` for cross-platform compatibility
- Bumped version to 0.4.0

### Fixed
- Windows CI build failure (Bash `if` in PowerShell context)
- ARM64 cross-compilation failure (`openssl-sys` dependency removed)

## [0.2.0] - 2026-06-30

### Added
- ANSI preview caching as `Text<'static>` (parsed once per update)
- GitHub API retry with 3× exponential backoff (1s/2s/4s)
- 14 new unit tests for config, search, and app modules (28 total)
- ARM Linux CI target (`aarch64-unknown-linux-gnu` via `cross`)

### Changed
- Bumped Cargo.toml version to 0.2.0 with full crate metadata
- Refactored `&PathBuf` to `&Path` across `shell.rs` and `themes.rs`

### Fixed
- Dead code removed: `refresh_tx` field and `ImmKind::Blank` variant
- `cargo clippy -D warnings` clean — zero suppressions
