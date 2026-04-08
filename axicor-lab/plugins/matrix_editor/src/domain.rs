use bevy::prelude::*;

#[derive(Component, Default)]
pub struct MatrixEditorState {
    pub active_zone: Option<String>,
}
