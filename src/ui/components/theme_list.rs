use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub(crate) fn draw_list(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    // show spinner while loading
    if app.loading || app.refreshing {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = (app.immersive_state.imm_cursor_tick as usize / 3) % spinner_frames.len();
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
            let star = if app.theme_state.favourites.contains(&t.name) {
                "★"
            } else {
                " "
            };
            let applied = match &app.theme_state.last_applied {
                Some(name) if name == &t.name => "✓ ",
                _ => "  ",
            };
            ListItem::new(format!("{star}{applied}{}", t.name))
        })
        .collect();

    let title = if app.theme_state.show_favs {
        format!(" favs ({total}) ")
    } else if app.theme_state.show_recent {
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
    state.select(if total > 0 { Some(app.theme_state.selected) } else { None });
    frame.render_stateful_widget(list, area, &mut state);
}
