use genesis_core::config::{SimulationConfig, BlueprintsConfig, IoConfig};
use crate::bake::axon_growth::LayerZRange;
use genesis_core::coords::unpack_position;
use crate::bake::neuron_placement::PlacedNeuron;
use crate::bake::axon_growth::{GrownAxon, GhostPacket, grow_single_axon, ShardBounds};
use crate::bake::spatial_grid::SpatialGrid;

const GXI_MAGIC: u32 = 0x47584930; // "GXI0"
const GXI_VERSION: u16 = 1;

/// Хэш по алгоритму FNV-1a
fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub struct VirtualAxonsResult {
    pub axons: Vec<GrownAxon>,
    pub ghosts: Vec<GhostPacket>,
    pub gxi_binary: Vec<u8>,
}

/// Выращивает виртуальные аксоны для входных данных и генерирует GXI-файл.
pub fn grow_input_maps(
    io_config: &IoConfig,
    zone_name: &str,
    placed_neurons: &[PlacedNeuron],
    layer_ranges: &[LayerZRange],
    blueprints: &BlueprintsConfig,
    sim: &SimulationConfig,
    shard_bounds: &ShardBounds,
    master_seed: u64,
    base_axon_id: u32,
) -> VirtualAxonsResult {
    let mut virtual_axons = Vec::new();
    let mut ghost_packets = Vec::new();

    // Отфильтруем входы, предназначенные для этой зоны
    let inputs: Vec<_> = io_config.inputs.iter()
        .filter(|i| i.target_zone == zone_name)
        .collect();

    if inputs.is_empty() {
        return VirtualAxonsResult {
            axons: vec![],
            ghosts: vec![],
            gxi_binary: vec![],
        };
    }

    let world_w_vox = sim.world.width_um / sim.simulation.voxel_size_um;
    let world_d_vox = sim.world.depth_um / sim.simulation.voxel_size_um;
    let world_h_vox = sim.world.height_um / sim.simulation.voxel_size_um;
    let spatial_grid = SpatialGrid::new(placed_neurons);

    // Сформируем бинарник GXI сразу по ходу генерации
    let mut gxi_bytes = Vec::new();
    // Header Placeholder
    gxi_bytes.extend_from_slice(&GXI_MAGIC.to_le_bytes());
    gxi_bytes.extend_from_slice(&GXI_VERSION.to_le_bytes());
    gxi_bytes.extend_from_slice(&(inputs.len() as u16).to_le_bytes());

    let mut map_descriptors = Vec::new();
    let mut flat_axon_ids = Vec::new();

    let mut cur_axon_id = base_axon_id;

    for input in &inputs {
        let name_bytes = input.name.as_bytes();
        let name_len = name_bytes.len() as u16;

        let desc_offset = flat_axon_ids.len() as u32;

        // Дескриптор в памяти
        let mut desc = Vec::new();
        desc.extend_from_slice(&name_len.to_le_bytes());
        desc.extend_from_slice(name_bytes);
        desc.extend_from_slice(&input.width.to_le_bytes());
        desc.extend_from_slice(&input.height.to_le_bytes());
        desc.extend_from_slice(&(desc_offset).to_le_bytes()); // Будет обновлено позже при записи смещения в Body
        
        map_descriptors.push(desc);

        let input_seed_base = master_seed ^ fnv1a(&input.name);

        let region_w = (world_w_vox as f32 / input.width as f32).max(1.0);
        let region_d = (world_d_vox as f32 / input.height as f32).max(1.0);

        for y in 0..input.height {
            for x in 0..input.width {
                let pixel_idx = y * input.width + x;
                let seed = input_seed_base ^ (pixel_idx as u64);

                let rx = (x as f32 * region_w) as u32;
                let ry = (y as f32 * region_d) as u32;
                let rx_end = ((x + 1) as f32 * region_w) as u32;
                let ry_end = ((y + 1) as f32 * region_d) as u32;

                // Найдём кандидатов
                let mut candidates = Vec::new();
                for (idx, neuron) in placed_neurons.iter().enumerate() {
                    let (nx, ny, _nz, ntype) = unpack_position(neuron.position);
                    if nx >= rx && nx < rx_end && ny >= ry && ny < ry_end {
                        let is_match = input.target_type == "ALL" || 
                            blueprints.neuron_types.get(ntype as usize)
                            .map_or(false, |nt| nt.name == input.target_type);
                        
                        if is_match {
                            candidates.push((idx, neuron, ntype as usize));
                        }
                    }
                }

                // Fallback: Если в регионе пусто, расширяем поиск на всё
                if candidates.is_empty() {
                    for (idx, neuron) in placed_neurons.iter().enumerate() {
                        let (_, _, _, ntype) = unpack_position(neuron.position);
                        let is_match = input.target_type == "ALL" || 
                            blueprints.neuron_types.get(ntype as usize)
                            .map_or(false, |nt| nt.name == input.target_type);
                        if is_match {
                            candidates.push((idx, neuron, ntype as usize));
                        }
                    }
                }

                // Если всё ещё пусто - берём кого угодно (для отказоустойчивости)
                if candidates.is_empty() {
                    for (idx, neuron) in placed_neurons.iter().enumerate() {
                        let (_, _, _, ntype) = unpack_position(neuron.position);
                        candidates.push((idx, neuron, ntype as usize));
                    }
                }

                // Выбираем сому
                let choice = (seed as usize) % candidates.len();
                let (_, chosen_neuron, actual_type) = candidates[choice];
                let (sx, sy, sz, _) = unpack_position(chosen_neuron.position);

                // Если "ALL", используем параметры фактического типа нейрона.
                // Иначе ищем тип по имени input.target_type, а если не нашли - actual_type.
                let nt = if input.target_type == "ALL" {
                    &blueprints.neuron_types[actual_type]
                } else {
                    blueprints.neuron_types.iter()
                        .find(|nt| nt.name == input.target_type)
                        .unwrap_or(&blueprints.neuron_types[actual_type])
                };

                let type_idx = blueprints.neuron_types.iter().position(|x| x.name == nt.name).unwrap_or(0);

                // Выращиваем виртуальный аксон!
                let (axon, ghost) = grow_single_axon(
                    sx, sy, sz,
                    usize::MAX, // soma_idx = usize::MAX (exterior marker)
                    type_idx,
                    nt,
                    sim,
                    world_w_vox,
                    world_d_vox,
                    world_h_vox,
                    layer_ranges,
                    &spatial_grid,
                    placed_neurons,
                    shard_bounds,
                    seed,
                );

                virtual_axons.push(axon);
                if let Some(gp) = ghost {
                    ghost_packets.push(gp);
                }

                flat_axon_ids.push(cur_axon_id);
                cur_axon_id += 1;
            }
        }
    }

    // Собираем дескрипторы, пересчитывая axon_offset корректно
    let mut all_descriptors = Vec::new();
    let mut current_offset: u32 = 0;
    for (i, mut desc) in map_descriptors.into_iter().enumerate() {
        // Перезаписываем axon_offset (последние 4 байта дескриптора)
        let offset_bytes = current_offset.to_le_bytes();
        let len = desc.len();
        desc[len - 4..].copy_from_slice(&offset_bytes);
        all_descriptors.append(&mut desc);

        // Инкрементируем offset на количество пикселей этой карты
        let map = &inputs[i];
        current_offset += map.width * map.height;
    }

    // Собираем GXI
    gxi_bytes.extend(all_descriptors);
    for id in flat_axon_ids {
        gxi_bytes.extend_from_slice(&id.to_le_bytes());
    }

    VirtualAxonsResult {
        axons: virtual_axons,
        ghosts: ghost_packets,
        gxi_binary: gxi_bytes,
    }
}
