use bevy::prelude::*;

#[derive(Component, Default)]
pub struct IoInspectorState {
    pub active_zone: Option<String>,
}
