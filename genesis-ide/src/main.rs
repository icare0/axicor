mod camera;
mod hud;
mod loader;
mod telemetry;
mod world;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Genesis IDE".into(),
            resolution: (1600.0, 900.0).into(),
            ..default()
        }),
        ..default()
    }))
    .init_state::<AppState>()
    .init_resource::<camera::CameraSpeed>()
    .init_resource::<camera::CameraMode>()
    .init_resource::<hud::SelectedNeuron>()
    .init_resource::<loader::WorldData>()
    .add_plugins((
        loader::LoaderPlugin,
        world::WorldPlugin,
        camera::CameraPlugin,
        hud::HudPlugin,
        telemetry::TelemetryPlugin,
    ));

    // Setup telemetry channel resources (mpsc pair).
    telemetry::setup_telemetry_resources(&mut app);

    app.run();
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Running,
}
