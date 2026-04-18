use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use crate::domain::{AnatomySlicerState, ShardCadEntity};
use node_editor::domain::BrainTopologyGraph;
use crate::cad_glass_material::CadGlassMaterial;

#[derive(Component)]
pub struct CadGeometryMarker;

#[derive(Component)]
pub struct CadHoverPlane;

pub fn spawn_cad_geometry_system(
    mut commands: Commands,
    query: Query<&AnatomySlicerState>,
    geometries: Query<Entity, With<CadGeometryMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glass_materials: ResMut<Assets<CadGlassMaterial>>,
    graph: Res<BrainTopologyGraph>,
) {
    if !geometries.is_empty() { return; }

    let Some(state) = query.iter().find(|s| s.active_zone.is_some()) else { return };
    if state.shard_rtt.is_none() { return; }

    let mut w = 32.0; let mut d = 32.0; let mut h = 32.0;
    let mut layers = vec![node_editor::domain::ShardLayer { name: "Main".to_string(), height_pct: 1.0 }];

    if let Some(ref shard_name) = state.active_zone {
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

    // 0. Transparent Hover-plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::from_size(Vec3::new(w * 1.1, 0.1, d * 1.1))),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(1.0, 1.0, 1.0, 0.0), // Fully transparent
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::Hidden,
            ..default()
        },
        RenderLayers::layer(2),
        ShardCadEntity,
        CadHoverPlane,
    ));

    let mut current_y = -h / 2.0;

    for layer in layers.iter() {
        let layer_h = (h * layer.height_pct).max(0.1);
        let center_y = current_y + layer_h / 2.0;

        let mesh = meshes.add(Cuboid::from_size(Vec3::new(w, layer_h, d)));
        
        let hash = axicor_core::hash::fnv1a_32(layer.name.as_bytes());
        // Deterministic color generation based on layer name (Luma 0.4 - 0.7)
        let base_luma = 0.4 + (hash % 100) as f32 / 100.0 * 0.3;
        let r_shift = (((hash >> 8) % 100) as f32 / 100.0) * 0.4 - 0.2;
        let g_shift = (((hash >> 16) % 100) as f32 / 100.0) * 0.4 - 0.2;
        let b_shift = (((hash >> 24) % 100) as f32 / 100.0) * 0.4 - 0.2;

        let r = (base_luma + r_shift).clamp(0.2f32, 0.9f32);
        let g = (base_luma + g_shift).clamp(0.2f32, 0.9f32);
        let b = (base_luma + b_shift).clamp(0.2f32, 0.9f32);

        // [DOD FIX] Render as semi-transparent with Fresnel-like glass material
        commands.spawn((
            MaterialMeshBundle {
                mesh: mesh.clone(),
                material: glass_materials.add(CadGlassMaterial {
                    color: Color::rgba(r, g, b, 0.15), // [DOD FIX] Alpha
                }),
                transform: Transform::from_xyz(0.0, center_y, 0.0),
                ..default()
            },
            RenderLayers::layer(2),
            ShardCadEntity,
            CadGeometryMarker,
        ));
        
        current_y += layer_h;
    }

    // [AESTHETICS] 3. Render I/O Port Planes (EnvRx and Inter-zone)
    if let Some(ref shard_name) = state.active_zone {
        if let Some(active_path) = graph.active_path.as_ref() {
            let project_dir = active_path.parent().unwrap_or(std::path::Path::new("."));
            let path_str = active_path.to_string_lossy();
            let is_sim = path_str.contains("simulation.toml");
            let dept_name = active_path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");

            let base_dir = if is_sim { project_dir.join(shard_name) } 
                           else { project_dir.join(&dept_name).join(shard_name) };

            // 3.1. External Inputs from io.toml (EnvRx)
            if let Ok(content) = layout_api::overlay_read_to_string(&base_dir.join("io.toml")) {
                if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                    if let Some(inputs) = doc.get("input").and_then(|i| i.as_array_of_tables()) {
                        for table in inputs.iter() {
                            if let Some(z) = table.get("entry_z").and_then(|v| v.as_integer()) {
                                let pos_y = -h / 2.0 + (z as f32) + 0.5;
                                commands.spawn((
                                    PbrBundle {
                                        mesh: meshes.add(Cuboid::from_size(Vec3::new(w * 1.05, 0.15, d * 1.05))),
                                        material: materials.add(StandardMaterial {
                                            base_color: Color::rgba(0.2, 0.8, 0.2, 0.3), // Green for external input
                                            alpha_mode: AlphaMode::Blend,
                                            unlit: true,
                                            ..default()
                                        }),
                                        transform: Transform::from_xyz(0.0, pos_y, 0.0),
                                        ..default()
                                    },
                                    RenderLayers::layer(2), ShardCadEntity, CadGeometryMarker,
                                ));
                            }
                        }
                    }
                }
            }

            // 3.2. Ghost Axon planes from brain.toml (Inter-zone)
            if let Ok(content) = layout_api::overlay_read_to_string(active_path) {
                if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                    if let Some(conns) = doc.get("connection").and_then(|i| i.as_array_of_tables()) {
                        for table in conns.iter() {
                            if table.get("to").and_then(|v| v.as_str()) == Some(shard_name) {
                                if let Some(z) = table.get("entry_z").and_then(|v| v.as_integer()) {
                                    let pos_y = -h / 2.0 + (z as f32) + 0.5;
                                    commands.spawn((
                                        PbrBundle {
                                            mesh: meshes.add(Cuboid::from_size(Vec3::new(w * 1.05, 0.15, d * 1.05))),
                                            material: materials.add(StandardMaterial {
                                                base_color: Color::rgba(1.0, 0.6, 0.0, 0.3), // Orange for inter-zone connections
                                                alpha_mode: AlphaMode::Blend,
                                                unlit: true,
                                                ..default()
                                            }),
                                            transform: Transform::from_xyz(0.0, pos_y, 0.0),
                                            ..default()
                                        },
                                        RenderLayers::layer(2), ShardCadEntity, CadGeometryMarker,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    info!("[Geometry] Simple Layer Cubes and Hover Plane spawned ({}x{}x{})", w, h, d);
}

pub fn sync_hover_plane_system(
    query: Query<&AnatomySlicerState>,
    mut plane_query: Query<(&mut Transform, &mut Visibility, &Handle<StandardMaterial>), With<CadHoverPlane>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    graph: Res<BrainTopologyGraph>,
) {
    let Some(state) = query.iter().find(|s| s.active_zone.is_some()) else { return };
    let Ok((mut transform, mut vis, mat_handle)) = plane_query.get_single_mut() else { return };

    if let Some((_pos, z_voxel)) = state.active_3d_hover {
        let mut h = 32.0;
        let mut _is_env = false; // DOD: Future use, check for dragging_pin in node_editor

        if let Some(ref shard_name) = state.active_zone {
            if let Some(active_path) = &graph.active_path {
                if let Some(session) = graph.sessions.get(active_path) {
                    if let Some(anatomy) = session.shard_anatomies.get(shard_name) {
                        h = anatomy.h;
                    }
                }
            }
        }

        let center_y = -h / 2.0 + z_voxel as f32 + 0.5;
        transform.translation.y = center_y;
        *vis = Visibility::Visible;
        
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.base_color = Color::rgba(1.0, 0.6, 0.0, 0.5); // Highlighter color
        }
    } else {
        *vis = Visibility::Hidden;
    }
}

pub fn refresh_cad_geometry_on_change_system(
    mut topo_changed: EventReader<layout_api::TopologyChangedEvent>,
    mut topo_mut: EventReader<node_editor::domain::TopologyMutation>,
    mut commands: Commands,
    geometries: Query<Entity, With<CadGeometryMarker>>,
    planes: Query<Entity, With<CadHoverPlane>>,
) {
    let mut should_refresh = false;
    for _ in topo_changed.read() { should_refresh = true; }
    for _ in topo_mut.read() { should_refresh = true; }

    if should_refresh {
        for ent in geometries.iter() { commands.entity(ent).despawn_recursive(); }
        for ent in planes.iter() { commands.entity(ent).despawn_recursive(); }
        info!("[Geometry] Triggered CAD cubes refresh");
    }
}
