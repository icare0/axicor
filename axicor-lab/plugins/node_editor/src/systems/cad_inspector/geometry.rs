use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use crate::domain::{NodeGraphUiState, EditorLevel, ShardCadEntity, BrainTopologyGraph};

#[derive(Component)]
pub struct CadGeometryMarker;

pub fn spawn_cad_geometry_system(
    mut commands: Commands,
    query: Query<&NodeGraphUiState>,
    geometries: Query<Entity, With<CadGeometryMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    graph: Res<BrainTopologyGraph>,
) {
    if !geometries.is_empty() { return; }

    let Some(state) = query.iter().find(|s| matches!(s.level, EditorLevel::Zone(_))) else { return };
    if state.shard_rtt.is_none() { return; }

    let mut w = 32.0; let mut d = 32.0; let mut h = 32.0;
    let mut layers = vec![crate::domain::ShardLayer { name: "Main".to_string(), height_pct: 1.0 }];

    if let EditorLevel::Zone(ref shard_name) = state.level {
        if let Some(active_path) = &graph.active_path {
            if let Some(session) = graph.sessions.get(active_path) {
                if let Some(anatomy) = session.shard_anatomies.get(shard_name) {
                    w = anatomy.w.clamp(1.0, 10000.0);
                    d = anatomy.d.clamp(1.0, 10000.0);
                    h = anatomy.h.clamp(1.0, 10000.0);
                    if !anatomy.layers.is_empty() { layers = anatomy.layers.clone(); }
                }
            }
        }
    }

    let mut current_y = -h / 2.0;

    for layer in layers.iter() {
        let layer_h = (h * layer.height_pct).max(0.1);
        let center_y = current_y + layer_h / 2.0;

        // Строим 1 прямоугольный куб на 1 слой
        let mesh = meshes.add(Cuboid::from_size(Vec3::new(w, layer_h * 0.95, d)));
        
        let hash = genesis_core::hash::fnv1a_32(layer.name.as_bytes());
        let base_luma = 0.15 + (hash % 100) as f32 / 100.0 * 0.4;
        let r_shift = ((hash >> 8) % 60) as f32 / 1000.0 - 0.03;
        let g_shift = ((hash >> 16) % 60) as f32 / 1000.0 - 0.03;
        let b_shift = ((hash >> 24) % 60) as f32 / 1000.0 - 0.03;
        
        let r = (base_luma + r_shift).clamp(0.0, 1.0);
        let g = (base_luma + g_shift).clamp(0.0, 1.0);
        let b = (base_luma + b_shift).clamp(0.0, 1.0);

        let material = materials.add(StandardMaterial {
            base_color: bevy::prelude::Color::rgba(r, g, b, 0.05),
            alpha_mode: bevy::prelude::AlphaMode::Blend,
            ..default()
        });

        commands.spawn((
            PbrBundle {
                mesh, 
                material,
                transform: Transform::from_xyz(0.0, center_y, 0.0),
                ..default()
            },
            RenderLayers::layer(2),
            ShardCadEntity,
            CadGeometryMarker,
        ));
        
        current_y += layer_h;
    }
    
    info!("[Geometry] Simple Layer Cubes spawned ({}x{}x{})", w, h, d);
}
