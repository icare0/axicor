use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use crate::tui::state::DashboardState;
use crate::tui::widgets::{
    global_state, core_loop, zone_table, io_panel, button_bar, event_log
};

pub fn draw(f: &mut Frame, state: &mut DashboardState) {
    let area = f.area();

    // Check if terminal is too narrow
    if area.width < 80 {
        draw_narrow_warning(f, area);
        return;
    }

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Global System State
            Constraint::Min(10),    // Middle panels
            Constraint::Length(1),  // Button Bar
            Constraint::Length(6),  // Event Log
        ])
        .split(area);

    global_state::draw(f, chunks[0], state);
    
    // Middle section distribution
    if area.width >= 120 {
        // Full width layout
        let middle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Core Loop
                Constraint::Percentage(40), // Per-Zone
                Constraint::Percentage(30), // Hardware & I/O
            ])
            .split(chunks[1]);

        core_loop::draw(f, middle_chunks[0], state);
        zone_table::draw(f, middle_chunks[1], state);
        io_panel::draw(f, middle_chunks[2], state);
    } else {
        // 80-119 cols layout
        let vertical_middle = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Core + Zone
                Constraint::Percentage(40), // I/O below
            ])
            .split(chunks[1]);

        let horizontal_top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Core Loop
                Constraint::Percentage(60), // Per-Zone
            ])
            .split(vertical_middle[0]);

        core_loop::draw(f, horizontal_top[0], state);
        zone_table::draw(f, horizontal_top[1], state);
        io_panel::draw(f, vertical_middle[1], state);
    }

    button_bar::draw(f, chunks[2], state);
    event_log::draw(f, chunks[3], state);
}

fn draw_narrow_warning(f: &mut Frame, area: Rect) {
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::style::{Style, Color, Modifier};

    let msg = "Terminal too narrow for optimal display.\nPlease resize to at least 80 columns.";
    let warning = Paragraph::new(msg)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("WARNING"));
    
    f.render_widget(warning, area);
}
