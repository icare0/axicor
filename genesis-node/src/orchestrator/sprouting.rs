// genesis-node/src/orchestrator/sprouting.rs
use genesis_core::constants::MAX_DENDRITE_SLOTS;
use genesis_core::layout::pack_dendrite_target;
use genesis_core::config::blueprints::NeuronType;

// FNV-1a для детерминированного псевдорандома (Stateless Hashing)
fn hash_seed(seed: u64, salt: u32) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for &b in &seed.to_le_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    for &b in &salt.to_le_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// Выполняет поиск новых связей для пустых слотов и рост аксонов (Nudging).
///
/// [DOD FIX] Добавлен параметр `neuron_types` для соблюдения Закона Дейла.
/// Тормозные нейроны (is_inhibitory=true) обязаны получать отрицательный вес.
pub fn run_cpu_sprouting(
    targets: &mut [u32],
    weights: &mut [i16],
    padded_n: usize,
    total_axons: u32,
    master_seed: u64,
    axon_tips_uvw: &mut [u32],
    axon_dirs_xyz: &[u32],
    voxel_size_um: f32,
    world_size_um: (f32, f32, f32),
    axon_growth_step_um: f32, // TODO Чистка архитектурного мусора
    host_neuron_positions: &[u32],
    spatial_grid: &std::sync::Arc<std::sync::Mutex<crate::orchestrator::spatial_grid::SpatialGrid>>,
    neuron_types: &[NeuronType], // [DOD] Закон Дейла: знак веса зависит от типа источника
) -> (usize, Vec<crate::network::slow_path::AxonHandoverEvent>) {
    let mut new_synapses = 0;
    let mut handovers = Vec::new();

    // 1. Axon Nudging & Boundary Checks
    for axon_id in 0..(total_axons as usize) {
        let packed_tip = axon_tips_uvw[axon_id];
        if packed_tip == 0 { continue; }

        let mut tx = packed_tip & 0x3FF;
        let mut ty = (packed_tip >> 10) & 0x3FF;
        let mut tz = (packed_tip >> 20) & 0xFF;

        let packed_dir = axon_dirs_xyz[axon_id];
        let dx = (packed_dir & 0xFF) as i8 as f32 / 127.0;
        let dy = ((packed_dir >> 8) & 0xFF) as i8 as f32 / 127.0;
        let dz = ((packed_dir >> 16) & 0xFF) as i8 as f32 / 127.0;

        let ux = tx as f32 * voxel_size_um + dx * axon_growth_step_um;
        let uy = ty as f32 * voxel_size_um + dy * axon_growth_step_um;
        let uz = tz as f32 * voxel_size_um + dz * axon_growth_step_um;

        if ux < 0.0 || ux >= world_size_um.0 || uy < 0.0 || uy >= world_size_um.1 || uz < 0.0 || uz >= world_size_um.2 {
             handovers.push(crate::network::slow_path::AxonHandoverEvent {
                local_axon_id: axon_id as u32,
                entry_x: (ux / voxel_size_um).clamp(0.0, 1023.0) as u16,
                entry_y: (uy / voxel_size_um).clamp(0.0, 1023.0) as u16,
                vector_x: (dx * 127.0) as i8,
                vector_y: (dy * 127.0) as i8,
                vector_z: (dz * 127.0) as i8,
                type_mask: (packed_tip >> 28) as u8,
                remaining_length: 100,
                _padding: 0,
            });
        } else {
            tx = (ux / voxel_size_um) as u32;
            ty = (uy / voxel_size_um) as u32;
            tz = (uz / voxel_size_um) as u32;
            axon_tips_uvw[axon_id] = (packed_tip & 0xF0000000) | (tz << 20) | (ty << 10) | tx;
        }
    }

    // 2. Synaptic Sprouting
    let grid = spatial_grid.lock().unwrap();
    for i in 0..padded_n {
        let nrn_pos = host_neuron_positions[i];
        if nrn_pos == 0 { continue; }

        // Unpack XYZ (Z|Y|X) — совпадает с baker coordinate packing
        let nx = nrn_pos & 0x3FF;
        let ny = (nrn_pos >> 10) & 0x3FF;
        let nz = (nrn_pos >> 20) & 0xFF;

        for slot in (0..MAX_DENDRITE_SLOTS).rev() {
            let col_idx = slot * padded_n + i;
            if targets[col_idx] != 0 { break; } // Слот занят — идём к более сильным

            let salt = (i as u32).wrapping_add(slot as u32);
            let seed = hash_seed(master_seed, salt) as u64;

            if let Some(candidate_axon) = grid.get_random_candidate(nx, ny, nz, seed) {
                // [DOD FIX 1] Правильная упаковка (Zero-Index Trap Prevention)
                // pack_dendrite_target добавляет +1 к axon_id — гарантия что 0 = пустой слот
                // Подключаемся к голове аксона (segment_offset = 0)
                let new_target = pack_dendrite_target(candidate_axon, 0);

                // [DOD FIX 2] Закон Дейла — знак веса из конфига Blueprint
                // Тормозные нейроны (Basket, Chandelier) дают ОТРИЦАТЕЛЬНЫЙ вес.
                // Возбуждающие (Pyramidal, Stellate) дают ПОЛОЖИТЕЛЬНЫЙ вес.
                let type_id = (axon_tips_uvw[candidate_axon as usize] >> 28) as usize;
                let final_weight = if let Some(owner_type) = neuron_types.get(type_id) {
                    let abs_weight = owner_type.initial_synapse_weight as i16;
                    if owner_type.is_inhibitory { -abs_weight } else { abs_weight }
                } else {
                    // Неизвестный тип — безопасный дефолт, не создаём OOB
                    74_i16
                };

                targets[col_idx] = new_target;
                weights[col_idx] = final_weight;
                new_synapses += 1;
                break;
            }
        }
    }

    (new_synapses, handovers)
}
