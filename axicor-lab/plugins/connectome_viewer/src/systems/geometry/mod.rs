pub mod somas;
pub mod axons;
pub mod dendrites;

use bevy::prelude::*;
use bevy::render::view::{RenderLayers, NoFrustumCulling};
use bevy::pbr::{MaterialMeshBundle, PbrBundle};
use layout_api::{PluginWindow, PluginInput};
use crate::domain::{ShardGeometry, ZoneSelectedEvent, NeuronInstances};
use crate::systems::material::NeuronInstanceMaterial;

pub fn load_zone_geometry_system(
    mut commands: Commands,
    mut events: EventReader<ZoneSelectedEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<NeuronInstanceMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    viewports: Query<(Entity, &PluginInput, &PluginWindow)>, 
    existing_geometries: Query<(Entity, &ShardGeometry)>,
    mut neuron_instances: ResMut<NeuronInstances>,
) {
    for ev in events.read() {
        // DOD FIX:     Axicor-Models (    )
        let axic_path = std::path::PathBuf::from("Axicor-Models")
            .join(format!("{}.axic", ev.project_name));

        let Some(archive) = axicor_core::vfs::AxicArchive::open(&axic_path) else {
            eprintln!("Failed to open VFS archive: {:?}", axic_path);
            continue;
        };

        // 1. SOMAS
        let blueprints_data = archive.get_file(&format!("{}/blueprints.toml", ev.shard_name));
        let pos_path = format!("baked/{}/shard.pos", ev.shard_name);
        
        let Some(pos_data) = archive.get_file(&pos_path) else {
            eprintln!("FATAL: Missing shard geometry at {} in archive", pos_path);
            continue;
        };

        let Some(soma_result) = somas::build_soma_instances(pos_data, blueprints_data) else {
            continue;
        };
        
        neuron_instances.data = soma_result.instances.clone();
        neuron_instances.selected = None;
        let soma_mesh_handle = meshes.add(soma_result.mesh);
        let soma_mat_handle = materials.add(NeuronInstanceMaterial {
            instances: soma_result.instances.clone(),
        });

        //  state_bytes     
        let state_path = format!("baked/{}/shard.state", ev.shard_name);
        let state_bytes = archive.get_file(&state_path);

        // 2. AXONS
        let paths_path = format!("baked/{}/shard.paths", ev.shard_name);
        let mut axon_mesh_handle = None;
        let mut axon_mat_handle = None;
        let mut segment_lookup = Vec::new();

        if let Some(paths_bytes) = archive.get_file(&paths_path) {
            // DOD FIX:  pos_data  center ,    instances
            if let Some(axon_result) = axons::build_axon_lines(paths_bytes, pos_data, soma_result.center, state_bytes.as_deref()) {
                axon_mesh_handle = Some(meshes.add(axon_result.mesh));
                axon_mat_handle = Some(standard_materials.add(StandardMaterial {
                    unlit: true,
                    base_color: Color::WHITE,
                    alpha_mode: bevy::pbr::AlphaMode::Blend,
                    ..Default::default()
                }));
                segment_lookup = axon_result.axon_segments_lookup;
            }
        }

        // 3. CACHE TOPOLOGY FOR TRACING
        let mut topology_graph = crate::domain::TopologyGraph::default();
        if let Some(state_data) = state_bytes {
            if !segment_lookup.is_empty() {
                crate::systems::geometry::dendrites::build_topology_graph(
                    state_data, pos_data, soma_result.center, &segment_lookup, &mut topology_graph
                );
            }
        }

        // DOD FIX:  Handle   O(1)  
        if let Some(mat) = &axon_mat_handle {
            topology_graph.global_axon_mat = mat.clone();
        }
        // :   
        topology_graph.soma_mat = soma_mat_handle.clone();

        commands.insert_resource(topology_graph);

        // 4. SPAWN (Orchestration)
        for (vp_entity, _input, plugin) in viewports.iter() {
            if plugin.plugin_id != "axicor.viewport_3d" { continue; }
            
            // Clean up old geometry
            for (geom_entity, geom) in existing_geometries.iter() {
                if geom.viewport == vp_entity {
                    commands.entity(geom_entity).despawn_recursive();
                }
            }

            let layer_id = (vp_entity.index() % 32) as u8;
            let layer = RenderLayers::layer(layer_id);
            commands.entity(vp_entity).insert(layer.clone());

            // Spawn Somas
            commands.spawn((
                MaterialMeshBundle {
                    mesh: soma_mesh_handle.clone(),
                    material: soma_mat_handle.clone(),
                    ..Default::default()
                },
                ShardGeometry { viewport: vp_entity },
                layer.clone(),
                NoFrustumCulling, 
            ));

            // Spawn Axons
            if let (Some(mh), Some(mat)) = (axon_mesh_handle.clone(), axon_mat_handle.clone()) {
                commands.spawn((
                    PbrBundle {
                        mesh: mh,
                        material: mat,
                        ..Default::default()
                    },
                    ShardGeometry { viewport: vp_entity },
                    layer.clone(),
                    NoFrustumCulling,
                ));
            }
        }
    }
}
