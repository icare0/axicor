use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Component)]
pub struct CodeEditorState {
    pub current_file: Option<PathBuf>,
    pub content: String,
}

impl Default for CodeEditorState {
    fn default() -> Self {
        Self {
            current_file: None,
            content: "/* Select a file in Project Explorer */\n".to_string(),
        }
    }
}
