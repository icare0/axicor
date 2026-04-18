/// Signal Physics: "Active Tail" (Spec 01 1.3).
///
/// A signal is a "train" sliding over axon segments.
/// `axon_head` starts at `AXON_SENTINEL - length * V_SEG` and increments by `V_SEG` each tick.
/// When `axon_head.wrapping_sub(segment_idx) < propagation_length`  the segment "lights up".
///
/// Integer arithmetic with u32 overflow guarantees determinism on GPU without floats.
use crate::constants::{AXON_SENTINEL, V_SEG};
use crate::types::AxonHead;

/// Branchless Active Tail check for GPU Hot Loop (Spec 03 1.3).
///
/// Checks if a dendritic segment falls within the signal's active tail.
/// No branching  AXON_SENTINEL (0x80000000) is handled automatically:
/// `0x80000000.wrapping_sub(any_small_idx)`  2.1B > any propagation_length.
///
/// # Guarantees
/// - Zero Warp Divergence on GPU (no `if`)
/// - Deterministic: identical result on CPU and GPU
/// - AXON_SENTINEL always returns `false`
#[inline(always)]
pub const fn is_in_active_tail(head_idx: u32, segment_idx: u32, propagation_length: u8) -> bool {
    let dist = head_idx.wrapping_sub(segment_idx);
    dist < (propagation_length as u32)
}

/// Checks if segment `segment_idx` is in the "active tail" for the current tick.
///
/// # Arguments
/// - `axon_head`  current axon head position (u32, wrapping)
/// - `segment_idx`  index of the segment being checked
/// - `propagation_length`  tail length in segments (`signal_propagation_length` from blueprints)
///
/// # Returns
/// `true` if the segment is within the active tail `[head - propagation_length, head]`.
#[inline]
pub fn is_segment_active(axon_head: AxonHead, segment_idx: u32, propagation_length: u32) -> bool {
    if axon_head == AXON_SENTINEL {
        return false;
    }
    axon_head.wrapping_sub(segment_idx) < propagation_length
}

/// Calculates the initial position of the axon head for N segments.
/// `head = AXON_SENTINEL - length * V_SEG`
///
/// This allows `propagate_axons` to correctly "reach" the end on the very first tick.
#[inline]
pub fn initial_axon_head(length_segments: u32) -> AxonHead {
    AXON_SENTINEL.wrapping_sub(length_segments * V_SEG)
}

#[cfg(test)]
#[path = "test_signal.rs"]
mod test_signal;

#[cfg(test)]
#[path = "test_train_model.rs"]
mod test_train_model;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AXON_SENTINEL;

    #[test]
    fn test_active_tail_normal_overlap() {
        // head=10, segment=8, prop=3 -> dist=2 < 3 -> true
        assert!(is_in_active_tail(10, 8, 3));
        // head=10, segment=10, prop=3 -> dist=0 < 3 -> true
        assert!(is_in_active_tail(10, 10, 3));
        // head=10, segment=9, prop=3 -> dist=1 < 3 -> true
        assert!(is_in_active_tail(10, 9, 3));
    }

    #[test]
    fn test_active_tail_outside() {
        // head=10, segment=7, prop=3 -> dist=3, NOT < 3 -> false
        assert!(!is_in_active_tail(10, 7, 3));
        // head=10, segment=0, prop=3 -> dist=10 -> false
        assert!(!is_in_active_tail(10, 0, 3));
    }

    #[test]
    fn test_sentinel_edge_case() {
        // AXON_SENTINEL (0x80000000) - 5 = 0x7FFFFFFB  2.1 billion -> always >= prop
        assert!(!is_in_active_tail(AXON_SENTINEL, 5, 3));
        assert!(!is_in_active_tail(AXON_SENTINEL, 0, 255));
        assert!(!is_in_active_tail(AXON_SENTINEL, 1000, 100));
    }

    #[test]
    fn test_active_tail_zero_propagation() {
        // prop=0 means no segment is ever active
        assert!(!is_in_active_tail(10, 10, 0));
        assert!(!is_in_active_tail(10, 9, 0));
    }
}
