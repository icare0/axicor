use bevy::prelude::*;
use bevy::render::{render_asset::RenderAssetUsages, render_resource::PrimitiveTopology};
use axicor_core::layout::{PathsFileHeader, PATHS_MAGIC, calculate_paths_matrix_offset, MAX_SEGMENTS_PER_AXON};
use axicor_core::types::PackedPosition;

pub struct AxonBuildResult {
    pub mesh: Mesh,
    pub axon_segments_lookup: Vec<Vec<Vec3>>,
}

pub fn build_axon_lines(
    paths_bytes: &[u8],
    pos_data: &[u8],
    center: Vec3,
    state_bytes: Option<&[u8]>,
) -> Option<AxonBuildResult> {
    let header = unsafe { &*(paths_bytes.as_ptr() as *const PathsFileHeader) };
    if header.magic != PATHS_MAGIC { return None; }

    let total_axons = header.total_axons as usize;
    let lengths_slice = &paths_bytes[std::mem::size_of::<PathsFileHeader>() .. std::mem::size_of::<PathsFileHeader>() + total_axons];
    let matrix_offset = calculate_paths_matrix_offset(total_axons);
    
    // DOD FIX: Безопасное чтение невыровненной матрицы путей
    let matrix_vec: Vec<u32> = paths_bytes[matrix_offset..].chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let matrix_slice = &matrix_vec;

    let mut line_positions = Vec::new();
    let mut line_colors = Vec::new();

    let mut axon_segments_lookup = vec![Vec::new(); total_axons];

    for axon_id in 0..total_axons {
        let len = lengths_slice[axon_id] as usize;
        if len == 0 { continue; } // Пропускаем только полностью пустые слоты

        let offset = axon_id * MAX_SEGMENTS_PER_AXON;
        if offset + len > matrix_slice.len() { continue; }
        let axon_path = &matrix_slice[offset .. offset + len];

        // 1. Сохраняем ВСЕ валидные точки в лукап (Гарантирует 1:1 маппинг с seg_idx)
        for i in 0..len {
            let p = PackedPosition(axon_path[i]);
            if p.0 == 0 { continue; }

            let v = Vec3::new(
                (p.x() as f32 * 0.025) - center.x,
                (p.z() as f32 * 0.025) - center.y,
                -(p.y() as f32 * 0.025) - center.z,
            );
            axon_segments_lookup[axon_id].push(v);
        }

        // 2. Строим 3D-линии сегментов (только если точек >= 2)
        let points = &axon_segments_lookup[axon_id];
        if points.len() >= 2 {
            // Извлекаем Type ID из первой точки для Закона Дейла
            let type_id = (axon_path[0] >> 28) & 0xF;
            let axon_color = if type_id % 2 != 0 {
                [0.9, 0.2, 0.2, 0.15]  // Inh: Красный
            } else {
                [1.0, 0.53, 0.0, 0.15] // Exc: Оранжевый
            };

            for i in 0..(points.len() - 1) {
                line_positions.push(points[i]);
                line_positions.push(points[i + 1]);
                line_colors.push(axon_color);
                line_colors.push(axon_color);
            }
        }
    }

    // DOD FIX: ДОБАВЛЯЕМ КОРНЕВЫЕ СЕГМЕНТЫ (От сомы к первому излому)
    if let Some(state_data) = state_bytes {
        // DOD FIX: shard.state is a headerless raw dump. 
        // 1166 bytes per neuron invariant (14 bytes soma + 1152 bytes dendrites)
        let padded_n = state_data.len() / 1166;
        
        // Offset for soma_to_axon: voltage(4) + flags(1) + thresh(4) + timers(1) = 10 bytes * N
        let soma_to_axon_offset = padded_n * 10;
        let targets_offset = padded_n * 14;

        let s2a_vec: Vec<u32> = state_data[soma_to_axon_offset .. targets_offset]
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();

        // Парсим сырые позиции для прямого маппинга по Dense ID
        let packed_positions: Vec<u32> = pos_data.chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();

        for dense_id in 0..padded_n {
            if dense_id >= packed_positions.len() { break; }
            let packed = packed_positions[dense_id];
            if packed == 0 { continue; } // Пропускаем пустышки

            let local_axon_id = s2a_vec[dense_id] as usize;

            // Проверяем валидность аксона (отсекаем пустышки 0xFFFFFFFF)
            if local_axon_id < total_axons && !axon_segments_lookup[local_axon_id].is_empty() {
                // Вычисляем позицию сомы на лету из Dense ID
                let x = (packed & 0x3FF) as f32 * 0.025;
                let y = ((packed >> 10) & 0x3FF) as f32 * 0.025;
                let z = ((packed >> 20) & 0xFF) as f32 * 0.025;
                let soma_pos = Vec3::new(x, z, -y) - center;

                let first_segment_pos = axon_segments_lookup[local_axon_id][0];

                // Восстанавливаем Закон Дейла из упакованного типа
                let type_id = (packed >> 28) & 0xF;
                let root_color = if type_id % 2 != 0 {
                    [0.9, 0.2, 0.2, 0.15]  // Inh: Красный
                } else {
                    [1.0, 0.53, 0.0, 0.15] // Exc: Оранжевый
                };

                line_positions.push(soma_pos);
                line_positions.push(first_segment_pos);
                line_colors.push(root_color);
                line_colors.push(root_color);
            }
        }
    }

    if line_positions.is_empty() { return None; }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, line_positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, line_colors);

    Some(AxonBuildResult {
        mesh,
        axon_segments_lookup,
    })
}
