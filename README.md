# posh-tui

A Rust terminal UI for browsing and previewing Oh My Posh themes with a live ANSI preview and immersive shell-style mode.

## Features
- Browse all Oh My Posh themes from GitHub.
- Live preview using the `oh-my-posh` CLI.
- Favourites and favourites-only view.
- Search (substring match).
- Horizontal scroll and zoom for the preview pane.
- Immersive full-screen preview mode.

## Requirements
- Rust toolchain with `cargo`.
- `oh-my-posh` installed and available in PATH.
- Network access to GitHub for theme list and downloads.

## Install & Run
```bash
cargo run
```

For a faster build:
```bash
cargo run --release
```

## Usage (Keybindings)
Normal mode:
- `↑/↓` or `j/k`: move selection
- `g` / `G`: top / bottom
- `PgUp` / `PgDn`: page up / down
- `Space`: render preview in the right pane
- `p`: immersive full-screen preview
- `Enter`: confirm apply (currently records last applied only)
- `f`: toggle favourite
- `F`: favourites view
- `<` / `>`: horizontal scroll in preview
- `-` / `=` / `0`: zoom out / in / reset
- `/`: search
- `?`: help
- `q` or `Ctrl+C`: quit

Search mode:
- `Esc`: cancel search
- `Backspace`: delete last char
- `Down/Up`: exit search mode

## Notes & Limitations
- Theme apply/undo and refresh are not implemented yet (placeholders in UI/help).
- Favourites are not persisted between sessions.
- If `oh-my-posh` is missing, preview will show an error message.
- Theme list is fetched from GitHub on startup.

## Project Layout
- `src/main.rs`: app startup and input handling
- `src/app.rs`: application state and core behaviors
- `src/ui.rs`: rendering and ANSI parsing
- `src/themes.rs`: GitHub theme list and downloads
- `src/preview.rs`: oh-my-posh CLI integration
- `src/error.rs`: error types

## License
No license specified yet.
