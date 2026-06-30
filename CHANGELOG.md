# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
