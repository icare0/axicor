use bevy::prelude::*;
use crate::domain::CodeEditorState;
use layout_api::{OpenFileEvent, PluginWindow, base_domain, DOMAIN_CODE_EDITOR}; 

pub fn handle_open_file_system(
    mut events: EventReader<OpenFileEvent>,
    mut editors: Query<&mut CodeEditorState>,
) {
    for ev in events.read() {
        // Загружаем файл в первый найденный редактор кода
        if let Some(mut state) = editors.iter_mut().next() {
            if let Ok(content) = std::fs::read_to_string(&ev.path) {
                state.current_file = Some(ev.path.clone());
                state.content = content;
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
            commands.entity(entity).insert(crate::domain::CodeEditorState::default());
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
                    state.content = "/* File has been deleted */\n".to_string();
                }
            }
        }
    }
}
