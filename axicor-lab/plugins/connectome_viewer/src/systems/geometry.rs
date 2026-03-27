use bevy::prelude::*;
use bevy::render::{render_asset::RenderAssetUsages, render_resource::PrimitiveTopology, view::RenderLayers};
use layout_api::PluginWindow;
use crate::domain::{ShardGeometry, ZoneSelectedEvent};

pub fn load_zone_geometry_system(
    mut commands: Commands,
    mut events: EventReader<ZoneSelectedEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viewports: Query<(Entity, &PluginWindow)>, 
    existing_geometries: Query<(Entity, &ShardGeometry)>,
    bundle: Res<layout_api::ActiveBundle>,
) {
    for ev in events.read() {
        if bundle.project_name != ev.project_name {
            eprintln!("Load project '{}' in Node Editor first!", ev.project_name);
            continue;
        }

        let internal_path = format!("baked/{}/shard.pos", ev.shard_name);

        let Some(data) = bundle.get_file(&internal_path) else {
            eprintln!("FATAL: Missing shard geometry at {} in archive", internal_path);
            continue;
        };

        let packed_positions: &[u32] = bytemuck::cast_slice(data);

        let mut min_bounds = Vec3::splat(f32::MAX);
        let mut max_bounds = Vec3::splat(f32::MIN);
        let mut valid_count = 0;

        for &packed in packed_positions {
            if packed == 0 { continue; }
            let x = (packed & 0x3FF) as f32 * 0.025;
            let y = ((packed >> 10) & 0x3FF) as f32 * 0.025;
            let z = ((packed >> 20) & 0xFF) as f32 * 0.025;

            let pos = Vec3::new(x, z, -y);
            min_bounds = min_bounds.min(pos);
            max_bounds = max_bounds.max(pos);
            valid_count += 1;
        }

        if valid_count == 0 { continue; }

        let center = (min_bounds + max_bounds) * 0.5;

        let mut positions = Vec::with_capacity(valid_count);
        let mut colors = Vec::with_capacity(valid_count);

        for &packed in packed_positions {
            if packed == 0 { continue; }
            let x = (packed & 0x3FF) as f32 * 0.025;
            let y = ((packed >> 10) & 0x3FF) as f32 * 0.025;
            let z = ((packed >> 20) & 0xFF) as f32 * 0.025;
            let type_id = (packed >> 28) & 0xF;

            let pos = Vec3::new(x, z, -y) - center;
            positions.push([pos.x, pos.y, pos.z]);

            let color = if type_id % 2 == 0 {
                [0.2, 0.8, 0.9, 1.0]
            } else {
                [0.9, 0.2, 0.2, 1.0]
            };
            colors.push(color);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::PointList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        let mesh_handle = meshes.add(mesh);
        let mat_handle = materials.add(StandardMaterial {
            unlit: true,
            ..Default::default()
        });

        for (vp_entity, plugin) in viewports.iter() {
            if plugin.plugin_id != "axicor.viewport_3d" { continue; } // DOD FIX: Сравниваем строки
            
            for (geom_entity, geom) in existing_geometries.iter() {
                if geom.viewport == vp_entity {
                    commands.entity(geom_entity).despawn_recursive();
                }
            }

            let layer_id = (vp_entity.index() % 32) as u8;
            commands.entity(vp_entity).insert(RenderLayers::layer(layer_id));

            commands.spawn((
                PbrBundle {
                    mesh: mesh_handle.clone(),
                    material: mat_handle.clone(),
                    ..Default::default()
                },
                RenderLayers::layer(layer_id),
                ShardGeometry { viewport: vp_entity },
            ));
        }
    }
}
