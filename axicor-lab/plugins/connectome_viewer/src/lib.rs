use bevy::prelude::*;
use bevy::render::{render_asset::RenderAssetUsages, render_resource::PrimitiveTopology, view::RenderLayers};
use layout_api::{PluginWindow, PluginDomain, PluginInput, ZoneSelectedEvent, PluginGeometry};

#[derive(Component)]
pub struct ShardGeometry {
    pub viewport: Entity,
}

#[derive(Component)]
pub struct ViewportCamera {
    pub target: Vec3,
    pub radius: f32,
    pub alpha: f32, // Rotation around Y
    pub beta: f32,  // Rotation up/down
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 40.0, // Дистанция, охватывающая весь шард
            alpha: std::f32::consts::PI / 4.0, // Изометрия по умолчанию
            beta: 0.5,
        }
    }
}

pub struct ConnectomeViewerPlugin;

impl Plugin for ConnectomeViewerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            viewport_camera_control_system,
            load_zone_geometry_system,
        ));
    }
}

pub fn load_zone_geometry_system(
    mut commands: Commands,
    mut events: EventReader<ZoneSelectedEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viewports: Query<(Entity, &PluginWindow)>, 
    existing_geometries: Query<(Entity, &ShardGeometry)>, // DOD: Пул текущей геометрии
) {
    for ev in events.read() {
        let path = format!("Genesis-Models/{}/baked/{}/shard.pos", ev.project_name, ev.shard_name);
        
        let Ok(data) = std::fs::read(&path) else {
            eprintln!("FATAL: Missing shard geometry at {}", path);
            continue;
        };

        // Мгновенный каст без парсинга (Zero-Copy Read)
        let packed_positions: &[u32] = bytemuck::cast_slice(&data);

        // Фаза 1: Динамический AABB
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

        // Фаза 2: Сборка буферов
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

        // Трансляция геометрии
        for (vp_entity, plugin) in viewports.iter() {
            if plugin.domain != PluginDomain::Viewport3D { continue; }
            
            // DOD FIX: Garbage Collection. Жестко выжигаем старую геометрию этого окна.
            for (geom_entity, geom) in existing_geometries.iter() {
                if geom.viewport == vp_entity {
                    commands.entity(geom_entity).despawn_recursive();
                }
            }

            let layer_id = (vp_entity.index() % 32) as u8;
            commands.entity(vp_entity).insert(RenderLayers::layer(layer_id));

            // Спавним новое облако и вешаем маркер принадлежности
            commands.spawn((
                PbrBundle {
                    mesh: mesh_handle.clone(),
                    material: mat_handle.clone(),
                    ..Default::default()
                },
                RenderLayers::layer(layer_id),
                ShardGeometry { viewport: vp_entity }, // Реляционная связь
            ));
        }
    }
}

pub fn viewport_camera_control_system(
    // DOD: Запрашиваем геометрию и проекцию для синхронизации линзы
    mut query: Query<(&mut Transform, &mut ViewportCamera, &PluginInput, &PluginGeometry, &mut Projection)>,
) {
    for (mut transform, mut cam, input, geom, mut projection) in query.iter_mut() {
        
        // DOD FIX 1: Жесткая синхронизация Aspect Ratio с RTT-текстурой
        if geom.size.x > 0.0 && geom.size.y > 0.0 {
            if let Projection::Perspective(ref mut persp) = *projection {
                let current_aspect = geom.size.x / geom.size.y;
                if (persp.aspect_ratio - current_aspect).abs() > 0.001 {
                    persp.aspect_ratio = current_aspect;
                }
            }
        }

        // DOD FIX 2: Квантование сырой дельты скролла (защита от скачков)
        if input.scroll_delta.abs() > 0.0 {
            let tick = input.scroll_delta.signum(); // Строго +1.0 или -1.0
            cam.radius -= tick * 0.15 * cam.radius; // Логарифмический шаг 15%
            cam.radius = cam.radius.clamp(0.1, 1000.0);
        }

        // Вращение (ПКМ)
        if input.is_secondary_pressed {
            cam.alpha -= input.cursor_delta.x * 0.005;
            cam.beta -= input.cursor_delta.y * 0.005;
            cam.beta = cam.beta.clamp(-std::f32::consts::PI / 2.0 + 0.01, std::f32::consts::PI / 2.0 - 0.01);
        }

        // Панорамирование (СКМ или Shift + ПКМ)
        if input.is_middle_pressed {
            let right = transform.right();
            let up = transform.up();
            let pan_speed = cam.radius * 0.002;
            cam.target -= right * input.cursor_delta.x * pan_speed;
            cam.target += up * input.cursor_delta.y * pan_speed;
        }

        // Пересчет декартовых координат из сферических
        let rotation = Quat::from_euler(EulerRot::YXZ, cam.alpha, cam.beta, 0.0);
        let offset = rotation * Vec3::new(0.0, 0.0, cam.radius);
        
        transform.translation = cam.target + offset;
        transform.look_at(cam.target, Vec3::Y);
    }
}
