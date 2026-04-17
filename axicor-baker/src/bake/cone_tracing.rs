use glam::Vec3;
use axicor_core::types::PackedPosition;
use crate::bake::spatial_grid::SpatialGrid;

pub struct ConeParams {
    pub radius_um: f32,
    pub fov_cos: f32,       // cos(FOV / 2.0). Если FOV = 60°, то cos(30°) ≈ 0.866
    pub owner_type: u8,     // [DOD] Сырой 4-битный тип владельца аксона
    pub type_affinity: f32, // [DOD] 0.0=тянется к чужим, 0.5=нейтрально, 1.0=к своим
}

/// Zero-Cost распаковка из 32 бит в f32 вектор (микрометры)
#[inline(always)]
pub fn unpack_to_vec3(pos: PackedPosition, voxel_size_um: f32) -> Vec3 {
    Vec3::new(
        (pos.x() as f32) * voxel_size_um,
        (pos.y() as f32) * voxel_size_um,
        (pos.z() as f32) * voxel_size_um,
    )
}

/// Сканирует пространство перед аксоном и вычисляет градиент притяжения (V_attract)
pub fn calculate_v_attract(
    origin_pos: PackedPosition,
    current_dir: Vec3,
    params: &ConeParams,
    grid: &SpatialGrid,
    voxel_size_um: f32,
) -> Vec3 {
    let origin_vec = unpack_to_vec3(origin_pos, voxel_size_um);

    // Переводим радиус поиска из мкм в чанки для SpatialGrid
    let radius_cells = (params.radius_um / (grid.cell_size as f32 * voxel_size_um)).ceil() as i32;

    let mut v_attract = Vec3::ZERO;

    // O(K) Zero-allocation spatial query
    grid.for_each_in_radius(&origin_pos, radius_cells, |dense_id| {
        let neighbor_pos = grid.get_position(dense_id);

        // Игнорируем себя (коллизия координат)
        if neighbor_pos.0 == origin_pos.0 { return; }

        let target_vec = unpack_to_vec3(neighbor_pos, voxel_size_um);
        let diff = target_vec - origin_vec;
        let dist_sq = diff.length_squared();

        // Быстрое отсечение по сфере (Squared — никаких sqrt!)
        if dist_sq > params.radius_um * params.radius_um || dist_sq == 0.0 {
            return;
        }

        let dist = dist_sq.sqrt();
        let dir_to_target = diff / dist;

        // Отсечение по Конусу (Cone Frustum Culling)
        let dot = current_dir.dot(dir_to_target);
        if dot > params.fov_cos {
            // [DOD] Branchless Type Affinity Math
            // is_same = 1.0 если типы совпадают, 0.0 если различаются
            let is_same = (neighbor_pos.type_id() == params.owner_type) as i32 as f32;

            // При is_same=1.0 → берём affinity
            // При is_same=0.0 → берём (1.0 - affinity)
            // ×2.0: при affinity=0.5 нейтральный множитель = 1.0 для всех
            let affinity_mod = (is_same * params.type_affinity
                + (1.0 - is_same) * (1.0 - params.type_affinity)) * 2.0;

            let weight = (1.0 / (dist_sq + 1.0)) * affinity_mod;
            v_attract += dir_to_target * weight;
        }
    });

    // Если в конусе пусто, вектор нулевой. Иначе возвращаем нормализованную тягу.
    if v_attract.length_squared() > 0.0 {
        v_attract.normalize()
    } else {
        Vec3::ZERO
    }
}
