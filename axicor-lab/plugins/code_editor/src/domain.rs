use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Component)]
pub struct CodeEditorState {
    pub current_file: Option<PathBuf>,
    pub content: String,
    pub saved_content: String,
}

impl Default for CodeEditorState {
    fn default() -> Self {
        let initial_text = "/* Select a file in Project Explorer */\n".to_string();
        Self {
            current_file: None,
            content: initial_text.clone(),
            saved_content: initial_text,
        }
    }
}
