/// Spatial coordinates, converters, and packing (Spec 01 §1.1–1.3).
///
/// Three coordinate systems (§1.1):
///   Microns    — absolute (1.0 = 1 µm), for physics and geometry
///   Fraction   — normalized [0.0, 1.0], for layer boundaries
///   VoxelCoord — discrete, for GPU and spatial hashing
///
/// PackedPosition layout: [Type(4b) | Z(8b) | Y(10b) | X(10b)]
/// Bit layout: type << 28 | z << 20 | y << 10 | x
///
/// Ranges:
///   X: 0..=1023  (10 bits)
///   Y: 0..=1023  (10 bits)
///   Z: 0..=255   (8 bits)
///   type_mask: 0..=15 (4 bits)
use crate::types::{Fraction, Microns, PackedPosition, VoxelCoord};

// ---------------------------------------------------------------------------
// §1.1 Converters between coordinate systems
// ---------------------------------------------------------------------------

/// Absolute µm → voxels (spatial discretization).
/// `voxel_size_um` is taken from `constants::VOXEL_SIZE_UM` or from config.
#[inline]
pub fn um_to_voxel(um: Microns, voxel_size_um: u32) -> VoxelCoord {
    (um / voxel_size_um as Microns) as VoxelCoord
}

/// Normalized fraction [0.0, 1.0] → voxels.
/// Used for translating layer `height_pct` / `population_pct` into voxel boundaries.
/// `world_dim_vox` — dimension size in voxels (e.g., world_h_vox).
#[inline]
pub fn pct_to_voxel(pct: Fraction, world_dim_vox: u32) -> VoxelCoord {
    (pct * world_dim_vox as Fraction) as VoxelCoord
}

/// Voxels → absolute µm.
#[inline]
pub fn voxel_to_um(vox: VoxelCoord, voxel_size_um: u32) -> Microns {
    vox as Microns * voxel_size_um as Microns
}

#[inline]
pub fn pack_position(x: u32, y: u32, z: u32, type_mask: u32) -> PackedPosition {
    PackedPosition::new(x, y, z, type_mask as u8)
}

#[inline]
pub fn unpack_position(p: PackedPosition) -> (u32, u32, u32, u32) {
    (p.x() as u32, p.y() as u32, p.z() as u32, p.type_id() as u32)
}

// ---------------------------------------------------------------------------
// §1.2 PackedTarget — identifier (Axon_ID, Segment_Index)
// ---------------------------------------------------------------------------

use crate::layout::{pack_dendrite_target, unpack_axon_id, unpack_segment_offset};
use crate::types::PackedTarget;

/// Packs `(axon_id, segment_idx)` into `PackedTarget`.
/// Layout: [31..24] segment_offset (8 bits) | [23..0] axon_id + 1 (24 bits).
#[inline]
pub fn pack_target(axon_id: u32, segment_idx: u32) -> PackedTarget {
    pack_dendrite_target(axon_id, segment_idx)
}

/// Unpacks `PackedTarget` into `(axon_id, segment_idx)`.
/// Returns `None` if `t == 0` (empty dendrite slot).
#[inline]
pub fn unpack_target(t: PackedTarget) -> Option<(u32, u32)> {
    if t == 0 { return None; }
    Some((unpack_axon_id(t), unpack_segment_offset(t)))
}
