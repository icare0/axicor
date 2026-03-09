use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Cell, Table, BorderType},
    Frame,
};
use crate::tui::state::{DashboardState, Phase};

pub fn draw(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PER-ZONE NEURAL TELEMETRY ")
        .border_type(BorderType::Plain);

    let rows: Vec<Row> = state.zones.iter().map(|z| {
        // Phase + time-to-night
        let ticks_in_phase = state.total_ticks % z.night_interval_ticks.max(1);
        let ticks_to_night = z.night_interval_ticks.saturating_sub(ticks_in_phase);
        let secs_to_night = if state.ticks_per_sec > 0.0 {
            (ticks_to_night as f64 / state.ticks_per_sec) as u64
        } else {
            0
        };

        let phase_str = match z.phase {
            Phase::Day => {
                let m = secs_to_night / 60;
                let s = secs_to_night % 60;
                format!("DAY {:02}:{:02}", m, s)
            }
            Phase::Night => "NIGHT".to_string(),
            Phase::Sleep => "SLEEP".to_string(),
        };

        // Spike color
        let rate = z.spike_rate;
        let color = if rate < 1.0 {
            Color::Cyan
        } else if rate < 5.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        let rate_str = format!("{:.2}%", rate);

        Row::new(vec![
            Cell::from(z.short_name.clone()),
            Cell::from(format!("{}", z.neuron_count)),
            Cell::from(format!("{}", z.axon_count)),
            Cell::from(rate_str).style(Style::default().fg(color)),
            Cell::from(phase_str),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(11), // wider for "DAY MM:SS"
        ]
    )
    .header(
        Row::new(vec!["ZONE ID", "NEUR", "AXON", "SPIKE", "PHASE"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1)
    )
    .block(block)
    .column_spacing(1);

    f.render_widget(table, area);
}
