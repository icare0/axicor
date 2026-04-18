use bevy::prelude::*;
use bevy::log::LogPlugin;
use bevy::winit::WinitSettings;

use axicor_lab::layout::WindowManagerPlugin;
use axicor_lab::plugins::AllPlugins;
use layout_api::OpenFileEvent;

fn main() {
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_writer(non_blocking_writer)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

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
        .add_event::<OpenFileEvent>()
        .insert_resource(WinitSettings::desktop_app())
        .run();
}
