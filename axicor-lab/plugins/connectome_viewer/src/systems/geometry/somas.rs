use bevy::prelude::*;
use bevy::render::{render_asset::RenderAssetUsages, render_resource::PrimitiveTopology};
use crate::domain::{NeuronInstanceData, ATTRIBUTE_SPHERE_ID};
use serde::Deserialize;

#[derive(Deserialize)]
struct BlueprintType {
    threshold: i32,
}

#[derive(Deserialize)]
struct Blueprints {
    neuron_type: Vec<BlueprintType>,
}

pub struct SomaBuildResult {
    pub mesh: Mesh,
    pub instances: Vec<NeuronInstanceData>,
    pub center: Vec3,
}

pub fn build_soma_instances(
    shard_pos_data: &[u8],
    blueprints_data: Option<&[u8]>,
) -> Option<SomaBuildResult> {
    // 1. Загружаем blueprints для получения thresholds
    let mut thresholds = vec![32000; 16]; // Fallback
    if let Some(bp_data) = blueprints_data {
        if let Ok(bp_str) = std::str::from_utf8(bp_data) {
            if let Ok(bp) = toml::from_str::<Blueprints>(bp_str) {
                for (i, t) in bp.neuron_type.iter().enumerate().take(16) {
                    thresholds[i] = t.threshold;
                }
            }
        }
    }

    // DOD FIX: Безопасное чтение невыровненной памяти через Vec<u32>
    let packed_positions_vec: Vec<u32> = shard_pos_data.chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let packed_positions = &packed_positions_vec;

    let mut min_bounds = Vec3::splat(f32::MAX);
    let mut max_bounds = Vec3::splat(f32::MIN);
    let mut valid_count = 0;
    
    // Первый проход для центрирования
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

    if valid_count == 0 { return None; }
    let center = (min_bounds + max_bounds) * 0.5;

    // 2. Сборка данных инстансов
    let mut instances = Vec::with_capacity(valid_count);
    for &packed in packed_positions {
        if packed == 0 { continue; }
        let x = (packed & 0x3FF) as f32 * 0.025;
        let y = ((packed >> 10) & 0x3FF) as f32 * 0.025;
        let z = ((packed >> 20) & 0xFF) as f32 * 0.025;
        let type_id = ((packed >> 28) & 0xF) as usize;

        let pos = Vec3::new(x, z, -y) - center;
        let threshold = thresholds[type_id];
        
        // DOD FIX: Физически точный расчет радиуса сомы.
        // Опорная точка: Пирамидальный нейрон L5 (Threshold 42000) имеет радиус 10 мкм.
        // Емкость мембраны пропорциональна площади поверхности сферы (R^2),
        // поэтому радиус R пропорционален sqrt(threshold).
        let base_threshold = 42000.0;
        let base_radius_um = 10.0;

        let radius_um = base_radius_um * (threshold as f32 / base_threshold).sqrt();

        // Защита от нулевых или экстремально малых порогов
        let clamped_radius_um = radius_um.clamp(3.0, 12.5);

        // Перевод микрометров в систему координат Bevy (где 1.0 единица = 1 мм = 1000 мкм)
        let scale = clamped_radius_um / 1000.0;

        let color = if type_id % 2 == 0 {
            Vec4::new(0.2, 0.8, 0.9, 1.0) // Exc
        } else {
            Vec4::new(0.9, 0.2, 0.2, 1.0) // Inh
        };

        instances.push(NeuronInstanceData {
            position: [pos.x, pos.y, pos.z],
            scale,
            color: color.into(),
        });
    }

    // 3. Создаем базовый меш Icosphere(2)
    #[allow(deprecated)]
    let base_sphere_mesh: Mesh = bevy::render::mesh::shape::Icosphere {
        radius: 1.0,
        subdivisions: 2,
    }.try_into().unwrap();

    let base_positions = base_sphere_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    let base_indices = match base_sphere_mesh.indices().unwrap() {
        bevy::render::mesh::Indices::U32(vec) => vec.clone(),
        bevy::render::mesh::Indices::U16(vec) => vec.iter().map(|&i| i as u32).collect(),
    };

    let vertices_per_sphere = base_positions.len() as u32;
    let indices_per_sphere = base_indices.len();

    let mut giant_positions = Vec::with_capacity(valid_count * vertices_per_sphere as usize);
    let mut giant_indices = Vec::with_capacity(valid_count * indices_per_sphere);
    let mut giant_sphere_ids = Vec::with_capacity(valid_count * vertices_per_sphere as usize);

    for i in 0..valid_count as u32 {
        giant_positions.extend_from_slice(base_positions);
        for &idx in &base_indices {
            giant_indices.push(idx + i * vertices_per_sphere);
        }
        for _ in 0..vertices_per_sphere {
            giant_sphere_ids.push(i);
        }
    }

    let mut instanced_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    instanced_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, giant_positions);
    instanced_mesh.insert_attribute(ATTRIBUTE_SPHERE_ID, giant_sphere_ids);
    instanced_mesh.insert_indices(bevy::render::mesh::Indices::U32(giant_indices));

    Some(SomaBuildResult {
        mesh: instanced_mesh,
        instances,
        center,
    })
}
