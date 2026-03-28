use bevy::prelude::*;

pub fn build_topology_graph(
    state_bytes: &[u8],
    pos_data: &[u8],
    center: Vec3,
    axon_segments_lookup: &[Vec<Vec3>],
    graph: &mut crate::domain::TopologyGraph,
) {
    let padded_n = state_bytes.len() / 1166; // 1166-Byte Invariant
    let soma_to_axon_offset = padded_n * 10;
    let targets_offset = padded_n * 14;

    graph.padded_n = padded_n;

    // Кэшируем маппинг аксонов (C-ABI: voltage(4) + flags(1) + thresh(4) + timers(1) = 10 bytes offset)
    graph.soma_to_axon = state_bytes[soma_to_axon_offset .. targets_offset]
        .chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();

    // ДОБАВЛЕНО: Строим обратный маппинг (Axon ID -> Soma Dense ID)
    let total_axons = axon_segments_lookup.len();
    let mut axon_to_soma = vec![usize::MAX; total_axons];
    for (soma_id, &axon_id) in graph.soma_to_axon.iter().enumerate() {
        if axon_id != 0xFFFFFFFF && (axon_id as usize) < total_axons {
            axon_to_soma[axon_id as usize] = soma_id;
        }
    }
    graph.axon_to_soma = axon_to_soma;

    // Кэшируем колонки дендритов (DOD: Columnar Memory Layout)
    let dendrites_bytes = padded_n * 128 * 4;
    graph.targets = state_bytes[targets_offset .. targets_offset + dendrites_bytes]
        .chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();

    graph.axon_segments = axon_segments_lookup.to_vec();

    // Кэшируем позиции сом для мгновенной трассировки
    let packed_positions: Vec<u32> = pos_data.chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();

    let mut somas = vec![Vec3::ZERO; padded_n];
    let mut compact_to_dense = Vec::new(); // ДОБАВЛЕНО

    for i in 0..padded_n {
        if i >= packed_positions.len() { break; }
        let packed = packed_positions[i];
        if packed != 0 {
            compact_to_dense.push(i); // ДОБАВЛЕНО: Сохраняем реальный dense_id
            let x = (packed & 0x3FF) as f32 * 0.025;
            let y = ((packed >> 10) & 0x3FF) as f32 * 0.025;
            let z = ((packed >> 20) & 0xFF) as f32 * 0.025;
            somas[i] = Vec3::new(x, z, -y) - center;
        }
    }
    graph.soma_positions = somas;
    graph.compact_to_dense = compact_to_dense; // ДОБАВЛЕНО
}
