use serde::{Deserialize, Serialize};

/// Packet sent when an axon wants to cross a shard boundary (Night Phase).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewAxon {
    /// Local absolute ID of the axon on the sending shard.
    pub source_axon_id: u32,
    /// X/Y/Z entry point on the boundary plane of the receiver. We use u32 packed.
    pub entry_point: (u16, u16),
    /// Normalized directional inertia vector.
    pub vector: (i8, i8, i8),
    /// Structural type of the sending neuron (Geo | Sign | Variant).
    pub type_mask: u8,
    /// How many segments this axon can still grow.
    pub remaining_length: u16,
}

/// Acknowledgment of a successful boundary crossing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckNewAxon {
    /// The original ID on the sender side so it knows which axon this is.
    pub source_axon_id: u32,
    /// The newly allocated Ghost ID on the receiver side.
    pub ghost_id: u32,
}

/// Packet sent when a long-range axon dies (pruned due to weight < threshold).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PruneAxon {
    /// The target's local ghost ID to be freed.
    pub ghost_id: u32,
}

/// The overarching enum for all Geometry events (Slow Path).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeometryRequest {
    Handover(NewAxon),
    Prune(PruneAxon),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeometryResponse {
    Ack(AckNewAxon),
    Ok,
    Error(String),
}
