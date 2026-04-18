use crate::domain::CodeEditorState;
use bevy::prelude::*;
use layout_api::{base_domain, OpenFileEvent, PluginWindow, DOMAIN_CODE_EDITOR};

pub fn handle_open_file_system(
    mut events: EventReader<OpenFileEvent>,
    mut editors: Query<&mut CodeEditorState>,
) {
    for ev in events.read() {
        //
        if let Some(mut state) = editors.iter_mut().next() {
            if let Ok(content) = layout_api::overlay_read_to_string(&ev.path) {
                state.current_file = Some(ev.path.clone());
                state.content = content.clone();
                state.saved_content = content;
            }
        }
    }
}

pub fn init_editor_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Added<PluginWindow>>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_CODE_EDITOR {
            commands
                .entity(entity)
                .insert(crate::domain::CodeEditorState::default());
        }
    }
}

pub fn evict_deleted_files_system(
    mut events: EventReader<layout_api::EntityDeletedEvent>,
    mut editors: Query<&mut crate::domain::CodeEditorState>,
) {
    for ev in events.read() {
        for mut state in editors.iter_mut() {
            if let Some(curr) = &state.current_file {
                if curr.starts_with(&ev.path) {
                    state.current_file = None;
                    let deleted_msg = "/* File has been deleted */\n".to_string();
                    state.content = deleted_msg.clone();
                    state.saved_content = deleted_msg;
                }
            }
        }
    }
}
