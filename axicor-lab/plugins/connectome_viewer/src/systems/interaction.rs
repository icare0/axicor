use crate::domain::{NeuronInstances, ShardGeometry, ViewportCamera};
use bevy::prelude::*;
use layout_api::PluginInput;

pub fn soma_picking_system(
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut instances: ResMut<NeuronInstances>,
    mut gizmos: Gizmos,
    viewports: Query<(&PluginInput, &Camera, &GlobalTransform), With<ViewportCamera>>,
) {
    // DOD FIX:   .
    //  CPU  O(N)      .
    if !mouse_btn.just_pressed(MouseButton::Left) {
        return;
    }

    if instances.data.is_empty() {
        return;
    }

    for (input, camera, cam_transform) in viewports.iter() {
        if let Some(ray) = camera.viewport_to_world(cam_transform, input.local_cursor) {
            let ray_origin = ray.origin;
            let ray_dir: Vec3 = ray.direction.into();

            let mut best_dist = f32::MAX;
            let mut best_idx = None;

            for (i, neuron) in instances.data.iter().enumerate() {
                let center = Vec3::from(neuron.position);
                let radius = neuron.scale;
                let v = center - ray_origin;
                let t = v.dot(ray_dir);
                if t < 0.0 {
                    continue;
                }
                let dist_sq = (v - ray_dir * t).length_squared();
                let hitbox_radius = (t * 0.01).max(radius);
                if dist_sq < hitbox_radius * hitbox_radius {
                    if t < best_dist {
                        best_dist = t;
                        best_idx = Some(i);
                    }
                }
            }

            if best_idx.is_some() {
                instances.selected = best_idx;
            } else {
                instances.selected = None;
            }
        }

        // Selection Visualization (Gizmos)
        if let Some(idx) = instances.selected {
            if let Some(neuron) = instances.data.get(idx) {
                gizmos.sphere(
                    Vec3::from(neuron.position),
                    Quat::IDENTITY,
                    neuron.scale * 1.5,
                    Color::WHITE,
                );
            }
        }
    }
}

use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::view::{NoFrustumCulling, RenderLayers};

pub fn trace_active_connections_system(
    mut commands: Commands,
    mut graph: ResMut<crate::domain::TopologyGraph>,
    instances: Res<crate::domain::NeuronInstances>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut soma_materials: ResMut<Assets<crate::systems::material::NeuronInstanceMaterial>>,
    viewports: Query<(Entity, &layout_api::PluginWindow)>,
) {
    //
    if graph.last_selected == instances.selected {
        return;
    }
    graph.last_selected = instances.selected;

    // 0. DOD FIX: X-Ray Mode (Focus)
    if let Some(global_mat) = materials.get_mut(&graph.global_axon_mat) {
        if instances.selected.is_some() {
            global_mat.base_color.set_a(0.05);
        } else {
            global_mat.base_color.set_a(1.0);
        }
    }

    if let Some(ent) = graph.traced_entity {
        commands.entity(ent).despawn_recursive();
        graph.traced_entity = None;
    }

    let Some(selected_compact) = instances.selected else {
        // :    ,    100%
        if let Some(soma_mat) = soma_materials.get_mut(&graph.soma_mat) {
            for inst in soma_mat.instances.iter_mut() {
                inst.color[3] = 1.0;
            }
        }
        return;
    };

    if selected_compact >= graph.compact_to_dense.len() {
        return;
    }
    let selected_soma = graph.compact_to_dense[selected_compact];
    if selected_soma >= graph.padded_n {
        return;
    }

    // DOD FIX:   Dense ID ,
    let mut visible_dense_somas = std::collections::HashSet::new();
    visible_dense_somas.insert(selected_soma);

    let mut lines_pos = Vec::new();
    let mut lines_col = Vec::new();

    let afferent_col = [0.0, 0.8, 1.0, 0.8]; //   (Cyan)
    let efferent_col = [1.0, 0.2, 0.8, 0.8]; //   (Magenta)
    let target_axon_col = [0.0, 1.0, 0.5, 0.6]; //  -
    let my_axon_col = [1.0, 0.5, 0.0, 0.8]; //
    let mut drawn_axons = std::collections::HashSet::new();

    let my_pos = graph.soma_positions[selected_soma];

    // 1.  (   )
    for slot in 0..128 {
        let idx = slot * graph.padded_n + selected_soma;
        if idx >= graph.targets.len() {
            break;
        }

        let target_packed = graph.targets[idx];
        if target_packed == 0 {
            break;
        }

        let axon_id_plus_1 = target_packed & axicor_core::constants::TARGET_AXON_MASK;
        let seg_idx = (target_packed >> axicor_core::constants::TARGET_SEG_SHIFT) as usize;

        if axon_id_plus_1 > 0 {
            let axon_id = (axon_id_plus_1 - 1) as usize;
            if axon_id < graph.axon_segments.len() && seg_idx < graph.axon_segments[axon_id].len() {
                let target_pos = graph.axon_segments[axon_id][seg_idx];
                lines_pos.push(my_pos);
                lines_pos.push(target_pos);
                lines_col.push(afferent_col);
                lines_col.push(afferent_col);

                //
                if axon_id < graph.axon_to_soma.len() {
                    let src_soma = graph.axon_to_soma[axon_id];
                    if src_soma != usize::MAX {
                        visible_dense_somas.insert(src_soma);
                    }
                }

                if drawn_axons.insert(axon_id) {
                    let pts = &graph.axon_segments[axon_id];
                    for i in 0..pts.len().saturating_sub(1) {
                        lines_pos.push(pts[i]);
                        lines_pos.push(pts[i + 1]);
                        lines_col.push(target_axon_col);
                        lines_col.push(target_axon_col);
                    }
                }
            }
        }
    }

    // 2.  (    )
    let my_axon = graph.soma_to_axon[selected_soma];
    if my_axon != 0xFFFFFFFF && (my_axon as usize) < graph.axon_segments.len() {
        let my_axon_idx = my_axon as usize;

        if drawn_axons.insert(my_axon_idx) {
            let pts = &graph.axon_segments[my_axon_idx];
            for i in 0..pts.len().saturating_sub(1) {
                lines_pos.push(pts[i]);
                lines_pos.push(pts[i + 1]);
                lines_col.push(my_axon_col);
                lines_col.push(my_axon_col);
            }
        }

        for other_soma in 0..graph.padded_n {
            if other_soma >= graph.soma_positions.len() {
                break;
            }
            let other_pos = graph.soma_positions[other_soma];
            if other_pos == Vec3::ZERO {
                continue;
            }

            for slot in 0..128 {
                let idx = slot * graph.padded_n + other_soma;
                if idx >= graph.targets.len() {
                    break;
                }

                let target_packed = graph.targets[idx];
                if target_packed == 0 {
                    break;
                }

                let axon_id_plus_1 = target_packed & axicor_core::constants::TARGET_AXON_MASK;
                if axon_id_plus_1 > 0 && (axon_id_plus_1 - 1) as usize == my_axon_idx {
                    let seg_idx =
                        (target_packed >> axicor_core::constants::TARGET_SEG_SHIFT) as usize;
                    if seg_idx < graph.axon_segments[my_axon_idx].len() {
                        let target_pos = graph.axon_segments[my_axon_idx][seg_idx];
                        lines_pos.push(other_pos);
                        lines_pos.push(target_pos);
                        lines_col.push(efferent_col);
                        lines_col.push(efferent_col);

                        //   ,
                        visible_dense_somas.insert(other_soma);
                    }
                }
            }
        }
    }

    // 3. DOD FIX: X-Ray Mode   (  )
    if let Some(soma_mat) = soma_materials.get_mut(&graph.soma_mat) {
        for (i, inst) in soma_mat.instances.iter_mut().enumerate() {
            let dense_id = graph.compact_to_dense.get(i).copied().unwrap_or(usize::MAX);
            if visible_dense_somas.contains(&dense_id) {
                inst.color[3] = 1.0; //    -
            } else {
                inst.color[3] = 0.05; //   -
            }
        }
    }

    // 4.  ...
    if lines_pos.is_empty() {
        return;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::LineList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, lines_pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, lines_col);

    let mat = materials.add(StandardMaterial {
        unlit: true,
        base_color: Color::WHITE,
        alpha_mode: bevy::pbr::AlphaMode::Blend,
        ..Default::default()
    });
    let mesh_handle = meshes.add(mesh);

    for (vp_entity, plugin) in viewports.iter() {
        if plugin.plugin_id != "axicor.viewport_3d" {
            continue;
        }
        let layer_id = (vp_entity.index() % 32) as u8;

        let id = commands
            .spawn((
                bevy::pbr::PbrBundle {
                    mesh: mesh_handle.clone(),
                    material: mat.clone(),
                    ..Default::default()
                },
                ShardGeometry {
                    viewport: vp_entity,
                },
                RenderLayers::layer(layer_id),
                NoFrustumCulling,
            ))
            .id();
        graph.traced_entity = Some(id);
        break;
    }
}
