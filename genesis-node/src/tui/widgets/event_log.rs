use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, BorderType},
    Frame,
};
use crate::tui::state::{DashboardState, LogLevel};

pub fn draw(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" NIGHT PHASE EVENTS LOG ")
        .border_type(BorderType::Plain);

    // We want to display them oldest to newest from top to bottom (if showing bottom of list)
    // Actually, ratatui List takes items top-to-bottom. If we want auto-scroll, we take the last 5.
    let display_items: Vec<ListItem> = state.events.iter()
        .skip(state.events.len().saturating_sub(5 + state.log_scroll))
        .take(5)
        .map(|e| {
            let color = match e.level {
                LogLevel::Info => Color::White,
                LogLevel::Warning => Color::Yellow,
                LogLevel::Night => Color::Cyan,
                LogLevel::Error => Color::Red,
            };
            let content = format!("[{}] {}", e.timestamp, e.message);
            ListItem::new(content).style(Style::default().fg(color))
        })
        .collect();

    let list = List::new(display_items).block(block);
    f.render_widget(list, area);
}
