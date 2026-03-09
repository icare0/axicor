use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::state::DashboardState;

pub fn handle_key(key: KeyEvent, state: &mut DashboardState) -> bool {
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc | KeyCode::F(6) => {
            state.is_running = false;
        }
        KeyCode::Char('s') | KeyCode::F(5) => {
            state.is_running = true;
        }
        KeyCode::Up => {
            // Scroll log up
            if state.log_scroll > 0 {
                state.log_scroll -= 1;
            }
        }
        KeyCode::Down => {
            // Scroll log down
            let max_scroll = state.events.len().saturating_sub(5);
            if state.log_scroll < max_scroll {
                state.log_scroll += 1;
            }
        }
        _ => {}
    }
    false
}
