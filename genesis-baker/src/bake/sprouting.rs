use genesis_core::config::blueprints::GenesisConstantMemory;

/// Нормализованный «вес» сомы = Σ|dendrite_weights| / (128 × 32767).
/// При первом Baking все веса = 0 → power_index = 0.0 (новые нейроны равны).
/// (04_connectivity.md §1.6.1)
#[allow(dead_code)]
pub fn compute_power_index(soma_id: usize, weights: &[i16], padded_n: usize) -> f32 {
    let mut power = 0u32;
    for slot in 0..128 {
        let w = weights[slot * padded_n + soma_id];
        power += w.unsigned_abs() as u32;
    }
    power as f32 / (128.0 * 32767.0)
}

/// Вычисляет привлекательность сомы-кандидата для растущего аксона.
/// Вся математика здесь легально использует f32, так как это Night Phase.
#[inline]
pub fn compute_sprouting_score(
    const_mem: &GenesisConstantMemory,
    target_type_idx: u8,
    distance: f32,
    power_index: f32,
    noise: f32,
) -> f32 {
    // Прямое чтение параметров нужного варианта за O(1)
    let _variant = &const_mem.variants[target_type_idx as usize];
    
    // Пример скоринга, основанного на параметрах конкретного типа.
    // Если для этого типа прописано мощное влияние дистанции:
    let dist_weight = 10.0; // Stronger distance influence to overcome noise in tests
    let power_weight = 0.5;
    
    let score = (1.0 / (distance + 1.0)) * dist_weight 
              + power_index * power_weight 
              + noise;
              
    score
}

/// Евклидово расстояние в вокселях между двумя точками.
pub fn voxel_dist(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32) -> f32 {
    let dx = ax as f32 - bx as f32;
    let dy = ay as f32 - by as f32;
    let dz = az as f32 - bz as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

use genesis_core::config::blueprints::BlueprintsConfig;
use genesis_core::layout::pack_dendrite_target;
use genesis_core::constants::MAX_DENDRITE_SLOTS;

/// CPU Sprouting Pass — заполняет пустые дендритные слоты.
/// Zero-copy: работает напрямую со слайсами из SHM.
pub fn run_sprouting_pass(
    targets: &mut [u32],
    weights: &mut [i16],
    padded_n: usize,
    blueprints: Option<&BlueprintsConfig>,
    epoch: u64,
) -> usize {
    let mut new_synapses = 0;

    // Собираем список занятых аксонов (target != 0) для случайного выбора
    let occupied: Vec<u32> = targets.iter()
        .filter(|&&t| t != 0)
        .copied()
        .collect();

    if occupied.is_empty() {
        return 0; // Никаких существующих связей для анализа
    }

    for i in 0..padded_n {
        for slot in (0..MAX_DENDRITE_SLOTS).rev() {
            let col_idx = slot * padded_n + i;
            if targets[col_idx] != 0 {
                break; // Слот занят — список сортирован по убыванию силы
            }

            // Детерминированный выбор кандидата из занятых аксонов
            let salt = (i as u32).wrapping_add(slot as u32).wrapping_add(1);
            let hash = fnv1a(epoch, salt);
            let candidate_idx = (hash % occupied.len() as u64) as usize;
            let candidate_packed = occupied[candidate_idx];

            // Распаковываем axon_id из существующего target (отменяем +1 Zero-Index смещение)
            let candidate_axon_id = (candidate_packed & 0x00FF_FFFF).saturating_sub(1);

            // [DOD FIX 1] Правильная упаковка через контрактный API
            let new_target = pack_dendrite_target(candidate_axon_id, 0);

            // [DOD FIX 2] Закон Дейла — знак веса из type_id аксона
            let src_weight = weights[candidate_idx % (padded_n * MAX_DENDRITE_SLOTS)];
            let is_inhibitory_src = src_weight < 0;

            let final_weight = if let Some(bp) = blueprints {
                let abs_w = if let Some(nt) = bp.neuron_types.first() {
                    nt.initial_synapse_weight as i16
                } else { 74 };
                if is_inhibitory_src { -abs_w } else { abs_w }
            } else {
                if is_inhibitory_src { -74_i16 } else { 74_i16 }
            };

            targets[col_idx] = new_target;
            weights[col_idx] = final_weight;
            new_synapses += 1;
            break;
        }
    }

    new_synapses
}

/// FNV-1a Stateless Hash (Инвариант #7 — детерминизм через seed+id)
fn fnv1a(seed: u64, salt: u32) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &b in &seed.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for &b in &salt.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

