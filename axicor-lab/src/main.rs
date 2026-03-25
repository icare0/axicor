mod layout;

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy_egui::EguiPlugin;
use crate::layout::data::*;
use crate::layout::systems::*;
use crate::layout::input::sync_plugin_geometry_system;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Genesis IDE - Modular Layout".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .insert_resource(Msaa::Off)
        .init_resource::<WorkspaceTree>()
        .init_resource::<WindowDragState>()
        .init_resource::<TopologyCache>()
        .init_resource::<TreeCommands>()
        .add_systems(Startup, setup_test_bench)
        .add_systems(Update, (
            sync_plugin_geometry_system,
            render_workspace_system,
            evaluate_drag_intents_system,
            execute_window_commands_system,
            window_garbage_collector_system,
        ).chain())
        .run();
}

fn setup_test_bench(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // --- Plugin A: Red Cube ---
    let tex_a = create_plugin_render_target(&mut images, 800, 600);
    let entity_a = commands.spawn((
        Camera3dBundle {
            camera: Camera {
                target: RenderTarget::Image(tex_a.clone()),
                clear_color: ClearColorConfig::Custom(Color::rgb(0.1, 0.1, 0.15)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PluginWindow { texture: tex_a },
        PluginInput::default(),
        PluginGeometry { size: Vec2::new(800.0, 600.0) },
    )).id();

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial::from(Color::RED)),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // --- Plugin B: Blue Sphere ---
    let tex_b = create_plugin_render_target(&mut images, 800, 600);
    let entity_b = commands.spawn((
        Camera3dBundle {
            camera: Camera {
                target: RenderTarget::Image(tex_b.clone()),
                clear_color: ClearColorConfig::Custom(Color::rgb(0.15, 0.1, 0.1)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PluginWindow { texture: tex_b },
        PluginInput::default(),
        PluginGeometry { size: Vec2::new(800.0, 600.0) },
    )).id();

    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(0.7)),
        material: materials.add(StandardMaterial::from(Color::BLUE)),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // --- Global Light ---
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // --- Tiling Setup ---
    let mut tree = egui_tiles::Tree::empty("workspace");
    let pane1 = tree.tiles.insert_pane(entity_a);
    let pane2 = tree.tiles.insert_pane(entity_b);
    let root = tree.tiles.insert_horizontal_tile(vec![pane1, pane2]);
    tree.root = Some(root);
    
    commands.insert_resource(WorkspaceTree { tree });
}
