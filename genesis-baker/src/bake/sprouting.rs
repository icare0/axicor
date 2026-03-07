use genesis_core::config::blueprints::NeuronType;



/// Вычисляет привлекательность сомы-кандидата для растущего аксона.
/// Вся математика здесь легально использует f32, так как это Night Phase.
#[inline]
pub fn compute_sprouting_score(
    target_type: &NeuronType,
    distance: f32,
    power_index: f32,
    noise: f32,
) -> f32 {
    let dist_score = 1.0 / (distance + 1.0);
    
    dist_score * target_type.sprouting_weight_distance 
        + power_index * target_type.sprouting_weight_power 
        + noise * target_type.sprouting_weight_explore
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

#[inline]
fn nudge_axon(axon_id: usize, tips: &mut [u32], dirs: &[u32]) {
    let packed_tip = tips[axon_id];
    if packed_tip == 0 { return; }

    let mut tx = packed_tip & 0x3FF;
    let mut ty = (packed_tip >> 10) & 0x3FF;
    let mut tz = (packed_tip >> 20) & 0xFF;
    
    let packed_dir = dirs[axon_id];
    let dx = (packed_dir & 0xFF) as i8;
    let dy = ((packed_dir >> 8) & 0xFF) as i8;
    let dz = ((packed_dir >> 16) & 0xFF) as i8;

    // Shift by 1 voxel along the majority axis
    let shift_x = if dx > 64 { 1 } else if dx < -64 { -1 } else { 0 };
    let shift_y = if dy > 64 { 1 } else if dy < -64 { -1 } else { 0 };
    let shift_z = if dz > 64 { 1 } else if dz < -64 { -1 } else { 0 };

    tx = (tx as i32 + shift_x).clamp(0, 1023) as u32;
    ty = (ty as i32 + shift_y).clamp(0, 1023) as u32;
    tz = (tz as i32 + shift_z).clamp(0, 255) as u32;

    tips[axon_id] = (packed_tip & 0xF0000000) | (tz << 20) | (ty << 10) | tx;
}

pub fn run_sprouting_pass(
    targets: &mut [u32],
    weights: &mut [i16],
    flags: &[u8],
    axon_tips_uvw: &mut [u32],
    axon_dirs_xyz: &[u32],
    soma_to_axon: &[u32],
    padded_n: usize,
    blueprints: Option<&BlueprintsConfig>,
    epoch: u64,
) -> usize {
    let total_axons = axon_tips_uvw.len();

    // 1. Living Axons (Activity-Based Nudging)
    // Local Axons: Nudge only if the soma spiked during the Day Phase
    for soma_idx in 0..padded_n {
        if (flags[soma_idx] & 0x01) != 0 {
            let axon_id = soma_to_axon[soma_idx];
            if axon_id != u32::MAX && (axon_id as usize) < total_axons {
                nudge_axon(axon_id as usize, axon_tips_uvw, axon_dirs_xyz);
            }
        }
    }
    // Ghost/Virtual Axons: Inertial Nudging (no local soma, always move)
    for axon_id in padded_n..total_axons {
        nudge_axon(axon_id, axon_tips_uvw, axon_dirs_xyz);
    }

    // 2. Synaptogenesis
    let mut new_synapses = 0;
    let occupied: Vec<u32> = targets.iter()
        .filter(|&&t| t != 0)
        .copied()
        .collect();

    if occupied.is_empty() {
        return 0; 
    }

    for i in 0..padded_n {
        for slot in (0..MAX_DENDRITE_SLOTS).rev() {
            let col_idx = slot * padded_n + i;
            if targets[col_idx] != 0 {
                break; 
            }

            let salt = (i as u32).wrapping_add(slot as u32).wrapping_add(1);
            let hash = fnv1a(epoch, salt);
            let candidate_idx = (hash % occupied.len() as u64) as usize;
            let candidate_packed = occupied[candidate_idx];

            let candidate_axon_id = (candidate_packed & 0x00FF_FFFF).saturating_sub(1);
            let new_target = pack_dendrite_target(candidate_axon_id, 0);

            // [DOD FIX] Strict Dale's Law: Extract type directly from the axon's geometry mask
            let type_id = if (candidate_axon_id as usize) < total_axons {
                (axon_tips_uvw[candidate_axon_id as usize] >> 28) as usize
            } else {
                0
            };

            let (is_inhibitory_src, initial_weight) = if let Some(bp) = blueprints {
                if let Some(nt) = bp.neuron_types.get(type_id) {
                    (nt.is_inhibitory, nt.initial_synapse_weight as i16)
                } else { (false, 74) }
            } else { (false, 74) };

            let final_weight = if is_inhibitory_src { -initial_weight } else { initial_weight };

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

