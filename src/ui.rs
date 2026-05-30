use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use crate::app::{App, ImmKind, Mode};

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

    if app.mode == Mode::Help    { draw_help(frame, area); }
    if app.mode == Mode::Confirm { draw_confirm(frame, app, area); }
}

fn draw_search(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.show_favs { " ★ favourites " } else { " posh-tui " };
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

    let block = Paragraph::new(query)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(block, area);
}

fn draw_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let list_pct: u16 = if area.width < 100 { 28 } else { 22 };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(list_pct),
            Constraint::Percentage(100 - list_pct),
        ])
        .split(area);

    app.preview_width  = chunks[1].width;
    app.terminal_width = area.width;

    draw_list(frame, app, chunks[0]);
    draw_preview(frame, app, chunks[1]);
}

fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_themes();
    let total   = visible.len();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|t| {
            let star = if app.favourites.contains(&t.name) { "★ " } else { "  " };
            ListItem::new(format!("{star}{}", t.name))
        })
        .collect();

    let title = if app.show_favs {
        format!(" favs ({total}) ")
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
    let theme_name = app.selected_theme()
        .map(|t| format!(" preview — {} ", t.name))
        .unwrap_or(" preview ".to_string());

    let zoom_label = if (app.zoom_factor - 1.0).abs() < 0.01 {
        String::new()
    } else {
        format!(" zoom {:.0}% ", (100.0 / app.zoom_factor) as u16)
    };

    let title = format!("{theme_name}{zoom_label}");
    let block = Block::default().borders(Borders::ALL).title(title);
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

    let text   = ansi_to_text(&app.preview_output);
    let offset = app.scroll_offset;

    // apply horizontal scroll by slicing each line
    let scrolled: Vec<Line> = text.lines.into_iter().map(|line| {
        let full: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let chars: Vec<char> = full.chars().collect();
        let start = (offset as usize).min(chars.len());
        let visible_str: String = chars[start..].iter().collect();

        // re-parse the sliced string with styles preserved best-effort
        Line::from(Span::raw(visible_str))
    }).collect();

    let para = Paragraph::new(Text::from(scrolled))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, inner);

    // scroll indicator
    if offset > 0 {
        let indicator = Paragraph::new(format!(" ◀ {offset}"))
            .style(Style::default().fg(Color::DarkGray));
        let ind_area = Rect {
            x: inner.x,
            y: inner.y,
            width: (format!(" ◀ {offset}").len() as u16).min(inner.width),
            height: 1,
        };
        frame.render_widget(indicator, ind_area);
    }
}

fn draw_immersive(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.terminal_width = area.width;

    // full black background
    let bg = Block::default()
        .style(Style::default().bg(Color::Black));
    frame.render_widget(bg, area);

    // header bar
    let theme_name = app.selected_theme()
        .map(|t| t.name.as_str())
        .unwrap_or("unknown");
    let header_text = format!(
        " immersive preview — {}   Esc: back · Enter: apply · type commands to test",
        theme_name
    );
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black));
    let header_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
    frame.render_widget(header, header_area);

    // shell area — everything below header, above input line
    let shell_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(3),
    };

    // collect all lines to render
    let mut all_lines: Vec<Line> = Vec::new();

    for entry in &app.imm_history {
        match entry.kind {
            ImmKind::Prompt => {
                let parsed = ansi_to_text(&entry.content);
                for line in parsed.lines {
                    all_lines.push(line);
                }
            }
            ImmKind::Input => {
                all_lines.push(Line::from(vec![
                    Span::styled("❯ ", Style::default().fg(Color::Green)),
                    Span::raw(entry.content.clone()),
                ]));
            }
            ImmKind::Output => {
                let parsed = ansi_to_text(&entry.content);
                for line in parsed.lines {
                    all_lines.push(line);
                }
            }
            ImmKind::Blank => {
                all_lines.push(Line::from(""));
            }
        }
    }

    // scroll so latest lines are always visible
    let visible_height = shell_area.height as usize;
    let skip = if all_lines.len() > visible_height {
        all_lines.len() - visible_height
    } else {
        0
    };
    let visible_lines: Vec<Line> = all_lines.into_iter().skip(skip).collect();

    let shell_para = Paragraph::new(Text::from(visible_lines))
        .style(Style::default().fg(Color::White).bg(Color::Black));
    frame.render_widget(shell_para, shell_area);

    // input line at bottom
    let cursor_char = if app.imm_cursor_tick < 30 { "█" } else { " " };
    let input_line = format!("❯ {}{}", app.imm_input, cursor_char);
    let input_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };
    let input_widget = Paragraph::new(input_line)
        .style(Style::default().fg(Color::Green).bg(Color::Black));
    frame.render_widget(input_widget, input_area);

    // thin separator above input
    let sep_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(3),
        width: area.width,
        height: 1,
    };
    let sep = Paragraph::new("─".repeat(area.width as usize))
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black));
    frame.render_widget(sep, sep_area);
}

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let text = match app.mode {
        Mode::Search  => " Esc: cancel  ↑↓: navigate",
        Mode::Normal  => " ↑↓/jk: move  Space: preview  p: immersive  Enter: apply  f: fav  F: favs  </>: scroll  -/=: zoom  /: search  ?: help  q: quit",
        Mode::Confirm => " Enter: confirm  Esc: cancel",
        Mode::Help    => " ?: close",
        Mode::Immersive => "",
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
        Line::from(Span::styled(" posh-tui — keybindings", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled(" ↑/k  ↓/j   ", Style::default().fg(Color::Yellow)), Span::raw("navigate list")]),
        Line::from(vec![Span::styled(" g / G      ", Style::default().fg(Color::Yellow)), Span::raw("top / bottom")]),
        Line::from(vec![Span::styled(" PgUp/PgDn  ", Style::default().fg(Color::Yellow)), Span::raw("page scroll")]),
        Line::from(""),
        Line::from(vec![Span::styled(" Space      ", Style::default().fg(Color::Green)), Span::raw("preview theme in pane")]),
        Line::from(vec![Span::styled(" p          ", Style::default().fg(Color::Cyan)),  Span::raw("immersive full-screen preview")]),
        Line::from(vec![Span::styled(" Enter      ", Style::default().fg(Color::Green)), Span::raw("apply theme to shell")]),
        Line::from(vec![Span::styled(" u          ", Style::default().fg(Color::Red)),   Span::raw("undo last apply")]),
        Line::from(""),
        Line::from(vec![Span::styled(" < / >      ", Style::default().fg(Color::Cyan)),  Span::raw("scroll preview left / right")]),
        Line::from(vec![Span::styled(" - / =      ", Style::default().fg(Color::Cyan)),  Span::raw("zoom out / in")]),
        Line::from(vec![Span::styled(" 0          ", Style::default().fg(Color::Cyan)),  Span::raw("reset zoom")]),
        Line::from(""),
        Line::from(vec![Span::styled(" f          ", Style::default().fg(Color::Cyan)),  Span::raw("toggle favourite")]),
        Line::from(vec![Span::styled(" F          ", Style::default().fg(Color::Cyan)),  Span::raw("favourites view")]),
        Line::from(vec![Span::styled(" /          ", Style::default().fg(Color::Cyan)),  Span::raw("search themes")]),
        Line::from(vec![Span::styled(" r          ", Style::default().fg(Color::Cyan)),  Span::raw("refresh from GitHub")]),
        Line::from(""),
        Line::from(vec![Span::styled(" ?          ", Style::default().fg(Color::DarkGray)), Span::raw("toggle help")]),
        Line::from(vec![Span::styled(" q / Ctrl+C ", Style::default().fg(Color::DarkGray)), Span::raw("quit")]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" help "))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_confirm(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(45, 35, area);
    frame.render_widget(Clear, popup);

    let name = app.selected_theme()
        .map(|t| t.name.as_str())
        .unwrap_or("unknown");

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Apply theme: "),
            Span::styled(name, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  This will patch your shell config.", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("  A backup will be created first.", Style::default().fg(Color::DarkGray))),
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
            .block(Block::default().borders(Borders::ALL).title(" apply theme "))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub fn ansi_to_text(s: &str) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>>  = Vec::new();
    let mut style = Style::default();
    let mut buf   = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            if !buf.is_empty() {
                spans.push(Span::styled(buf.clone(), style));
                buf.clear();
            }
            chars.next();
            let mut seq = String::new();
            for ch in chars.by_ref() {
                seq.push(ch);
                if ch.is_ascii_alphabetic() { break; }
            }
            let params = &seq[..seq.len().saturating_sub(1)];
            style = apply_ansi_style(params, style);
        } else if c == '\n' {
            if !buf.is_empty() {
                spans.push(Span::styled(buf.clone(), style));
                buf.clear();
            }
            lines.push(Line::from(spans.clone()));
            spans.clear();
        } else if c == '\r' {
            // skip
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() { spans.push(Span::styled(buf, style)); }
    if !spans.is_empty() { lines.push(Line::from(spans)); }
    Text::from(lines)
}

fn apply_ansi_style(params: &str, current: Style) -> Style {
    let codes: Vec<u8> = params.split(';').filter_map(|s| s.parse().ok()).collect();
    let mut style = current;
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            0  => style = Style::default(),
            1  => style = style.add_modifier(Modifier::BOLD),
            2  => style = style.add_modifier(Modifier::DIM),
            3  => style = style.add_modifier(Modifier::ITALIC),
            4  => style = style.add_modifier(Modifier::UNDERLINED),
            30 => style = style.fg(Color::Black),
            31 => style = style.fg(Color::Red),
            32 => style = style.fg(Color::Green),
            33 => style = style.fg(Color::Yellow),
            34 => style = style.fg(Color::Blue),
            35 => style = style.fg(Color::Magenta),
            36 => style = style.fg(Color::Cyan),
            37 => style = style.fg(Color::White),
            40 => style = style.bg(Color::Black),
            41 => style = style.bg(Color::Red),
            42 => style = style.bg(Color::Green),
            43 => style = style.bg(Color::Yellow),
            44 => style = style.bg(Color::Blue),
            45 => style = style.bg(Color::Magenta),
            46 => style = style.bg(Color::Cyan),
            47 => style = style.bg(Color::White),
            90 => style = style.fg(Color::DarkGray),
            91 => style = style.fg(Color::LightRed),
            92 => style = style.fg(Color::LightGreen),
            93 => style = style.fg(Color::LightYellow),
            94 => style = style.fg(Color::LightBlue),
            95 => style = style.fg(Color::LightMagenta),
            96 => style = style.fg(Color::LightCyan),
            97 => style = style.fg(Color::White),
            38 => {
                if i + 1 < codes.len() {
                    match codes[i+1] {
                        5 if i+2 < codes.len() => { style = style.fg(Color::Indexed(codes[i+2])); i += 2; }
                        2 if i+4 < codes.len() => { style = style.fg(Color::Rgb(codes[i+2], codes[i+3], codes[i+4])); i += 4; }
                        _ => {}
                    }
                }
            }
            48 => {
                if i + 1 < codes.len() {
                    match codes[i+1] {
                        5 if i+2 < codes.len() => { style = style.bg(Color::Indexed(codes[i+2])); i += 2; }
                        2 if i+4 < codes.len() => { style = style.bg(Color::Rgb(codes[i+2], codes[i+3], codes[i+4])); i += 4; }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    style
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