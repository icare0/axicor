mod layout;

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy_egui::EguiPlugin;
use project_explorer::ProjectExplorerPlugin;
use connectome_viewer::ConnectomeViewerPlugin;
use node_editor::NodeEditorPlugin;
use crate::layout::data::*;
use crate::layout::systems::{create_plugin_render_target, render_workspace_system, evaluate_drag_intents_system, execute_window_commands_system, window_garbage_collector_system, window_drag_execution_system};
use crate::layout::input::sync_plugin_geometry_system;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Axicor Lab".into(),
                decorations: false,
                transparent: true, // DOD FIX: Блокируем отрисовку мусорного фона ОС при старте
                window_theme: Some(bevy::window::WindowTheme::Dark),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .add_plugins((
            ProjectExplorerPlugin,
            ConnectomeViewerPlugin,
            NodeEditorPlugin,
        ))
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .insert_resource(Msaa::Off)
        .init_resource::<layout_api::WindowDragRequest>()
        .init_resource::<ProjectFsCache>()
        .init_resource::<WorkspaceTree>()
        .init_resource::<WindowDragState>()
        .init_resource::<TopologyCache>()
        .init_resource::<TreeCommands>()
        .add_event::<ZoneSelectedEvent>()
        .add_systems(Startup, setup_test_bench)
        .add_systems(Update, (
            sync_plugin_geometry_system,
            render_workspace_system,
            evaluate_drag_intents_system,
            execute_window_commands_system,
            window_garbage_collector_system,
            window_drag_execution_system,
        ).chain())
        .run();
}

fn setup_test_bench(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    // --- Viewport3D ---
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
        connectome_viewer::ViewportCamera::default(),
        PluginWindow { domain: PluginDomain::Viewport3D, texture: Some(tex_a) },
        PluginInput::default(),
        PluginGeometry { size: Vec2::new(800.0, 600.0) },
    )).id();

    // --- Project Explorer ---
    let entity_b = commands.spawn((
        PluginWindow { domain: PluginDomain::ProjectExplorer, texture: None },
        PluginInput::default(),
        PluginGeometry { size: Vec2::new(400.0, 600.0) },
    )).id();

    // --- Node Editor ---
    let entity_c = commands.spawn((
        PluginWindow { domain: PluginDomain::NodeEditor, texture: None },
        PluginInput::default(),
        PluginGeometry { size: Vec2::new(400.0, 600.0) },
    )).id();

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
    let pane3 = tree.tiles.insert_pane(entity_c);
    
    // Horizontal layout: [Viewport3D] | [Vertical: Explorer over NodeEditor]
    let right_col = tree.tiles.insert_vertical_tile(vec![pane2, pane3]);
    let root = tree.tiles.insert_horizontal_tile(vec![pane1, right_col]);
    tree.root = Some(root);
    
    commands.insert_resource(WorkspaceTree { tree });
}
