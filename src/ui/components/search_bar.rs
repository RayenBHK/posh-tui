use crate::app::Mode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub(crate) fn draw_search(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let is_fav = app
        .selected_theme()
        .map(|t| app.theme_state.favourites.contains(&t.name))
        .unwrap_or(false);

    let fav_indicator = if is_fav { " ★" } else { "" };

    let applied_label = match &app.theme_state.last_applied {
        Some(name) => format!(" applied: {name} "),
        None => String::new(),
    };

    let _title = if app.theme_state.show_favs {
        format!(" ★ favourites{fav_indicator} ")
    } else {
        format!(" posh-tui{fav_indicator}  {applied_label}")
    };

    let query = if app.mode == Mode::Search {
        format!("/ {}_", app.search_state.search_query)
    } else if app.search_state.search_query.is_empty() {
        " type / to search...".to_string()
    } else {
        format!("/ {}", app.search_state.search_query)
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
    } else if app.theme_state.show_favs {
        format!(" ★ favourites{fav_indicator} ")
    } else if app.theme_state.show_recent {
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
