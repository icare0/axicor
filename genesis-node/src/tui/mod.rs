pub mod state;
pub mod layout;
pub mod input;
pub mod widgets;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use state::DashboardState;

pub fn run_tui(state: Arc<Mutex<DashboardState>>, log_mode: bool) -> Result<()> {
    if log_mode {
        run_log_reporter(state);
        return Ok(());
    }

    // [TUI] Redirect stdout/stderr to /dev/null to suppress println! from all threads.
    // Boot-time prints (before this point) still go to terminal normally.
    // After TUI exits, we restore normally via LeaveAlternateScreen (crossterm handles this).
    unsafe {
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_WRONLY);
        if devnull >= 0 {
            libc::dup2(devnull, libc::STDOUT_FILENO);
            libc::dup2(devnull, libc::STDERR_FILENO);
            libc::close(devnull);
        }
    }

    // Initialize TUI — writes go to the alternate screen via crossterm's own fd
    enable_raw_mode()?;
    // Re-open /dev/tty for the TUI backend (crossterm needs a real terminal)
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    execute!(&tty, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(tty);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        // Can't eprintln here since stderr is /dev/null, but that's fine
        let _ = err;
    }
    
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, state: Arc<Mutex<DashboardState>>) -> Result<()> {
    let tick_rate = Duration::from_millis(200);

    // Open /dev/tty for keyboard events since stdin might be redirected
    let tty_fd = std::fs::File::open("/dev/tty")?;
    // crossterm reads from /dev/tty automatically on Linux when stdin is not a tty
    drop(tty_fd);

    loop {
        terminal.draw(|f| {
            let mut s = state.lock().unwrap();
            layout::draw(f, &mut s);
        })?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                let mut s = state.lock().unwrap();
                if input::handle_key(key, &mut s) {
                    break;
                }
            }
        }
    }
    
    Ok(())
}

fn run_log_reporter(state: Arc<Mutex<DashboardState>>) {
    let mut last_batch = 0;
    
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let s = state.lock().unwrap();
        
        if s.batch_number == last_batch && s.is_running {
            continue; // Wait for updates
        }
        
        last_batch = s.batch_number;
        let wall_ms = s.wall_ms_history.back().copied().unwrap_or(0);
        let tps = s.ticks_per_sec;
        let ticks = s.total_ticks;
        
        let now = chrono::Local::now().format("%H:%M:%S");
        eprintln!("[{}] [BATCH] #{} | {} ticks | {}ms wall | {:.2} t/s", now, s.batch_number, ticks, wall_ms, tps);
        
        if !s.zones.is_empty() {
            eprint!("[{}] [ZONE]  ", now);
            for (i, z) in s.zones.iter().enumerate() {
                eprint!("{}: {} spikes ({:.2}%)", z.short_name, z.spikes_last_batch, z.spike_rate);
                if i < s.zones.len() - 1 {
                    eprint!(" | ");
                }
            }
            eprintln!();
        }
        
        eprintln!("[{}] [IO]    UDP IN: {} | OUT: {} | Oversized: {}", now, s.udp_in_packets, s.udp_out_packets, s.oversized_skips);
    }
}
