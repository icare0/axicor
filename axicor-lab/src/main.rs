use bevy::prelude::*;
use bevy::log::LogPlugin;

pub mod layout;

#[path = "../plugins/mod.rs"]
pub mod plugins;

use crate::layout::WindowManagerPlugin;
use crate::plugins::AllPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Axicor Lab".into(),
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                decorations: false,
                ..default()
            }),
            ..default()
        }).set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,bevy_render::view::window=error".into(),
            level: bevy::log::Level::INFO,
            ..default()
        }))
        .add_plugins(WindowManagerPlugin)
        .add_plugins(AllPlugins)
        .run();
}
