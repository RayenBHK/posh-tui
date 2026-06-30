use crate::app::{App, ImmKind, Mode};
use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    if app.mode == Mode::Immersive {
        draw_immersive(frame, app);
        return;
    }

    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_search(frame, app, outer[0]);
    draw_main(frame, app, outer[1]);
    draw_statusbar(frame, app, outer[2]);

    match app.mode {
        Mode::Help => draw_help(frame, area),
        Mode::Confirm => draw_confirm(frame, app, area),
        Mode::SoftRevert => draw_soft_revert(frame, app, area),
        Mode::HardRevert => draw_hard_revert(frame, app, area),
        Mode::Message => draw_message(frame, app, area),
        _ => {}
    }
}

fn draw_search(frame: &mut Frame, app: &App, area: Rect) {
    let is_fav = app
        .selected_theme()
        .map(|t| app.favourites.contains(&t.name))
        .unwrap_or(false);

    let fav_indicator = if is_fav { " ★" } else { "" };

    let applied_label = match &app.last_applied {
        Some(name) => format!(" applied: {name} "),
        None => String::new(),
    };

    let _title = if app.show_favs {
        format!(" ★ favourites{fav_indicator} ")
    } else {
        format!(" posh-tui{fav_indicator}  {applied_label}")
    };

    let query = if app.mode == Mode::Search {
        format!("/ {}_", app.search_query)
    } else if app.search_query.is_empty() {
        " type / to search...".to_string()
    } else {
        format!("/ {}", app.search_query)
    };

    let style = if app.mode == Mode::Search {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if app.loading {
        " posh-tui — loading... ".to_string()
    } else if app.refreshing {
        " posh-tui — refreshing... ".to_string()
    } else if app.show_favs {
        format!(" ★ favourites{fav_indicator} ")
    } else if app.show_recent {
        format!(" ⏱ recent{fav_indicator} ")
    } else {
        format!(" posh-tui{fav_indicator}  {applied_label}")
    };

    frame.render_widget(
        Paragraph::new(query)
            .style(style)
            .block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn draw_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut main_area = area;
    if !app.hide_font_warning {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        main_area = v_chunks[0];

        let banner = Paragraph::new(
            " ℹ️  Ensure a Nerd Font is installed in your terminal. Press 'N' to dismiss. ",
        )
        .style(Style::default().fg(Color::Black).bg(Color::Yellow));
        frame.render_widget(banner, v_chunks[1]);
    }

    let list_pct: u16 = if main_area.width < 100 { 28 } else { 22 };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(list_pct),
            Constraint::Percentage(100 - list_pct),
        ])
        .split(main_area);

    app.preview_width = chunks[1].width;
    app.terminal_width = main_area.width;

    draw_list(frame, app, chunks[0]);
    draw_preview(frame, app, chunks[1]);
}

fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    // show spinner while loading
    if app.loading || app.refreshing {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = (app.imm_cursor_tick as usize / 3) % spinner_frames.len();
        let spinner = spinner_frames[frame_idx];
        let label = if app.refreshing {
            "refreshing..."
        } else {
            "loading themes..."
        };

        let block = Block::default().borders(Borders::ALL).title(" themes ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(format!("\n  {spinner} {label}"))
                .style(Style::default().fg(Color::Yellow)),
            inner,
        );
        return;
    }

    let visible = app.visible_themes();
    let total = visible.len();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|t| {
            let star = if app.favourites.contains(&t.name) {
                "★"
            } else {
                " "
            };
            let applied = match &app.last_applied {
                Some(name) if name == &t.name => "✓ ",
                _ => "  ",
            };
            ListItem::new(format!("{star}{applied}{}", t.name))
        })
        .collect();

    let title = if app.show_favs {
        format!(" favs ({total}) ")
    } else if app.show_recent {
        format!(" recent ({total}) ")
    } else {
        format!(" themes ({total}) ")
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(if total > 0 { Some(app.selected) } else { None });
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    let theme_name = app
        .selected_theme()
        .map(|t| format!(" preview — {} ", t.name))
        .unwrap_or(" preview ".to_string());

    let zoom_label = if (app.zoom_factor - 1.0).abs() < 0.01 {
        String::new()
    } else {
        format!(" zoom {:.0}% ", (100.0 / app.zoom_factor) as u16)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{theme_name}{zoom_label}"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.preview_loading {
        frame.render_widget(Paragraph::new("  loading..."), inner);
        return;
    }

    if app.preview_output.is_empty() {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("  press ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::styled(" to preview  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled("p", Style::default().fg(Color::Cyan)),
            Span::styled(" for immersive mode", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(hint, inner);
        return;
    }

    let offset = app.scroll_offset as usize;
    // Use the pre-parsed cache populated by poll_preview(); never re-parses on render.
    let text = app.cached_preview.clone().unwrap_or_default();

    // scroll by skipping N chars worth of spans per line — preserves styling
    let scrolled: Vec<Line> = text
        .lines
        .into_iter()
        .map(|line| {
            let mut remaining_skip = offset;
            let mut new_spans: Vec<Span<'static>> = Vec::new();

            for span in line.spans {
                let content = span.content.to_string();
                let char_count = content.chars().count();

                if remaining_skip >= char_count {
                    // skip entire span
                    remaining_skip -= char_count;
                } else if remaining_skip > 0 {
                    // partial skip — take chars after the offset
                    let visible: String = content.chars().skip(remaining_skip).collect();
                    remaining_skip = 0;
                    if !visible.is_empty() {
                        new_spans.push(Span::styled(visible, span.style));
                    }
                } else {
                    // no skip needed — take whole span
                    new_spans.push(Span::styled(content, span.style));
                }
            }

            Line::from(new_spans)
        })
        .collect();

    let para = Paragraph::new(Text::from(scrolled)).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);

    // scroll position indicator
    if offset > 0 {
        let label = format!(" ◀ +{offset} ");
        let ind_area = Rect {
            x: inner.x,
            y: inner.y,
            width: (label.len() as u16).min(inner.width),
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(Color::DarkGray)),
            ind_area,
        );
    }
}

fn draw_immersive(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.terminal_width = area.width;

    // full black background
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    // top hint bar — 1 line
    let theme_name = app
        .selected_theme()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let hint = Line::from(vec![
        Span::styled(
            " immersive — ",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            &*theme_name,
            Style::default().fg(Color::Green).bg(Color::Black),
        ),
        Span::styled(
            "   Esc: back",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            "   Ctrl+A: apply",
            Style::default().fg(Color::Yellow).bg(Color::Black),
        ),
        Span::styled(
            "   Enter: run command",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            "   type 'help' for commands",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
    ]);
    let hint_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().bg(Color::Black)),
        hint_area,
    );

    // separator under hint
    let sep1_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        sep1_area,
    );

    // input line pinned at very bottom
    let cursor_char = if app.imm_cursor_tick < 30 { "█" } else { " " };
    let input_line = format!("❯ {}{}", app.imm_input, cursor_char);
    let input_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(input_line).style(Style::default().fg(Color::Green).bg(Color::Black)),
        input_area,
    );

    // separator above input
    let sep2_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        sep2_area,
    );

    // shell history area — between the two separators
    let history_area = Rect {
        x: area.x,
        y: area.y + 2,
        width: area.width,
        height: area.height.saturating_sub(4), // hint + sep1 + sep2 + input
    };

    // build all lines from history
    let mut all_lines: Vec<Line<'static>> = Vec::new();

    for entry in &app.imm_history {
        match entry.kind {
            ImmKind::Prompt => {
                // render the actual oh-my-posh ANSI prompt
                let parsed = ansi_to_text(&entry.content);
                for line in parsed.lines {
                    all_lines.push(line);
                }
            }
            ImmKind::Input => {
                // show what was typed, indented with a dim arrow
                all_lines.push(Line::from(vec![
                    Span::styled("❯ ", Style::default().fg(Color::Green).bg(Color::Black)),
                    Span::styled(
                        entry.content.clone(),
                        Style::default().fg(Color::White).bg(Color::Black),
                    ),
                ]));
            }
            ImmKind::Output => {
                // parse ANSI in output too (ls colors, git colors etc.)
                let parsed = ansi_to_text(&entry.content);
                for line in parsed.lines {
                    // ensure bg is black
                    let styled: Vec<Span<'static>> = line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, s.style.bg(Color::Black)))
                        .collect();
                    all_lines.push(Line::from(styled));
                }
            }
        }
    }

    // always scroll to show latest lines — like a real terminal
    let visible_h = history_area.height as usize;
    let skip = all_lines.len().saturating_sub(visible_h);
    let visible: Vec<Line<'static>> = all_lines.into_iter().skip(skip).collect();

    frame.render_widget(
        Paragraph::new(Text::from(visible))
            .style(Style::default().fg(Color::White).bg(Color::Black)),
        history_area,
    );
}

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let text = match app.mode {
        Mode::Search  => " Esc: cancel  ↑↓: navigate",
        Mode::Normal  => " Space: preview  p: immersive  Enter: apply  u: undo  U: revert  Ctrl+U: hard revert  r: refresh  /: search  x: random  R: recent  ?: help  q: quit",
        Mode::Confirm => " Enter: confirm  Esc: cancel",
        Mode::Help    => " ?: close",
        Mode::Message => " any key: close",
        _             => "",
    };
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(52, 80, area);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            " posh-tui — keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ↑/k  ↓/j   ", Style::default().fg(Color::Yellow)),
            Span::raw("navigate list"),
        ]),
        Line::from(vec![
            Span::styled(" g / G      ", Style::default().fg(Color::Yellow)),
            Span::raw("top / bottom"),
        ]),
        Line::from(vec![
            Span::styled(" PgUp/PgDn  ", Style::default().fg(Color::Yellow)),
            Span::raw("page scroll"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Space      ", Style::default().fg(Color::Green)),
            Span::raw("preview theme in pane"),
        ]),
        Line::from(vec![
            Span::styled(" p          ", Style::default().fg(Color::Cyan)),
            Span::raw("immersive full-screen preview"),
        ]),
        Line::from(vec![
            Span::styled(" Enter      ", Style::default().fg(Color::Green)),
            Span::raw("apply theme to shell"),
        ]),
        Line::from(vec![
            Span::styled(" u          ", Style::default().fg(Color::Red)),
            Span::raw("undo last apply"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" < / >      ", Style::default().fg(Color::Cyan)),
            Span::raw("scroll preview left / right"),
        ]),
        Line::from(vec![
            Span::styled(" - / =      ", Style::default().fg(Color::Cyan)),
            Span::raw("zoom out / in"),
        ]),
        Line::from(vec![
            Span::styled(" 0          ", Style::default().fg(Color::Cyan)),
            Span::raw("reset zoom"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" f          ", Style::default().fg(Color::Cyan)),
            Span::raw("toggle favourite"),
        ]),
        Line::from(vec![
            Span::styled(" F          ", Style::default().fg(Color::Cyan)),
            Span::raw("favourites view"),
        ]),
        Line::from(vec![
            Span::styled(" R          ", Style::default().fg(Color::Cyan)),
            Span::raw("recently viewed"),
        ]),
        Line::from(vec![
            Span::styled(" x          ", Style::default().fg(Color::Cyan)),
            Span::raw("random theme"),
        ]),
        Line::from(vec![
            Span::styled(" /          ", Style::default().fg(Color::Cyan)),
            Span::raw("search themes"),
        ]),
        Line::from(vec![
            Span::styled(" r          ", Style::default().fg(Color::Cyan)),
            Span::raw("refresh theme list from GitHub"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ?          ", Style::default().fg(Color::DarkGray)),
            Span::raw("toggle help"),
        ]),
        Line::from(vec![
            Span::styled(" q / Ctrl+C ", Style::default().fg(Color::DarkGray)),
            Span::raw("quit"),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" help "))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_confirm(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 45, area);
    frame.render_widget(Clear, popup);

    let name = app
        .selected_theme()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "unknown".into());

    let (shell_name, rc, backup) = match &app.shell_info {
        Some(i) => (
            i.shell.name().to_string(),
            i.rc_path.display().to_string(),
            i.backup_path.display().to_string(),
        ),
        None => ("unknown".into(), "unknown".into(), "unknown".into()),
    };

    let theme_path = app.cache_dir.join(
        app.selected_theme()
            .map(|t| t.filename.as_str())
            .unwrap_or(""),
    );

    let init_line = match &app.shell_info {
        Some(i) => i.shell.init_line(&theme_path),
        None => String::new(),
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Apply theme: "),
            Span::styled(
                &*name,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Shell:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(shell_name, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  File:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(rc, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Backup: ", Style::default().fg(Color::DarkGray)),
            Span::styled(backup, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Writes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(init_line, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Green)),
            Span::raw(": confirm   "),
            Span::styled("Esc", Style::default().fg(Color::Red)),
            Span::raw(": cancel"),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" apply theme "),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_soft_revert(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup);

    let (rc, has) = match &app.shell_info {
        Some(i) => (i.rc_path.display().to_string(), i.has_managed),
        None => ("unknown".into(), false),
    };

    let lines = if has {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Soft revert",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Removes: ", Style::default().fg(Color::DarkGray)),
                Span::raw("posh-tui managed block only"),
            ]),
            Line::from(vec![
                Span::styled("  File:    ", Style::default().fg(Color::DarkGray)),
                Span::styled(rc, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Lines removed:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "    # posh-tui:start",
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                "    eval \"$(oh-my-posh init ...)\"",
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                "    # posh-tui:end",
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Enter", Style::default().fg(Color::Green)),
                Span::raw(": confirm   "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(": cancel"),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No posh-tui block found in your rc file.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Nothing to remove.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Esc", Style::default().fg(Color::Yellow)),
                Span::raw(": close"),
            ]),
        ]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" soft revert "),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_hard_revert(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(55, 55, area);
    frame.render_widget(Clear, popup);

    let rc = match &app.shell_info {
        Some(i) => i.rc_path.display().to_string(),
        None => "unknown".into(),
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ⚠ Hard revert",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "  Removes ALL oh-my-posh lines, including",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  ones you may have added manually.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  File: ", Style::default().fg(Color::DarkGray)),
            Span::styled(rc, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Lines that will be deleted:",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    if app.hard_revert_preview.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (none found — config is already clean)",
            Style::default().fg(Color::Green),
        )));
    } else {
        for (num, content) in &app.hard_revert_preview {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    line {num}: "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(content.clone(), Style::default().fg(Color::Red)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Enter", Style::default().fg(Color::Green)),
        Span::raw(": confirm   "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(": cancel"),
    ]));

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" hard revert "),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_message(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(48, 38, area);
    frame.render_widget(Clear, popup);

    let color = if app.message_is_err {
        Color::Red
    } else {
        Color::Green
    };
    let title = if app.message_is_err {
        " error "
    } else {
        " done "
    };

    let mut lines = vec![Line::from("")];
    for l in app.message.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {l}"),
            Style::default().fg(color),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  any key", Style::default().fg(Color::DarkGray)),
        Span::raw(": close"),
    ]));

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub fn ansi_to_text(s: &str) -> Text<'static> {
    s.to_string().into_text().unwrap_or_default()
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::themes::Theme;
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ui_snapshot() {
        let themes = vec![
            Theme {
                name: "theme1".to_string(),
                filename: "theme1.json".to_string(),
                local: None,
                raw_url: "".into(),
            },
            Theme {
                name: "theme2".to_string(),
                filename: "theme2.json".to_string(),
                local: None,
                raw_url: "".into(),
            },
        ];
        let mut app = App::new(themes, PathBuf::from("/tmp"));
        app.selected = 0;
        app.search_query = "".to_string();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            content.push('\n');
        }

        insta::assert_snapshot!(content);
    }
}
