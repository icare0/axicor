/// Sentinel value for inactive axons.
/// dist = AXON_SENTINEL - seg_idx = huge → is_active = false (u32 underflow legalized).
pub const AXON_SENTINEL: u32 = 0x80000000;

/// Maximum dendrite slots per neuron (LTM: 0..79, WM: 80..127).
pub const MAX_DENDRITE_SLOTS: usize = 128;

/// LTM / WM boundary slot index.
pub const WM_SLOT_START: usize = 80;

/// target_packed bit layout: [31..10] Axon_ID (22 bits) | [9..0] Segment_Index (10 bits)
pub const TARGET_AXON_SHIFT: u32 = 10;
pub const TARGET_SEG_MASK: u32 = 0x3FF;

/// Warp size for GPU alignment (padded_n must be multiple of this).
pub const GPU_WARP_SIZE: usize = 32;
