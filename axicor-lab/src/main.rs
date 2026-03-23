mod layout;
mod viewport;
mod widgets;
mod panels;
mod theme;

use bevy::prelude::*;
use crate::layout::systems::setup_layout;
use crate::layout::interaction::{border_interaction_system, border_drag_system, DragState};
use crate::viewport::{setup_3d_scene, sync_camera_viewport};
use crate::widgets::slider::{slider_drag_system, ActiveSlider};
use crate::widgets::button::button_system;

fn main() {
    App::new()
        .insert_resource(DragState::default())
        .insert_resource(ActiveSlider::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Axicor Lab".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_layout, setup_3d_scene))
        .add_systems(Update, (
            border_interaction_system, 
            border_drag_system,
            slider_drag_system,
            button_system,
        ))
        .add_systems(PostUpdate, sync_camera_viewport)
        .run();
}
