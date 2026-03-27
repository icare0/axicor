use bevy::{
    prelude::*,
    window::PrimaryWindow,
    winit::WinitWindows,
};
use crate::layout::domain::OsWindowCommand;

pub fn window_drag_execution_system(
    mut events: EventReader<OsWindowCommand>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let Ok(primary_entity) = window_query.get_single() else { return };
    let Some(winit_window) = winit_windows.get_window(primary_entity) else { return };

    for cmd in events.read() {
        match cmd {
            OsWindowCommand::Drag => { let _ = winit_window.drag_window(); }
            OsWindowCommand::Minimize => winit_window.set_minimized(true),
            OsWindowCommand::Maximize => winit_window.set_maximized(!winit_window.is_maximized()),
        }
    }
}